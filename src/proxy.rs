use crate::{
	asset_extract::Assets,
	config::Config,
	module::{Module, ModuleInstance, PacketFlow, ProxySide, RootModuleInstance},
	packets::{ClientPacket, ServerPacket},
	read::RPRead,
	write::RPWrite,
};
use rc4::{consts::U13, KeyInit, Rc4, StreamCipher};
use std::{
	io::{ErrorKind, Write},
	sync::Arc,
	time::Instant,
};
use tokio::{
	io::{self, AsyncReadExt, AsyncWriteExt, BufReader},
	net::{
		tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf},
		TcpStream,
	},
	select,
};
use tracing::{error, instrument};

// RC4 keys (server to client and client to server)
#[rustfmt::skip]
const RC4_K_S_TO_C: &[u8; 13] = &[0xC9, 0x1D, 0x9E, 0xEC, 0x42, 0x01, 0x60, 0x73, 0x0D, 0x82, 0x56, 0x04, 0xE0];
#[rustfmt::skip]
const RC4_K_C_TO_S: &[u8; 13] = &[0x5A, 0x4D, 0x20, 0x16, 0xBC, 0x16, 0xDC, 0x64, 0x88, 0x31, 0x94, 0xFF, 0xD9];

// Default buffer size for reading and writing packets
const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

pub struct Proxy<'a> {
	pub config: Arc<Config>,
	pub assets: Arc<Assets>,
	pub modules: RootModuleInstance,
	pub write: ProxyWriteHalf<'a>,
	// if true will not read incoming data from the server
	// because client is behind and we need to wait for it to catch up
	pub pause_server_read: bool,
}

// The write half of the proxy, both for server and client
pub struct ProxyWriteHalf<'a> {
	client: WriteHalf<'a>,
	server: WriteHalf<'a>,
	client_rc4: Rc4<U13>,
	server_rc4: Rc4<U13>,
	// buffer used for writing packets
	write_buf: Vec<u8>,
}

// Basically a specialised better BufReader
struct PacketReader<'a> {
	stream: ReadHalf<'a>,
	rc4: Rc4<U13>,
	buf: Vec<u8>,
	// how many bytes are in the buffer right now
	cursor: usize,
}

impl<'a> Proxy<'a> {
	#[instrument(skip_all, fields(ip = ?server.peer_addr()?))]
	pub async fn run(
		config: Arc<Config>,
		assets: Arc<Assets>,
		modules: RootModuleInstance,
		mut client: TcpStream,
		mut server: TcpStream,
	) -> io::Result<()> {
		let (client_read, client_write) = client.split();
		let (server_read, server_write) = server.split();

		let mut proxy = Proxy {
			config,
			assets,
			modules,
			write: ProxyWriteHalf {
				client: client_write,
				server: server_write,
				client_rc4: Rc4::new(RC4_K_S_TO_C.into()),
				server_rc4: Rc4::new(RC4_K_C_TO_S.into()),
				write_buf: vec![0u8; DEFAULT_BUFFER_SIZE],
			},
			pause_server_read: false,
		};

		let mut client_reader = PacketReader::new(client_read, RC4_K_C_TO_S);
		let mut server_reader = PacketReader::new(server_read, RC4_K_S_TO_C);

		loop {
			select! {
				biased;
				r = server_reader.wait_for_whole_packet(), if !proxy.pause_server_read => {
					if let Err(e) = r {
						RootModuleInstance::disconnect(&mut proxy, ProxySide::Server).await?;

						return Err(e);
					}

					let raw_packet = server_reader.get_raw_packet();

					match ServerPacket::rp_read(&mut &raw_packet[..]) {
						Ok(mut p) => {
							match RootModuleInstance::server_packet(&mut proxy, &mut p).await {
								Ok(PacketFlow::Forward) => {
									// 👍
									// forward the packet
									proxy.write.send_client(&p).await?;
								}
								Ok(PacketFlow::Block) => {}, // dont forward the packet
								Err(e) => {
									error!("Error handling server packet: {e:?}");
								}
							}
						}
						Err(e) => {
							error!("Error parsing server packet: {e:?}");
						}
					}

					server_reader.pop_packet();
				},
				r = client_reader.wait_for_whole_packet() => {
					if let Err(e) = r {
						RootModuleInstance::disconnect(&mut proxy, ProxySide::Client).await?;

						return Err(e);
					}

					let raw_packet = client_reader.get_raw_packet();

					match ClientPacket::rp_read(&mut &raw_packet[..]) {
						Ok(mut p) => {
							match RootModuleInstance::client_packet(&mut proxy, &mut p).await {
								Ok(PacketFlow::Forward) => {
									// 👍
									// forward the packet
									proxy.write.send_server(&p).await?;
								}
								Ok(PacketFlow::Block) => {}, // dont forward the packet
								Err(e) => {
									error!("Error handling client packet: {e:?}");
								}
							}
						}
						Err(e) => {
							error!("Error parsing client packet: {e:?}");
						}
					}

					client_reader.pop_packet();
				},
			};
		}
	}
}

impl<'a> ProxyWriteHalf<'a> {
	/// Sends a packet TO the server
	pub async fn send_server(&mut self, packet: &ClientPacket<'_>) -> io::Result<()> {
		self.write_buf.clear();
		0u32.rp_write(&mut self.write_buf)?; // placeholder for length

		let packet_length = packet.rp_write(&mut self.write_buf)? as u32 + 4; // +4 because the packet length itself is included
		self.write_buf[0..4].copy_from_slice(&u32::to_be_bytes(packet_length)[..]);

		self.server_rc4.apply_keystream(&mut self.write_buf[5..]);

		self.server.write_all(&self.write_buf).await?;
		self.server.flush().await?;

		Ok(())
	}
	/// Sends a packet TO the client
	pub async fn send_client(&mut self, packet: &ServerPacket<'_>) -> io::Result<()> {
		self.write_buf.clear();
		0u32.rp_write(&mut self.write_buf)?; // placeholder for length

		let packet_length = packet.rp_write(&mut self.write_buf)? as u32 + 4; // +4 because the packet length itself is included
		self.write_buf[0..4].copy_from_slice(&u32::to_be_bytes(packet_length)[..]);

		self.client_rc4.apply_keystream(&mut self.write_buf[5..]);

		self.client.write_all(&self.write_buf).await?;
		self.client.flush().await?;

		Ok(())
	}
}

impl<'a> PacketReader<'a> {
	fn new(stream: ReadHalf<'a>, rc4_key: &[u8; 13]) -> Self {
		PacketReader {
			stream,
			rc4: Rc4::new(rc4_key.into()),
			buf: vec![0u8; DEFAULT_BUFFER_SIZE],
			cursor: 0,
		}
	}
	// Will complete when a whole packet is ready to be read
	async fn wait_for_whole_packet(&mut self) -> io::Result<()> {
		// first we gotta wait for the packet length
		let packet_length = loop {
			if self.cursor >= 4 {
				break read_packet_size(&self.buf[0..4]);
			}

			// read more into buffer
			self.cursor += self.stream.read(&mut self.buf[self.cursor..]).await?;
		};

		if packet_length < 5 {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				format!("Packet size cannot be less than 5: {packet_length}"),
			));
		}

		if packet_length > self.buf.len() {
			self.buf.reserve(packet_length - self.buf.len());
		}

		// now we gotta wait for the whole length to arrive
		loop {
			if self.cursor >= packet_length {
				break;
			}

			// read more...
			self.cursor += self.stream.read(&mut self.buf[self.cursor..]).await?;
		}

		// and finally decipher the packet
		self.rc4.apply_keystream(&mut self.buf[5..packet_length]);

		Ok(())
	}
	// Will get a packet from the buffer (will panic if there isnt a full packet in the buffer, call wait_for_whole_packet first!)
	// Also this wont remove the packet from the buffer, so successive calls will return the same packet!
	// Call pop_packet next!
	fn get_raw_packet<'b>(&'b mut self) -> &'b [u8] {
		let packet_size = read_packet_size(&self.buf[0..4]);

		&self.buf[4..packet_size]
	}
	// Will remove the first packet from the buffer
	fn pop_packet(&mut self) {
		let packet_size = read_packet_size(&self.buf[0..4]);

		self.buf.rotate_left(packet_size);

		self.cursor -= packet_size;
	}
}

fn read_packet_size(buf: &[u8]) -> usize {
	assert_eq!(buf.len(), 4, "packet size must be 4 bytes");

	u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize
}

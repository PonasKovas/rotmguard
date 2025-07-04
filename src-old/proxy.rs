use crate::{
	assets::Assets,
	config::Config,
	module::{ModuleInstance, PacketFlow, ProxySide, RootModuleInstance},
	packets::{ClientPacket, ServerPacket},
	read::RPRead,
	write::RPWrite,
};
use anyhow::{bail, Context, Result};
use rc4::{consts::U13, KeyInit, Rc4, StreamCipher};
use std::{
	mem::swap,
	ops::{Deref, DerefMut},
	sync::Arc,
};
use tokio::{
	io::{self, AsyncReadExt, AsyncWriteExt, BufWriter},
	net::{
		tcp::{OwnedReadHalf, ReadHalf},
		TcpStream,
	},
	select,
	sync::mpsc::{channel, Sender},
	time::{interval, sleep_until, timeout, Duration, Instant},
};
use tracing::instrument;

// RC4 keys (server to client and client to server)
#[rustfmt::skip]
const RC4_K_S_TO_C: &[u8; 13] = &[0xC9, 0x1D, 0x9E, 0xEC, 0x42, 0x01, 0x60, 0x73, 0x0D, 0x82, 0x56, 0x04, 0xE0];
#[rustfmt::skip]
const RC4_K_C_TO_S: &[u8; 13] = &[0x5A, 0x4D, 0x20, 0x16, 0xBC, 0x16, 0xDC, 0x64, 0x88, 0x31, 0x94, 0xFF, 0xD9];

// Default buffer size for reading and writing packets
const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

// Now this wasnt tested statistically but it doesn't matter that much probably
// basically when reading packet length we also make sure that theres AT LEAST this much
// space for the whole packet to read in the same syscall, if not more.
// For most packets it should be plenty i think.
const AVG_PACKET_LENGTH: usize = 512;

// Period of writing buffered packets, in milliseconds.
// So all buffered packets will be sent to client/server ever X ms.
const WRITE_PERIOD: u64 = 10;

pub struct Proxy {
	pub config: Arc<Config>,
	pub assets: Arc<Assets>,
	pub modules: RootModuleInstance,
	pub client: Sender<ServerPacket>,
	pub server: Sender<ClientPacket>,
}

// Basically a specialised better BufWriter
pub struct PacketWriter {
	rc4: Rc4<U13>,
	buf: Vec<u8>,
}

// Basically a specialised better BufReader
struct PacketReader {
	stream: OwnedReadHalf,
	rc4: Rc4<U13>,
	buf: Vec<u8>,
	// start and end of written data in the buffer
	buf_start: usize,
	buf_end: usize,
}

// A reference to a raw packet that will remove it from the buffer on Drop
struct RawPacketRef<'a> {
	reader: &'a mut PacketReader,
}

impl Proxy {
	#[instrument(skip_all, fields(ip = ?server.peer_addr()?))]
	pub async fn run(
		config: Arc<Config>,
		assets: Arc<Assets>,
		modules: RootModuleInstance,
		client: TcpStream,
		server: TcpStream,
	) -> Result<()> {
		let (client_read, mut client_write) = client.into_split();
		let (server_read, mut server_write) = server.into_split();

		let (client_sender, mut client_recv) = channel::<ServerPacket>(100);
		let (server_sender, mut server_recv) = channel::<ClientPacket>(100);
		tokio::spawn(async move {
			let mut writer = PacketWriter {
				rc4: Rc4::new(RC4_K_S_TO_C.into()),
				buf: Vec::with_capacity(DEFAULT_BUFFER_SIZE),
			};

			let mut last_flush = Instant::now();
			loop {
				select! {
					_ = sleep_until(last_flush + Duration::from_millis(WRITE_PERIOD)) => {
						client_write.write_all(&writer.buf).await.unwrap();
						client_write.flush().await.unwrap();
						writer.buf.clear();
						last_flush = Instant::now();
					}
					pkt = client_recv.recv() => {
						match pkt {
							Some(pkt) => {
								writer.add_server_packet(&pkt);

								if writer.buf.len() > 8 * 1024 {
									client_write.write_all(&writer.buf).await.unwrap();
									client_write.flush().await.unwrap();
									writer.buf.clear();
									last_flush = Instant::now();
								}
							},
							None => return,
						}
					}
				}
			}
		});
		tokio::spawn(async move {
			let mut writer = PacketWriter {
				rc4: Rc4::new(RC4_K_C_TO_S.into()),
				buf: Vec::with_capacity(DEFAULT_BUFFER_SIZE),
			};

			let mut last_flush = Instant::now();
			loop {
				select! {
					_ = sleep_until(last_flush + Duration::from_millis(WRITE_PERIOD)) => {
						server_write.write_all(&writer.buf).await.unwrap();
						server_write.flush().await.unwrap();
						writer.buf.clear();
						last_flush = Instant::now();
					}
					pkt = server_recv.recv() => {
						match pkt {
							Some(pkt) => {
								writer.add_client_packet(&pkt);

								if writer.buf.len() > 8 * 1024 {
									server_write.write_all(&writer.buf).await.unwrap();
									server_write.flush().await.unwrap();
									writer.buf.clear();
									last_flush = Instant::now();
								}
							},
							None => return,
						}
					}
				}
			}
		});

		let mut proxy = Proxy {
			config,
			assets,
			modules,
			client: client_sender,
			server: server_sender,
		};

		let mut client_reader = PacketReader::new(client_read, RC4_K_C_TO_S);
		let mut server_reader = PacketReader::new(server_read, RC4_K_S_TO_C);

		loop {
			select! {
				p = server_reader.wait_for_whole_packet() => {
					let raw_packet = match p {
						Ok(p) => p,
						Err(e) => {
							RootModuleInstance::disconnect(&mut proxy, ProxySide::Server).await.context("modules disconnect()")?;

							bail!("Error reading server packet: {e:?}");
						}
					};

					match ServerPacket::rp_read(&mut &raw_packet[..]) {
						Ok(mut p) => {
							match RootModuleInstance::server_packet(&mut proxy, &mut p).await {
								Ok(PacketFlow::Forward) => {
									// 👍
									// forward the packet
									proxy.client.send(p).await.unwrap();
								}
								Ok(PacketFlow::Block) => {}, // dont forward the packet
								Err(e) => {
									bail!("Error handling server packet: {e:?}");
								}
							}
						}
						Err(e) => {
							bail!("Error parsing server packet: {e:?}");
						}
					}
				},
				p = client_reader.wait_for_whole_packet() => {
					let raw_packet = match p {
						Ok(p) => p,
						Err(e) => {
							RootModuleInstance::disconnect(&mut proxy, ProxySide::Client).await?;

							bail!("Error reading client packet: {e:?}");
						}
					};

					match ClientPacket::rp_read(&mut &raw_packet[..]) {
						Ok(mut p) => {
							match RootModuleInstance::client_packet(&mut proxy, &mut p).await {
								Ok(PacketFlow::Forward) => {
									// 👍
									// forward the packet
									proxy.server.send(p).await.unwrap();
								}
								Ok(PacketFlow::Block) => {}, // dont forward the packet
								Err(e) => {
									bail!("Error handling client packet: {e:?}");
								}
							}
						}
						Err(e) => {
							bail!("Error parsing client packet: {e:?}");
						}
					}
				},
			};
		}
	}
}

impl PacketWriter {
	pub fn add_client_packet(&mut self, packet: &ClientPacket) {
		let len_pos = self.buf.len();
		0u32.rp_write(&mut self.buf); // placeholder for length

		let packet_length = packet.rp_write(&mut self.buf) as u32 + 4; // +4 because the packet length itself is included
		self.buf[len_pos..(len_pos + 4)].copy_from_slice(&u32::to_be_bytes(packet_length)[..]);

		self.rc4.apply_keystream(&mut self.buf[(len_pos + 5)..]);
	}
	pub fn add_server_packet(&mut self, packet: &ServerPacket) {
		let len_pos = self.buf.len();
		0u32.rp_write(&mut self.buf); // placeholder for length

		let packet_length = packet.rp_write(&mut self.buf) as u32 + 4; // +4 because the packet length itself is included
		self.buf[len_pos..(len_pos + 4)].copy_from_slice(&u32::to_be_bytes(packet_length)[..]);

		self.rc4.apply_keystream(&mut self.buf[(len_pos + 5)..]);
	}
}

impl PacketReader {
	fn new(stream: OwnedReadHalf, rc4_key: &[u8; 13]) -> Self {
		PacketReader {
			stream,
			rc4: Rc4::new(rc4_key.into()),
			buf: {
				let mut vec = vec![0u8; DEFAULT_BUFFER_SIZE];
				vec.resize(vec.capacity(), 0); // make sure we use all capacity
				vec
			},
			buf_start: 0,
			buf_end: 0,
		}
	}
	// Resets the buf_start to be 0 and copies all written data to the start
	fn reset_buf(&mut self) {
		self.buf.copy_within(self.buf_start..self.buf_end, 0);
		self.buf_end -= self.buf_start;
		self.buf_start = 0;
	}
	fn get_packet_size(&self) -> usize {
		assert!(
			self.buf_end - self.buf_start >= 4,
			"not enough bytes to read packet size"
		);

		let i = self.buf_start;
		let b = &self.buf;
		u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize
	}
	// Will complete when a whole packet is ready to be read
	async fn wait_for_whole_packet(&mut self) -> io::Result<RawPacketRef> {
		// first we gotta wait for the packet length
		let packet_length = loop {
			// check if we have at least 4 bytes written in the buffer
			if self.buf_end - self.buf_start >= 4 {
				break self.get_packet_size();
			}

			// check if we have enough bytes total until buffer end for the
			// packet length and a full average packet hopefully
			if self.buf.len() - self.buf_start < 4 + AVG_PACKET_LENGTH {
				// not enough space in this buffer for reading the packet length
				// reset the buffer to the start
				self.reset_buf();
			}

			// read more into buffer
			self.buf_end += self.stream.read(&mut self.buf[self.buf_end..]).await?;
		};

		if packet_length < 5 {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				format!("Packet size cannot be less than 5: {packet_length}"),
			));
		}

		// check if we have enough space in the buffer for the whole packet
		if self.buf.len() - self.buf_start < packet_length {
			// check if we could just reset the buffer and avoid a re-allocation
			if self.buf.len() >= packet_length {
				// good, reset it
				self.reset_buf();
			} else {
				// even if we reset it wouldnt be enough space, need to re-allocate
				let mut new_buf = vec![0u8; packet_length];
				// make sure we use all capacity of the new buf
				new_buf.resize(new_buf.capacity(), 0);
				// copy all written data from old buffer to the new one
				new_buf[0..(self.buf_end - self.buf_start)]
					.copy_from_slice(&self.buf[self.buf_start..self.buf_end]);

				swap(&mut new_buf, &mut self.buf);
			}
		}

		// now we gotta wait for the whole length to arrive
		loop {
			if self.buf_end - self.buf_start >= packet_length {
				break;
			}

			// read more...
			self.buf_end += self.stream.read(&mut self.buf[self.buf_end..]).await?;
		}

		// and finally decipher the packet
		self.rc4
			.apply_keystream(&mut self.buf[(self.buf_start + 5)..(self.buf_start + packet_length)]);

		Ok(RawPacketRef { reader: self })
	}
}

impl<'a> Deref for RawPacketRef<'a> {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		let packet_size = self.reader.get_packet_size();

		&self.reader.buf[(self.reader.buf_start + 4)..(self.reader.buf_start + packet_size)]
	}
}

impl<'a> DerefMut for RawPacketRef<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		let packet_size = self.reader.get_packet_size();

		&mut self.reader.buf[(self.reader.buf_start + 4)..(self.reader.buf_start + packet_size)]
	}
}

impl<'a> Drop for RawPacketRef<'a> {
	fn drop(&mut self) {
		self.reader.buf_start += self.reader.get_packet_size();
	}
}

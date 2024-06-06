use crate::{
	packets::{ClientPacket, ServerPacket},
	read::RPRead,
	rotmguard::RotmGuard,
	write::RPWrite,
};
use hex::FromHex;
use rc4::{consts::U13, KeyInit, Rc4, StreamCipher};
use tokio::{
	io::{self, AsyncReadExt, AsyncWriteExt, BufReader},
	net::TcpStream,
	select,
};
use tracing::{error, instrument};

const RC4_K_S_TO_C: &'static str = "c91d9eec420160730d825604e0";
const RC4_K_C_TO_S: &'static str = "5a4d2016bc16dc64883194ffd9";

// Default buffer size for reading and writing packets
const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

pub struct Proxy {
	pub rotmguard: RotmGuard,
	pub client: BufReader<TcpStream>,
	pub server: BufReader<TcpStream>,
	rc4: Rc4State,
	// buffer used for writing packets
	write_buf: Vec<u8>,
}

struct Rc4State {
	client_out: Rc4<U13>,
	client_in: Rc4<U13>,
	server_out: Rc4<U13>,
	server_in: Rc4<U13>,
}

impl Proxy {
	pub fn new(client: TcpStream, server: TcpStream) -> Self {
		Self {
			rotmguard: RotmGuard::new(),
			client: BufReader::new(client),
			server: BufReader::new(server),
			rc4: Rc4State {
				client_out: Rc4::new(
					(&Vec::from_hex(RC4_K_S_TO_C).expect("RC4 key invalid hex form")[..]).into(),
				),
				client_in: Rc4::new(
					(&Vec::from_hex(RC4_K_C_TO_S).expect("RC4 key invalid hex form")[..]).into(),
				),
				server_out: Rc4::new(
					(&Vec::from_hex(RC4_K_C_TO_S).expect("RC4 key invalid hex form")[..]).into(),
				),
				server_in: Rc4::new(
					(&Vec::from_hex(RC4_K_S_TO_C).expect("RC4 key invalid hex form")[..]).into(),
				),
			},
			write_buf: vec![0u8; DEFAULT_BUFFER_SIZE],
		}
	}
	#[instrument(skip(self), fields(ip = ?self.server.get_ref().peer_addr()?))]
	pub async fn run(self: &mut Self) -> io::Result<()> {
		let mut buf = vec![0u8; DEFAULT_BUFFER_SIZE];
		loop {
			select! {
				b = self.client.read_u8() => {
					let raw_packet = read_raw_packet(&mut self.client, &mut buf, b?).await?;

					self.rc4.decipher_client(&mut raw_packet[5..]);

					match ClientPacket::rp_read(&mut &raw_packet[4..]) {
						Ok(p) => {
							match RotmGuard::handle_client_packet(self, &p).await {
								Ok(true) => {}, // 👍
								Ok(false) => continue, // dont forward the packet
								Err(e) => {
									error!("Error handling client packet: {e:?}");
								}

							}
						},
						Err(e) => {
							error!("Error parsing client packet: {e:?}");
						}
					}

					// forward the packet
					self.rc4.cipher_server(&mut raw_packet[5..]);

					self.server.write_all(&raw_packet).await?;
					self.server.flush().await?;
				},
				b = self.server.read_u8() => {
					let raw_packet = read_raw_packet(&mut self.server, &mut buf, b?).await?;

					self.rc4.decipher_server(&mut raw_packet[5..]);

					match ServerPacket::rp_read(&mut &raw_packet[4..]) {
						Ok(p) => {
							match RotmGuard::handle_server_packet(self, &p).await {
								Ok(true) => {}, // 👍
								Ok(false) => continue, // dont forward the packet
								Err(e) => {
									error!("Error handling server packet: {e:?}");
								}

							}
						},
						Err(e) => {
							error!("Error parsing server packet: {e:?}");
						}
					}

					// forward the packet
					self.rc4.cipher_client(&mut raw_packet[5..]);

					self.client.write_all(&raw_packet).await?;
					self.client.flush().await?;
				},
			}
		}
	}
	/// Sends a packet TO the server
	pub async fn send_server(&mut self, packet: &ClientPacket) -> io::Result<()> {
		self.write_buf.clear();
		0u32.rp_write(&mut self.write_buf)?; // placeholder for length

		let packet_length = packet.rp_write(&mut self.write_buf)? as u32 + 4; // +4 because the packet length itself is included
		self.write_buf[0..4].copy_from_slice(&u32::to_be_bytes(packet_length)[..]);

		self.rc4.cipher_server(&mut self.write_buf[5..]);

		self.server.write_all(&self.write_buf).await?;
		self.server.flush().await?;

		Ok(())
	}
	/// Sends a packet TO the client
	pub async fn send_client(&mut self, packet: &ServerPacket) -> io::Result<()> {
		self.write_buf.clear();
		0u32.rp_write(&mut self.write_buf)?; // placeholder for length

		let packet_length = packet.rp_write(&mut self.write_buf)? as u32 + 4; // +4 because the packet length itself is included
		self.write_buf[0..4].copy_from_slice(&u32::to_be_bytes(packet_length)[..]);

		self.rc4.cipher_client(&mut self.write_buf[5..]);

		self.client.write_all(&self.write_buf).await?;
		self.client.flush().await?;

		Ok(())
	}
}

impl Rc4State {
	pub fn decipher_client(&mut self, buf: &mut [u8]) {
		self.client_in.apply_keystream(buf);
	}
	pub fn decipher_server(&mut self, buf: &mut [u8]) {
		self.server_in.apply_keystream(buf);
	}
	pub fn cipher_client(&mut self, buf: &mut [u8]) {
		self.client_out.apply_keystream(buf);
	}
	pub fn cipher_server(&mut self, buf: &mut [u8]) {
		self.server_out.apply_keystream(buf);
	}
}

/// Reads the packet length prefix and reads that exact number of bytes into a buffer
async fn read_raw_packet<'a>(
	socket: &mut BufReader<TcpStream>,
	buf: &'a mut Vec<u8>,
	first_byte: u8,
) -> io::Result<&'a mut [u8]> {
	buf[0] = first_byte;
	socket.read_exact(&mut buf[1..5]).await?;
	let packet_size = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;

	if packet_size < 5 {
		return Err(io::Error::new(
			io::ErrorKind::InvalidData,
			format!("Packet size cannot be less than 5: {packet_size}"),
		));
	}

	buf.reserve_exact(packet_size);

	socket.read_exact(&mut buf[5..packet_size]).await?;

	Ok(&mut buf[..packet_size])
}

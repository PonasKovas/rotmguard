use crate::rc4::Rc4;
use anyhow::{Result, bail};
use bytes::{Buf as _, BytesMut};
use tokio::{io::AsyncReadExt as _, net::tcp::OwnedReadHalf};

// BufReader buffer size
const BUFFER_SIZE: usize = 8 * 1024;

const MAX_PACKET_LENGTH: usize = 10 * 1024 * 1024; // 10 MiB

pub struct Reader {
	stream: OwnedReadHalf,
	buf: BytesMut,
	buf_read: usize,
	rc4: Rc4,
}

impl Reader {
	pub fn new(stream: OwnedReadHalf, rc4_key: &[u8]) -> Self {
		Self {
			stream,
			buf: BytesMut::new(),
			buf_read: 0,
			rc4: Rc4::new(rc4_key),
		}
	}
	// reads more data into the buffer, returns how many bytes were read
	//
	// cancel safe
	pub async fn read_more(&mut self) -> Result<usize, tokio::io::Error> {
		// first reserve space for at least BUFFER_SIZE bytes
		self.buf.resize(self.buf_read + BUFFER_SIZE, 0u8);

		let read = self.stream.read(&mut self.buf[self.buf_read..]).await?;

		self.buf_read += read;

		Ok(read)
	}
	// Tries to get a packet from the read buffer, if a full one is there.
	// Doesnt validate anything, just reads the length prefix and gets that many bytes,
	// deciphers
	pub fn try_get_packet(&mut self) -> Result<Option<BytesMut>> {
		if self.buf_read < 5 {
			// minimum for any valid packet is 5 bytes - 4 for length and 1 for packet id
			return Ok(None);
		}

		let packet_length =
			u32::from_be_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;

		if packet_length < 5 {
			bail!("packet too small: {packet_length} bytes");
		}

		if packet_length > MAX_PACKET_LENGTH {
			bail!("packet too big: {packet_length} bytes");
		}

		if self.buf_read < packet_length {
			// packet length includes itself
			return Ok(None);
		}

		// we have a full packet
		self.buf_read -= packet_length;

		// decipher
		self.rc4.apply(&mut self.buf[5..packet_length]);

		// split it off
		// separate the actual packet data
		let mut packet_bytes = self.buf.split_to(packet_length);
		// remove the packet length prefix
		packet_bytes.advance(4);

		Ok(Some(packet_bytes))
	}
}

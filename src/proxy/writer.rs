use crate::rc4::Rc4;
use anyhow::Result;
use bytes::Bytes;
use tokio::{io::AsyncWriteExt as _, net::tcp::OwnedWriteHalf, sync::mpsc::Receiver};
use tracing::{error, warn};

// BufWriter buffer size
const BUFFER_SIZE: usize = 8 * 1024;

pub async fn task(stream: OwnedWriteHalf, channel: Receiver<WriterMessage>, rc4_key: &[u8; 13]) {
	let writer = Writer {
		stream,
		buf: Vec::with_capacity(BUFFER_SIZE),
		rc4: Rc4::new(rc4_key),
		channel,
	};

	if let Err(e) = writer.run().await {
		error!("Writer task: {e:?}");
	}
}

/// Basically a BufWriter but specialized and also handles rc4 cipher
/// i wouldnt have needed this if tokio's BufWriter let me get mutable access to the buffer
/// to cipher the bytes....
struct Writer {
	stream: OwnedWriteHalf,
	buf: Vec<u8>,
	rc4: Rc4,
	channel: Receiver<WriterMessage>,
}
pub enum WriterMessage {
	Flush,
	Bytes(Bytes),
}

impl Writer {
	async fn run(mut self) -> Result<()> {
		let mut last_pkt_id = None;

		loop {
			// timeout added for debugging purposes when connection gets seemingly stuck.
			let msg = match tokio::time::timeout(
				tokio::time::Duration::from_secs(5),
				self.channel.recv(),
			)
			.await
			{
				Ok(Some(msg)) => msg,
				Ok(None) => break,
				Err(_) => {
					// no sent data in 5 seconds?
					// If there are unflushed bytes still, this is a bug on our side
					if !self.buf.is_empty() {
						warn!(
							"No data sent in 5 seconds. Packets still wait unflushed. Last packet ID: {last_pkt_id:?}"
						);
					}
					continue;
				}
			};

			match msg {
				WriterMessage::Flush => {
					self.flush().await?;
				}
				WriterMessage::Bytes(bytes) => {
					// packet len
					self.write(&u32::to_be_bytes(bytes.len() as u32 + 4), false) // includes itself so +4
						.await?;

					// packet id
					self.write(&bytes[0..1], false).await?;

					// packet itself (ciphered)
					self.write(&bytes[1..], true).await?;

					last_pkt_id = Some(bytes[0]);
				}
			}
		}
		Ok(())
	}
	async fn flush(&mut self) -> Result<(), tokio::io::Error> {
		self.stream.write_all(&self.buf).await?;
		self.buf.clear();

		Ok(())
	}
	async fn write(&mut self, data: &[u8], cipher: bool) -> Result<(), tokio::io::Error> {
		let mut to_write = data;

		while !to_write.is_empty() {
			let available_space = self.buf.capacity() - self.buf.len();

			if available_space == 0 {
				self.flush().await?;
				continue;
			}

			let n = available_space.min(to_write.len());
			let start_in_buf = self.buf.len();

			self.buf.extend_from_slice(&to_write[..n]);
			if cipher {
				self.rc4.apply(&mut self.buf[start_in_buf..]);
			}

			to_write = &to_write[n..];
		}

		Ok(())
	}
}

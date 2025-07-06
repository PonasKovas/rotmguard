use crate::util::PACKET_ID::*;
use crate::{Rotmguard, rc4::Rc4};
use anyhow::Result;
use bytes::Bytes;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tokio::{io::AsyncWriteExt as _, net::tcp::OwnedWriteHalf, sync::mpsc::Receiver};
use tracing::{error, warn};

// BufWriter buffer size
const BUFFER_SIZE: usize = 8 * 1024;

// small packets that are sent frequently
// we will avoid flushing the buffers for these packets
// since they usually come together with some other packets that will get it flushed
#[rustfmt::skip]
const LOW_PRIORITY_PACKETS: &[u8] = &[
	S2C_UPDATE, C2S_UPDATEACK, S2C_ENEMYSHOOT, S2C_REALM_SCORE_UPDATE, C2S_SHOOT_ACK,
	S2C_SHOWEFFECT, C2S_PLAYERSHOOT, C2S_OTHERHIT, S2C_DAMAGE, C2S_ENEMYHIT, S2C_PING,
	C2S_PONG, S2C_NOTIFICATION, S2C_CLIENTSTAT, S2C_INVRESULT, C2S_USEITEM, C2S_PLAYERHIT,
	S2C_AOE, C2S_AOEACK,
];

pub async fn task(
	rotmguard: Arc<Rotmguard>,
	stream: OwnedWriteHalf,
	channel: Receiver<Bytes>,
	rc4_key: &[u8; 13],
) {
	let writer = Writer {
		stream,
		buf: Vec::with_capacity(BUFFER_SIZE),
		rc4: Rc4::new(rc4_key),
		channel,
	};

	if let Err(e) = writer.run(rotmguard).await {
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
	channel: Receiver<Bytes>,
}

impl Writer {
	async fn run(mut self, rotmguard: Arc<Rotmguard>) -> Result<()> {
		let mut unflushed_packet_ids = Vec::with_capacity(128);
		let mut last_flush = Instant::now();

		loop {
			// timeout added for debugging purposes when connection gets seemingly stuck.
			let bytes = match tokio::time::timeout(
				tokio::time::Duration::from_secs(5),
				self.channel.recv(),
			)
			.await
			{
				Ok(Some(msg)) => msg,
				Ok(None) => break,
				Err(_) => {
					// no sent data in 5 seconds?
					// If there are unflushed bytes still, this might be a bug on our side
					if !self.buf.is_empty() {
						warn!(
							"No data sent in 5 seconds. Packets still wait unflushed. unflushed_packets: {unflushed_packet_ids:?}"
						);
					}
					continue;
				}
			};

			unflushed_packet_ids.push(bytes[0]);
			rotmguard
				.flush_skips
				.total_packets
				.fetch_add(1, Ordering::Relaxed);

			// packet len
			self.write(&u32::to_be_bytes(bytes.len() as u32 + 4), false) // includes itself so +4
				.await?;

			// packet id
			self.write(&bytes[0..1], false).await?;

			// packet itself (ciphered)
			self.write(&bytes[1..], true).await?;

			if !LOW_PRIORITY_PACKETS.contains(&bytes[0]) {
				self.flush().await?;

				unflushed_packet_ids.clear();

				let elapsed = last_flush.elapsed().as_micros() as u64;
				last_flush = Instant::now();
				rotmguard
					.flush_skips
					.flushes
					.fetch_add(1, Ordering::Relaxed);
				rotmguard
					.flush_skips
					.total_time
					.fetch_add(elapsed, Ordering::Relaxed);
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

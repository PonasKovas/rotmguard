use anyhow::Result;
use std::env;
use tokio::{
	fs::File,
	io::{AsyncWriteExt, BufWriter},
	time::Instant,
};

pub enum PacketLogger {
	NonActive,
	Active {
		log: BufWriter<File>,
		start_time: Instant,
	},
}

pub enum Direction {
	C2S,
	S2C,
}

pub fn enabled() -> bool {
	env::var("LOG_PACKETS").is_ok()
}

impl PacketLogger {
	pub async fn new() -> Result<Self> {
		let s = if enabled() {
			let writer = tokio::io::BufWriter::new(
				tokio::fs::File::create(format!("packet_data-{}", chrono::Local::now())).await?,
			);

			Self::Active {
				log: writer,
				start_time: Instant::now(),
			}
		} else {
			Self::NonActive
		};

		Ok(s)
	}
	pub async fn add(&mut self, direction: Direction, packet: &[u8]) -> Result<()> {
		let (log, start_time) = match self {
			PacketLogger::NonActive => return Ok(()),
			PacketLogger::Active { log, start_time } => (log, start_time),
		};

		let elapsed = start_time.elapsed();

		let dir = match direction {
			Direction::C2S => b'c',
			Direction::S2C => b's',
		};

		log.write_u8(dir).await?; // direction - either 'c' or 's' (coming from where?)
		log.write_u128_le(elapsed.as_nanos()).await?; // time as nanoseconds since start of logging
		log.write_u32_le(packet.len() as u32).await?; // packet length
		log.write_all(packet).await?; // the actual packet (first byte is the packet id)

		Ok(())
	}
	pub async fn finish(self) -> Result<()> {
		if let Self::Active {
			mut log,
			start_time: _,
		} = self
		{
			log.flush().await?;
		}

		Ok(())
	}
}

use crate::Direction;
use anyhow::Result;
use tokio::{
	fs::File,
	io::{AsyncWriteExt, BufWriter},
	time::Instant,
};
use tracing::error;

pub struct PacketLogger {
	// option because needs to be moved out on Drop
	log: Option<BufWriter<File>>,
	start_time: Instant,
}

impl PacketLogger {
	pub async fn new() -> Result<Self> {
		let writer = tokio::io::BufWriter::new(
			tokio::fs::File::create(format!("packet_data-{}", chrono::Local::now())).await?,
		);

		Ok(Self {
			log: Some(writer),
			start_time: Instant::now(),
		})
	}
	pub async fn add(&mut self, direction: Direction, packet: &[u8]) -> Result<()> {
		let elapsed = self.start_time.elapsed();

		let dir = match direction {
			Direction::C2S => b'c',
			Direction::S2C => b's',
		};

		let log = self.log.as_mut().unwrap();

		log.write_u8(dir).await?; // direction - either 'c' or 's' (coming from where?)
		log.write_u128_le(elapsed.as_nanos()).await?; // time as nanoseconds since start of logging
		log.write_u32_le(packet.len() as u32).await?; // packet length
		log.write_all(packet).await?; // the actual packet (first byte is the packet id)

		Ok(())
	}
}

impl Drop for PacketLogger {
	fn drop(&mut self) {
		let mut log = self.log.take().unwrap();

		tokio::spawn(async move {
			if let Err(e) = log.flush().await {
				error!("error flushing log: {e}");
			}
		});
	}
}

use crate::Direction;
use std::{
	sync::atomic::{AtomicU64, Ordering},
	time::Duration,
};
use tracing::info;

#[derive(Default)]
pub struct FlushSkips {
	s2c: Stats,
	c2s: Stats,
}

#[derive(Default)]
struct Stats {
	// total packets forwarded/sent
	total_packets: AtomicU64,
	// total IO flushes on the stream
	flushes: AtomicU64,
	// total summed spaces between flushes
	total_time: AtomicU64,
}

impl FlushSkips {
	pub fn add_packet(&self, direction: Direction, elapsed: Duration, flushed: bool) {
		let s = match direction {
			Direction::C2S => &self.c2s,
			Direction::S2C => &self.s2c,
		};

		s.total_packets.fetch_add(1, Ordering::Relaxed);

		if flushed {
			s.flushes.fetch_add(1, Ordering::Relaxed);
			s.total_time
				.fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
		}
	}
}

impl Drop for FlushSkips {
	fn drop(&mut self) {
		for (direction, stats) in [("s2c", &self.s2c), ("c2s", &self.c2s)] {
			let total = stats.total_packets.load(Ordering::Relaxed);
			let flushes = stats.flushes.load(Ordering::Relaxed);
			let total_time = stats.total_time.load(Ordering::Relaxed);

			let percent_flushed = 100.0 * flushes as f32 / total as f32;
			let avg_delay = total_time as f32 / total as f32;

			info!(
				"[{direction}] Flush skip stats: total packets: {total}. Flushed: {percent_flushed:.2}%. Avg delay: {avg_delay:.2}us"
			);
		}
	}
}

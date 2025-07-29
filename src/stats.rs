use crate::Direction;
use std::{
	sync::atomic::{AtomicU64, Ordering},
	time::Duration,
};
use tracing::info;

#[derive(Default)]
pub struct Stats {
	s2c: DirectionStats,
	c2s: DirectionStats,
}

#[derive(Default)]
struct DirectionStats {
	// total bytes sent
	total_bytes: AtomicU64,
	// total packets forwarded/sent
	total_packets: AtomicU64,
	// total IO flushes on the stream
	flushes: AtomicU64,
	// total summed spaces between flushes
	total_time: AtomicU64,
}

impl Stats {
	pub fn add_packet(&self, direction: Direction, size: usize, elapsed: Duration, flushed: bool) {
		let s = match direction {
			Direction::C2S => &self.c2s,
			Direction::S2C => &self.s2c,
		};

		s.total_bytes.fetch_add(size as u64, Ordering::Relaxed);
		s.total_packets.fetch_add(1, Ordering::Relaxed);

		if flushed {
			s.flushes.fetch_add(1, Ordering::Relaxed);
			s.total_time
				.fetch_add(elapsed.as_micros() as u64, Ordering::Relaxed);
		}
	}
}

impl Drop for Stats {
	fn drop(&mut self) {
		for (direction, stats) in [("s2c", &self.s2c), ("c2s", &self.c2s)] {
			let total_bytes = stats.total_bytes.load(Ordering::Relaxed);
			let total_packets = stats.total_packets.load(Ordering::Relaxed);
			let flushes = stats.flushes.load(Ordering::Relaxed);
			let total_time = stats.total_time.load(Ordering::Relaxed);
			let total_time_s = total_time as f32 / 1_000_000.;

			let percent_flushed = 100.0 * flushes as f32 / total_packets as f32;
			let avg_delay = total_time as f32 / total_packets as f32;

			info!(
				"[{direction} stats]
	- Total bytes forwarded: {total_bytes}
	- Total packets: {total_packets}
	- Total proxy time: {total_time_s:.1}s
	- Flushed packets: {percent_flushed:.2}%
	- Avg flush delay: {avg_delay:.2}us"
			);
		}
	}
}

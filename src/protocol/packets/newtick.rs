use super::with_context;
use crate::protocol::{
	PACKET_ID, RPReadError, RotmgStr, read_f32, read_str, read_u8, read_u16, read_u32, write_f32,
	write_str, write_u8, write_u16, write_u32,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::{iter, mem::take};

const STRING_STATS: [u8; 14] = [6, 31, 38, 54, 62, 71, 72, 80, 82, 115, 121, 127, 128, 147];

pub struct NewTick {
	pub tick_id: u32,
	pub tick_time: u32,
	pub real_time_ms: u32,
	pub last_real_time_ms: u16,
	pub statuses: Statuses,
}

pub struct Statuses(u16, Bytes);
pub struct ObjectStatusData {
	pub object_id: u32,
	pub position_x: f32,
	pub position_y: f32,
	pub stats: Stats,
}
pub struct Stats(u32, Bytes);
pub enum StatData {
	String {
		stat_type: u8,
		stat: RotmgStr,
		secondary: i64,
	},
	Int {
		stat_type: u8,
		stat: i64,
		secondary: i64,
	},
}

impl NewTick {
	pub const ID: u8 = PACKET_ID::S2C_NEWTICK;

	with_context! { "NewTick packet";
		pub fn parse(bytes: &mut Bytes) -> Result<NewTick, RPReadError> {
			let tick_id = read_u32(bytes, "tick_id")?;
			let tick_time = read_u32(bytes, "tick_time")?;
			let real_time_ms = read_u32(bytes, "real_time_ms")?;
			let last_real_time_ms = read_u16(bytes, "last_real_time_ms")?;

			let statuses_n = read_u16(bytes, "statuses len")?;
			let statuses = Statuses(statuses_n, take(bytes));

			Ok(NewTick{ tick_id, tick_time, real_time_ms, last_real_time_ms, statuses })
		}
	}
}

impl Statuses {
	pub fn into_iter(&self) -> impl Iterator<Item = Result<ObjectStatusData, RPReadError>> {
		let mut bytes = self.1.clone();
		let mut i = 0;

		iter::from_fn(move || {
			if i == self.0 {
				return None;
			}
			i += 1;

			let parse_result = (|| {
				let object_id = read_compressed_int(&mut bytes, "object_id")? as u32;
				let position_x = read_f32(&mut bytes, "position_x")?;
				let position_y = read_f32(&mut bytes, "position_y")?;

				let n_stats = read_u8(&mut bytes, "stats len")? as u32;
				let stats = Stats(n_stats, bytes.clone());
				for _ in 0..n_stats {
					// skip stats, move to next ObjectStatusData
					read_stat(&mut bytes)?;
				}

				Ok(ObjectStatusData {
					object_id,
					position_x,
					position_y,
					stats,
				})
			})();

			Some(parse_result)
		})
	}
}

impl Stats {
	pub fn into_iter(&self) -> impl Iterator<Item = Result<StatData, RPReadError>> {
		let mut bytes = self.1.clone();
		let mut i = 0;
		iter::from_fn(move || {
			if i == self.0 {
				return None;
			}
			i += 1;

			Some(read_stat(&mut bytes))
		})
	}
}

fn read_stat(data: &mut impl Buf) -> Result<StatData, RPReadError> {
	fn inner(data: &mut impl Buf) -> Result<StatData, RPReadError> {
		let stat_type = read_u8(data, "stat type")?;

		if STRING_STATS.contains(&stat_type) {
			let stat = read_str(data, "primary stat string")?;
			let secondary = read_compressed_int(data, "secondary stat")?;

			Ok(StatData::String {
				stat_type,
				stat,
				secondary,
			})
		} else {
			let stat = read_compressed_int(data, "primary stat int")?;
			let secondary = read_compressed_int(data, "secondary stat")?;

			Ok(StatData::Int {
				stat_type,
				stat,
				secondary,
			})
		}
	}

	inner(data).map_err(|e| RPReadError::WithContext {
		ctx: "StatData".to_owned(),
		inner: Box::new(e),
	})
}

pub struct NewTickBuilder {
	bytes: BytesMut,
	status_n: usize,        // positions in the bytes
	stats_n: Option<usize>, // if 0 statuses, there is no last stats_n
}

pub fn create_newtick(
	tick_id: u32,
	tick_time: u32,
	real_time_ms: u32,
	last_real_time_ms: u16,
) -> NewTickBuilder {
	let mut bytes = BytesMut::new();

	write_u8(PACKET_ID::S2C_NEWTICK, &mut bytes);

	write_u32(tick_id, &mut bytes);
	write_u32(tick_time, &mut bytes);
	write_u32(real_time_ms, &mut bytes);
	write_u16(last_real_time_ms, &mut bytes);

	let status_n = bytes.len();
	write_u16(0, &mut bytes); // status_n

	NewTickBuilder {
		bytes,
		status_n,
		stats_n: None,
	}
}

impl NewTickBuilder {
	pub fn add_object(&mut self, object_id: u32, pos_x: f32, pos_y: f32) {
		let status_n_slice = &mut self.bytes[self.status_n..self.status_n + 2];

		let current_status_n = u16::from_be_bytes(status_n_slice.try_into().unwrap());
		status_n_slice.copy_from_slice(&(current_status_n + 1).to_be_bytes());

		write_compressed_int(object_id as i64, &mut self.bytes);
		write_f32(pos_x, &mut self.bytes);
		write_f32(pos_y, &mut self.bytes);

		let stats_n = self.bytes.len();
		write_u8(0, &mut self.bytes); // stats_n

		self.stats_n = Some(stats_n);
	}
	pub fn add_stat(&mut self, stat: StatData) {
		match self.stats_n {
			Some(n) => {
				self.bytes[n] += 1;
			}
			None => {
				panic!("attempted to add a stat to a NewTick packet without any object statuses")
			}
		}

		match stat {
			StatData::String {
				stat_type,
				stat,
				secondary,
			} => {
				write_u8(stat_type, &mut self.bytes);
				write_str(&*stat, &mut self.bytes);
				write_compressed_int(secondary, &mut self.bytes);
			}
			StatData::Int {
				stat_type,
				stat,
				secondary,
			} => {
				write_u8(stat_type, &mut self.bytes);
				write_compressed_int(stat, &mut self.bytes);
				write_compressed_int(secondary, &mut self.bytes);
			}
		}
	}
	pub fn finish(self) -> Bytes {
		self.bytes.freeze()
	}
}

pub fn write_compressed_int(value: i64, mut out: impl BufMut) {
	let is_negative = value < 0;
	let mut value = value.abs();

	let mut byte = (value & 0b00111111) as u8;
	value >>= 6;
	if value != 0 {
		byte |= 0b10000000;
	}
	if is_negative {
		byte |= 0b01000000;
	}

	write_u8(byte, &mut out);

	while value != 0 {
		let mut byte = (value & 0b01111111) as u8;
		value >>= 7;
		if value != 0 {
			byte |= 0b10000000;
		}
		write_u8(byte, &mut out);
	}
}

pub fn read_compressed_int(
	data: &mut impl Buf,
	explanation: &'static str,
) -> Result<i64, RPReadError> {
	pub fn inner(data: &mut impl Buf) -> Result<i64, RPReadError> {
		let mut byte = read_u8(data, "reading varint")?;

		let is_negative = (byte & 0b01000000) != 0;
		let mut shift = 6;
		let mut value = (byte & 0b00111111) as i64;

		while (byte & 0b10000000) != 0 {
			if shift >= 6 + 7 * 7 {
				return Err(RPReadError::InvalidVarint);
			}

			byte = read_u8(data, "reading varint")?;
			value |= ((byte & 0b01111111) as i64) << shift;
			shift += 7;
		}

		if is_negative {
			value = -value;
		}

		Ok(value)
	}

	inner(data).map_err(|e| RPReadError::WithContext {
		ctx: explanation.to_owned(),
		inner: Box::new(e),
	})
}

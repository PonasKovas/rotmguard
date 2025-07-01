use bytes::{Buf, BufMut, Bytes};
use std::ops::Deref;
use thiserror::Error;

mod packet_ids;
pub mod packets;

pub use packet_ids::PACKET_ID;

#[derive(Error, Debug)]
pub enum RPReadError {
	#[error("not enough data to parse")]
	NotEnoughData,
	#[error("invalid utf-8")]
	InvalidUtf8,
	#[error("invalid varint - too long")]
	InvalidVarint,
	#[error("{ctx}: {inner}")]
	WithContext { ctx: String, inner: Box<Self> },
}

// Functions for reading primitive data types in the ROTMG network format.

pub fn read_u8(data: &mut impl Buf, explanation: &'static str) -> Result<u8, RPReadError> {
	data.try_get_u8().map_err(|_| RPReadError::WithContext {
		ctx: explanation.to_owned(),
		inner: Box::new(RPReadError::NotEnoughData),
	})
}
pub fn write_u8(data: u8, mut out: impl BufMut) {
	out.put_u8(data);
}

pub fn read_u16(data: &mut impl Buf, explanation: &'static str) -> Result<u16, RPReadError> {
	data.try_get_u16().map_err(|_| RPReadError::WithContext {
		ctx: explanation.to_owned(),
		inner: Box::new(RPReadError::NotEnoughData),
	})
}
pub fn write_u16(data: u16, mut out: impl BufMut) {
	out.put_u16(data);
}

pub fn read_u32(data: &mut impl Buf, explanation: &'static str) -> Result<u32, RPReadError> {
	data.try_get_u32().map_err(|_| RPReadError::WithContext {
		ctx: explanation.to_owned(),
		inner: Box::new(RPReadError::NotEnoughData),
	})
}
pub fn write_u32(data: u32, mut out: impl BufMut) {
	out.put_u32(data);
}

pub fn read_f32(data: &mut impl Buf, explanation: &'static str) -> Result<f32, RPReadError> {
	data.try_get_f32().map_err(|_| RPReadError::WithContext {
		ctx: explanation.to_owned(),
		inner: Box::new(RPReadError::NotEnoughData),
	})
}
pub fn write_f32(data: f32, mut out: impl BufMut) {
	out.put_f32(data);
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct RotmgStr(Bytes);
impl RotmgStr {
	fn new(bytes: Bytes) -> Result<Self, RPReadError> {
		match str::from_utf8(&bytes) {
			Ok(_) => Ok(Self(bytes)),
			Err(_) => Err(RPReadError::InvalidUtf8),
		}
	}
}
impl Deref for RotmgStr {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		unsafe { str::from_utf8_unchecked(&self.0) }
	}
}

pub fn read_str(data: &mut impl Buf, explanation: &'static str) -> Result<RotmgStr, RPReadError> {
	fn read_inner(data: &mut impl Buf) -> Result<RotmgStr, RPReadError> {
		let len = read_u16(data, "string len")? as usize;

		if data.remaining() < len {
			return Err(RPReadError::WithContext {
				ctx: format!("string contents ({len} bytes)"),
				inner: Box::new(RPReadError::NotEnoughData),
			});
		}

		let s = data.copy_to_bytes(len);

		RotmgStr::new(s)
	}

	read_inner(data).map_err(|e| RPReadError::WithContext {
		ctx: explanation.to_owned(),
		inner: Box::new(e),
	})
}
pub fn write_str(data: &str, mut out: impl BufMut) {
	let len: u16 = match data.len().try_into() {
		Ok(l) => l,
		Err(_) => panic!("strings cannot be longer than u16::MAX in the rotmg protocol"),
	};

	write_u16(len, &mut out);
	out.put_slice(data.as_bytes());
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

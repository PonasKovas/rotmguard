use bytes::{Buf, BufMut, Bytes};
use std::ops::Deref;
use thiserror::Error;

pub mod packet_ids;
pub mod packets;

#[derive(Error, Debug)]
pub enum RPReadError {
	#[error("not enough data to parse")]
	NotEnoughData,
	#[error("invalid utf-8")]
	InvalidUtf8,
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

use anyhow::{Context, Result, bail};
use bytes::{Buf, BufMut};

mod packet_ids;
pub mod util;

pub use packet_ids::PACKET_ID;

pub fn read_str(mut bytes: &[u8]) -> Result<&str> {
	let len = bytes.try_get_u16().context("string length")? as usize;

	if bytes.remaining() < len {
		bail!("not enough bytes to fit {len} long string");
	}

	Ok(str::from_utf8(&bytes[..len])?)
}
pub fn write_str(data: &str, mut out: impl BufMut) {
	let len: u16 = match data.len().try_into() {
		Ok(l) => l,
		Err(_) => panic!("strings cannot be longer than u16::MAX in the rotmg protocol"),
	};

	out.put_u16(len);
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

	out.put_u8(byte);

	while value != 0 {
		let mut byte = (value & 0b01111111) as u8;
		value >>= 7;
		if value != 0 {
			byte |= 0b10000000;
		}
		out.put_u8(byte);
	}
}

pub fn read_compressed_int(data: &mut impl Buf) -> Result<i64> {
	let mut byte = data.try_get_u8()?;

	let is_negative = (byte & 0b01000000) != 0;
	let mut shift = 6;
	let mut value = (byte & 0b00111111) as i64;

	while (byte & 0b10000000) != 0 {
		if shift >= 6 + 7 * 7 {
			bail!("Varint too long");
		}

		byte = data.try_get_u8()?;
		value |= ((byte & 0b01111111) as i64) << shift;
		shift += 7;
	}

	if is_negative {
		value = -value;
	}

	Ok(value)
}

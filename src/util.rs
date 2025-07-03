use anyhow::{Result, bail};
use bytes::{Buf, BufMut, BytesMut};

mod create_packet;
mod packet_ids;
mod view;

pub use create_packet::*;
pub use packet_ids::PACKET_ID;
pub use view::View;

pub fn read_str<'d, 'c>(mut view: View<'d, 'c>) -> Result<&'d str> {
	let len = view.try_get_u16()? as usize;

	if view.remaining() < len {
		bail!("not enough bytes to fit {len} long string");
	}

	let s = &view.slice()[..len];
	view.advance(len);

	Ok(str::from_utf8(s)?)
}
pub fn write_str(data: &str, mut out: impl BufMut) {
	let len: u16 = match data.len().try_into() {
		Ok(l) => l,
		Err(_) => panic!("strings cannot be longer than u16::MAX in the rotmg protocol"),
	};

	out.put_u16(len);
	out.put_slice(data.as_bytes());
}
pub fn read_compressed_int(mut view: View) -> Result<i64> {
	let mut byte = view.try_get_u8()?;

	let is_negative = (byte & 0b01000000) != 0;
	let mut shift = 6;
	let mut value = (byte & 0b00111111) as i64;

	while (byte & 0b10000000) != 0 {
		if shift >= 6 + 7 * 7 {
			bail!("Varint too long");
		}

		byte = view.try_get_u8()?;
		value |= ((byte & 0b01111111) as i64) << shift;
		shift += 7;
	}

	if is_negative {
		value = -value;
	}

	Ok(value)
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

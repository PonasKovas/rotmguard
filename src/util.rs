use anyhow::{Result, bail};
use bytes::{Buf, BufMut};

mod create_packet;
mod packet_ids;
mod stat_types;
mod view;

pub use create_packet::*;
pub use packet_ids::*;
pub use stat_types::*;
pub use view::View;

// Object stat types that are strings instead of integers
pub const OBJECT_STR_STATS: [u8; 14] = [6, 31, 38, 54, 62, 71, 72, 80, 82, 115, 121, 127, 128, 147];

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
pub fn size_as_compressed_int(value: i64) -> usize {
	let value = value.abs() as u64;

	let bits_needed = 64 - value.leading_zeros() as usize;
	// first byte fits only 6 bits because of sign flag
	if bits_needed <= 6 {
		return 1;
	}
	let remaining_bits = bits_needed - 6;

	// subsequent bytes fit 7 bits
	1 + remaining_bits.div_ceil(7)
}
pub fn read_compressed_int(mut view: View) -> Result<i64> {
	let mut byte = view.try_get_u8()?;

	let is_negative = (byte & 0x40) != 0;
	let mut shift = 6;
	let mut value = (byte & 0x3f) as i64;

	while (byte & 0x80) != 0 {
		if shift >= 64 {
			bail!("Varint too long");
		}

		byte = view.try_get_u8()?;
		value |= ((byte & 0x7f) as i64) << shift;
		shift += 7;
	}

	if is_negative {
		value = -value;
	}

	Ok(value)
}
pub fn write_compressed_int(value: i64, out: impl BufMut) {
	let size = size_as_compressed_int(value);

	write_compressed_int_exact_size(value, size, out);
}
// writes the varint potentially bad-formed with trailing zero bytes so its exactly N bytes long
pub fn write_compressed_int_exact_size(value: i64, exact_size: usize, mut out: impl BufMut) {
	let natural_size = size_as_compressed_int(value);
	if exact_size < natural_size {
		panic!(
			"varint {value} cant be written as {exact_size} bytes as it requires at least {natural_size} bytes inherently. This is a bug.",
		)
	}

	let mut buf = [0x80u8; 10]; // continuation bit set for all bytes
	buf[exact_size - 1] = 0; // remove the continuation bit from the last byte
	if value.is_negative() {
		buf[0] |= 0x40;
	}
	let mut value = value.abs();

	let first_byte = (value & 0x3f) as u8;
	value >>= 6;
	buf[0] |= first_byte;

	for i in 1..natural_size {
		buf[i] |= (value & 0x7f) as u8;
		value >>= 7;
	}

	out.put_slice(&buf[..exact_size]);
}

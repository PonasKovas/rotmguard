use std::borrow::Cow;

/// Write packet/datatype in the game protocol format
pub trait RPWrite {
	// Returns how many bytes were written
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize;
}

impl RPWrite for bool {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		buf.extend_from_slice(if *self { &[1] } else { &[0] });

		1
	}
}

impl RPWrite for u8 {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		buf.extend_from_slice(&self.to_be_bytes()[..]);

		1
	}
}

impl RPWrite for u16 {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		buf.extend_from_slice(&self.to_be_bytes()[..]);

		2
	}
}

impl RPWrite for u32 {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		buf.extend_from_slice(&self.to_be_bytes()[..]);

		4
	}
}

impl RPWrite for i8 {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		buf.extend_from_slice(&self.to_be_bytes()[..]);

		1
	}
}

impl RPWrite for i16 {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		buf.extend_from_slice(&self.to_be_bytes()[..]);

		2
	}
}

impl RPWrite for i32 {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		buf.extend_from_slice(&self.to_be_bytes()[..]);

		4
	}
}

impl RPWrite for f32 {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		buf.extend_from_slice(&self.to_be_bytes()[..]);

		4
	}
}

impl RPWrite for String {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let string_bytes = self.as_bytes();
		let len = string_bytes.len();

		(len as u16).rp_write(buf);
		buf.extend_from_slice(string_bytes);

		2 + string_bytes.len()
	}
}

impl<'a> RPWrite for Cow<'a, str> {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let string_bytes = self.as_bytes();
		let len = string_bytes.len();

		(len as u16).rp_write(buf);
		buf.extend_from_slice(string_bytes);

		2 + string_bytes.len()
	}
}

pub fn write_compressed_int(value: &i64, buf: &mut Vec<u8>) -> usize {
	let is_negative = *value < 0;
	let mut value = value.abs();

	let mut byte = (value & 0b00111111) as u8;
	value >>= 6;
	if value != 0 {
		byte |= 0b10000000;
	}
	if is_negative {
		byte |= 0b01000000;
	}

	let mut written = 0;
	written += byte.rp_write(buf);

	while value != 0 {
		let mut byte = (value & 0b01111111) as u8;
		value >>= 7;
		if value != 0 {
			byte |= 0b10000000;
		}
		written += byte.rp_write(buf);
	}

	written
}

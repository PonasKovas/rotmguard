use std::{
	borrow::Cow,
	io::{self, Error, Read},
};

/// Read packet/datatype in the game protocol format
pub trait RPRead<'a> {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized;
}

impl<'a> RPRead<'a> for bool {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized,
	{
		let mut bytes = [0; 1];
		data.read_exact(&mut bytes)?;

		Ok(bytes[0] != 0)
	}
}

impl<'a> RPRead<'a> for u8 {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized,
	{
		let mut bytes = [0; 1];
		data.read_exact(&mut bytes)?;

		Ok(u8::from_be_bytes(bytes))
	}
}

impl<'a> RPRead<'a> for u16 {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self> {
		let mut bytes = [0; 2];
		data.read_exact(&mut bytes)?;

		Ok(u16::from_be_bytes(bytes))
	}
}

impl<'a> RPRead<'a> for u32 {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized,
	{
		let mut bytes = [0; 4];
		data.read_exact(&mut bytes)?;

		Ok(u32::from_be_bytes(bytes))
	}
}

impl<'a> RPRead<'a> for i8 {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized,
	{
		let mut bytes = [0; 1];
		data.read_exact(&mut bytes)?;

		Ok(i8::from_be_bytes(bytes))
	}
}

impl<'a> RPRead<'a> for i16 {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self> {
		let mut bytes = [0; 2];
		data.read_exact(&mut bytes)?;

		Ok(i16::from_be_bytes(bytes))
	}
}

impl<'a> RPRead<'a> for i32 {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized,
	{
		let mut bytes = [0; 4];
		data.read_exact(&mut bytes)?;

		Ok(i32::from_be_bytes(bytes))
	}
}

impl<'a> RPRead<'a> for f32 {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized,
	{
		let mut bytes = [0; 4];
		data.read_exact(&mut bytes)?;

		Ok(f32::from_be_bytes(bytes))
	}
}

impl<'a> RPRead<'a> for Cow<'a, str> {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized,
	{
		let strlen = u16::rp_read(data)? as usize;

		let r = match std::str::from_utf8(&data[..strlen]) {
			Ok(r) => r,
			Err(e) => {
				return Err(Error::new(
					io::ErrorKind::InvalidData,
					format!("error parsing string: {e:?}"),
				));
			}
		};

		*data = &data[strlen..];

		Ok(Cow::Borrowed(r))
	}
}

pub fn read_compressed_int<'a>(data: &mut &'a [u8]) -> io::Result<i64> {
	let mut byte = u8::rp_read(data)?;
	let is_negative = (byte & 0b01000000) != 0;
	let mut shift = 6;
	let mut value = (byte & 0b00111111) as i64;

	while (byte & 0b10000000) != 0 {
		if shift >= 6 + 7 * 7 {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"Invalid VarInt: too long",
			));
		}

		byte = u8::rp_read(data)?;
		value |= ((byte & 0b01111111) as i64) << shift;
		shift += 7;
	}

	if is_negative {
		value = -value;
	}

	Ok(value)
}

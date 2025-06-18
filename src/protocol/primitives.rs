use super::{RPRead, RPReadError, RPWrite, getn};

impl<'a> RPRead<'a> for u8 {
	type Out = Self;

	fn rp_read(data: &mut &'a [u8]) -> Result<Self::Out, RPReadError> {
		Ok(u8::from_be_bytes(getn(data, 1)?.try_into().unwrap()))
	}
}
impl RPWrite for u8 {
	type Data = Self;

	fn rp_write(data: &Self::Data, mut out: impl bytes::BufMut) {
		out.put_u8(*data);
	}
}

impl<'a> RPRead<'a> for u16 {
	type Out = Self;

	fn rp_read(data: &mut &'a [u8]) -> Result<Self::Out, RPReadError> {
		Ok(u16::from_be_bytes(getn(data, 2)?.try_into().unwrap()))
	}
}
impl RPWrite for u16 {
	type Data = Self;

	fn rp_write(data: &Self::Data, mut out: impl bytes::BufMut) {
		out.put_u16(*data);
	}
}

impl<'a> RPRead<'a> for String {
	type Out = &'a str;

	fn rp_read(data: &mut &'a [u8]) -> Result<Self::Out, RPReadError> {
		let len = u16::rp_read(data)?;
		let s = getn(data, len as usize)?;

		match str::from_utf8(s) {
			Ok(s) => Ok(s),
			Err(_) => Err(RPReadError::InvalidUtf8),
		}
	}
}
impl RPWrite for String {
	type Data = str;

	fn rp_write(data: &Self::Data, mut out: impl bytes::BufMut) {
		let len: u16 = match data.len().try_into() {
			Ok(l) => l,
			Err(_) => panic!("strings cannot be longer than u16::MAX in the protocol"),
		};

		u16::rp_write(&len, &mut out);
		out.put_slice(data.as_bytes());
	}
}

use bytes::BufMut;
use thiserror::Error;

pub mod packet_ids;
pub mod packets;
mod primitives;

#[derive(Error, Debug)]
pub enum RPReadError {
	#[error("not enough data to parse")]
	NotEnoughData,
	#[error("invalid utf-8")]
	InvalidUtf8,
}

pub trait RPRead<'a> {
	type Out;

	fn rp_read(data: &mut &'a [u8]) -> Result<Self::Out, RPReadError>;
}

pub trait RPWrite {
	type Data: ?Sized;

	fn rp_write(data: &Self::Data, out: impl BufMut);
}

fn getn<'a>(data: &mut &'a [u8], n: usize) -> Result<&'a [u8], RPReadError> {
	if data.len() < n {
		return Err(RPReadError::NotEnoughData);
	}

	let (a, b) = data.split_at(n);
	*data = b;
	Ok(a)
}

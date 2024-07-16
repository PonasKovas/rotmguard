use super::ServerPacket;
use crate::{extra_datatypes::ObjectId, read::RPRead, write::RPWrite};
use anyhow::Result;
use std::{
	borrow::Cow,
	io::{self, Write},
};

#[derive(Debug, Clone)]
pub struct CreateSuccess<'a> {
	pub object_id: ObjectId,
	pub char_id: u32,
	pub unknown: Cow<'a, str>,
}

impl<'a> RPRead<'a> for CreateSuccess<'a> {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			object_id: ObjectId(u32::rp_read(data)?),
			char_id: u32::rp_read(data)?,
			unknown: Cow::rp_read(data)?,
		})
	}
}

impl<'a> RPWrite for CreateSuccess<'a> {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.object_id.0.rp_write(buf)?;
		written += self.char_id.rp_write(buf)?;
		written += self.unknown.rp_write(buf)?;

		Ok(written)
	}
}

impl<'a> From<CreateSuccess<'a>> for ServerPacket<'a> {
	fn from(value: CreateSuccess<'a>) -> Self {
		Self::CreateSuccess(value)
	}
}

use super::ServerPacket;
use crate::{extra_datatypes::ObjectId, read::RPRead, write::RPWrite};
use std::io::{self, Read, Write};

#[derive(Debug, Clone)]
pub struct CreateSuccess {
	pub object_id: ObjectId,
	pub char_id: u32,
	pub unknown: String,
}

impl RPRead for CreateSuccess {
	fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			object_id: ObjectId(u32::rp_read(data)?),
			char_id: u32::rp_read(data)?,
			unknown: String::rp_read(data)?,
		})
	}
}

impl RPWrite for CreateSuccess {
	fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
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

impl From<CreateSuccess> for ServerPacket {
	fn from(value: CreateSuccess) -> Self {
		Self::CreateSuccess(value)
	}
}

use super::ServerPacket;
use crate::{extra_datatypes::ObjectId, read::RPRead, write::RPWrite};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct CreateSuccess {
	pub object_id: ObjectId,
	pub char_id: u32,
	pub unknown: String,
}

impl RPRead for CreateSuccess {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
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
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.object_id.0.rp_write(buf);
		written += self.char_id.rp_write(buf);
		written += self.unknown.rp_write(buf);

		written
	}
}

impl From<CreateSuccess> for ServerPacket {
	fn from(value: CreateSuccess) -> Self {
		Self::CreateSuccess(value)
	}
}

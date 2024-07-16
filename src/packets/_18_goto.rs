use super::ServerPacket;
use crate::{
	extra_datatypes::{ObjectId, WorldPos},
	read::RPRead,
	write::RPWrite,
};
use anyhow::Result;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct GotoPacket {
	pub object_id: ObjectId,
	pub position: WorldPos,
	pub unknown: i32,
}

impl<'a> RPRead<'a> for GotoPacket {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			object_id: ObjectId(u32::rp_read(data)?),
			position: WorldPos::rp_read(data)?,
			unknown: i32::rp_read(data)?,
		})
	}
}

impl RPWrite for GotoPacket {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.object_id.0.rp_write(buf)?;
		written += self.position.rp_write(buf)?;
		written += self.unknown.rp_write(buf)?;

		Ok(written)
	}
}

impl<'a> From<GotoPacket> for ServerPacket<'a> {
	fn from(value: GotoPacket) -> Self {
		Self::Goto(value)
	}
}

use super::ClientPacket;
use crate::{
	extra_datatypes::{ObjectId, ProjectileId},
	read::RPRead,
	write::RPWrite,
};
use anyhow::Result;
use std::io::{Write};

#[derive(Debug, Clone, Copy)]
pub struct PlayerHit {
	pub bullet_id: ProjectileId,
}

impl<'a> RPRead<'a> for PlayerHit {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			bullet_id: ProjectileId {
				id: u16::rp_read(data)?,
				owner_id: ObjectId(u32::rp_read(data)?),
			},
		})
	}
}

impl RPWrite for PlayerHit {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.bullet_id.id.rp_write(buf)?;
		written += self.bullet_id.owner_id.0.rp_write(buf)?;

		Ok(written)
	}
}

impl<'a> From<PlayerHit> for ClientPacket<'a> {
	fn from(value: PlayerHit) -> Self {
		Self::PlayerHit(value)
	}
}

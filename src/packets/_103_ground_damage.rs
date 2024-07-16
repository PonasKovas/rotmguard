use super::ClientPacket;
use crate::{extra_datatypes::WorldPos, read::RPRead, write::RPWrite};
use anyhow::Result;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy)]
pub struct GroundDamage {
	pub time: i32,
	pub position: WorldPos,
}

impl<'a> RPRead<'a> for GroundDamage {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			time: i32::rp_read(data)?,
			position: WorldPos::rp_read(data)?,
		})
	}
}

impl RPWrite for GroundDamage {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.time.rp_write(buf)?;
		written += self.position.rp_write(buf)?;

		Ok(written)
	}
}

impl<'a> From<GroundDamage> for ClientPacket<'a> {
	fn from(value: GroundDamage) -> Self {
		Self::GroundDamage(value)
	}
}

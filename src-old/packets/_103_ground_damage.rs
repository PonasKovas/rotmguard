use super::ClientPacket;
use crate::{extra_datatypes::WorldPos, read::RPRead, write::RPWrite};
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub struct GroundDamage {
	pub time: i32,
	pub position: WorldPos,
}

impl RPRead for GroundDamage {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
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
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.time.rp_write(buf);
		written += self.position.rp_write(buf);

		written
	}
}

impl From<GroundDamage> for ClientPacket {
	fn from(value: GroundDamage) -> Self {
		Self::GroundDamage(value)
	}
}

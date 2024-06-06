use super::ClientPacket;
use crate::{extra_datatypes::WorldPos, read::RPRead};
use std::io::Read;

#[derive(Debug, Clone, Copy)]
pub struct GroundDamage {
	pub time: i32,
	pub position: WorldPos,
}

impl RPRead for GroundDamage {
	fn rp_read<R: Read>(data: &mut R) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			time: i32::rp_read(data)?,
			position: WorldPos::rp_read(data)?,
		})
	}
}

impl From<GroundDamage> for ClientPacket {
	fn from(value: GroundDamage) -> Self {
		Self::GroundDamage(value)
	}
}

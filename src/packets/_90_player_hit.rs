use std::io::Read;

use super::ClientPacket;
use crate::read::RPRead;

#[derive(Debug, Clone, Copy)]
pub struct PlayerHit {
	pub bullet_id: u16,
	pub owner_id: u32,
}

impl RPRead for PlayerHit {
	fn rp_read<R: Read>(data: &mut R) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			bullet_id: u16::rp_read(data)?,
			owner_id: u32::rp_read(data)?,
		})
	}
}

impl From<PlayerHit> for ClientPacket {
	fn from(value: PlayerHit) -> Self {
		Self::PlayerHit(value)
	}
}

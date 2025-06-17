use super::ServerPacket;
use crate::{extra_datatypes::WorldPos, read::RPRead, write::RPWrite};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AoePacket {
	pub position: WorldPos,
	pub radius: f32,
	pub damage: u16,
	pub effect: u8,
	pub duration: f32,
	pub orig_type: u16,
	pub color: u32,
	pub armor_piercing: bool,
}

impl RPRead for AoePacket {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			position: WorldPos::rp_read(data)?,
			radius: f32::rp_read(data)?,
			damage: u16::rp_read(data)?,
			effect: u8::rp_read(data)?,
			duration: f32::rp_read(data)?,
			orig_type: u16::rp_read(data)?,
			color: u32::rp_read(data)?,
			armor_piercing: bool::rp_read(data)?,
		})
	}
}

impl RPWrite for AoePacket {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.position.rp_write(buf);
		written += self.radius.rp_write(buf);
		written += self.damage.rp_write(buf);
		written += self.effect.rp_write(buf);
		written += self.duration.rp_write(buf);
		written += self.orig_type.rp_write(buf);
		written += self.color.rp_write(buf);
		written += self.armor_piercing.rp_write(buf);

		written
	}
}

impl From<AoePacket> for ServerPacket {
	fn from(value: AoePacket) -> Self {
		Self::Aoe(value)
	}
}

use super::ClientPacket;
use crate::{extra_datatypes::WorldPos, read::RPRead, write::RPWrite};
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub struct PlayerShoot {
	pub time: u32,
	pub bullet_id: u16,
	pub weapon_id: u16,
	pub projectile_id: u8,
	pub position: WorldPos,
	pub angle: f32,
	pub burst: bool,
	pub pattern_id: u8,
	pub attack_type: u8,
	pub player_pos: WorldPos,
}

impl RPRead for PlayerShoot {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(PlayerShoot {
			time: u32::rp_read(data)?,
			bullet_id: u16::rp_read(data)?,
			weapon_id: u16::rp_read(data)?,
			projectile_id: u8::rp_read(data)?,
			position: WorldPos::rp_read(data)?,
			angle: f32::rp_read(data)?,
			burst: bool::rp_read(data)?,
			pattern_id: u8::rp_read(data)?,
			attack_type: u8::rp_read(data)?,
			player_pos: WorldPos::rp_read(data)?,
		})
	}
}

impl RPWrite for PlayerShoot {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.time.rp_write(buf);
		written += self.bullet_id.rp_write(buf);
		written += self.weapon_id.rp_write(buf);
		written += self.projectile_id.rp_write(buf);
		written += self.position.rp_write(buf);
		written += self.angle.rp_write(buf);
		written += self.burst.rp_write(buf);
		written += self.pattern_id.rp_write(buf);
		written += self.attack_type.rp_write(buf);
		written += self.player_pos.rp_write(buf);

		written
	}
}

impl From<PlayerShoot> for ClientPacket {
	fn from(value: PlayerShoot) -> Self {
		Self::PlayerShoot(value)
	}
}

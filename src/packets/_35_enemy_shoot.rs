use super::ServerPacket;
use crate::{
	extra_datatypes::{ObjectId, ProjectileId, WorldPos},
	read::RPRead,
	write::RPWrite,
};
use anyhow::Result;
use std::io::{self, ErrorKind, Write};

#[derive(Debug, Clone, Copy)]
pub struct EnemyShoot {
	pub bullet_id: ProjectileId,
	pub bullet_type: u8,
	pub position: WorldPos,
	pub angle: f32,
	pub damage: i16,
	pub numshots: u8,
	pub angle_between_shots: f32,
}

impl<'a> RPRead<'a> for EnemyShoot {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let bullet_id = ProjectileId {
			id: u16::rp_read(data)?,
			owner_id: ObjectId(u32::rp_read(data)?),
		};
		let bullet_type = u8::rp_read(data)?;
		let position = WorldPos::rp_read(data)?;
		let angle = f32::rp_read(data)?;
		let damage = i16::rp_read(data)?;

		let (numshots, angle_between_shots) = if data.len() > 0 {
			(u8::rp_read(data)?, f32::rp_read(data)?)
		} else {
			(1, 0.0)
		};

		Ok(Self {
			bullet_id,
			bullet_type,
			position,
			angle,
			damage,
			numshots,
			angle_between_shots,
		})
	}
}

impl RPWrite for EnemyShoot {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.bullet_id.id.rp_write(buf)?;
		written += self.bullet_id.owner_id.0.rp_write(buf)?;
		written += self.bullet_type.rp_write(buf)?;
		written += self.position.rp_write(buf)?;
		written += self.angle.rp_write(buf)?;
		written += self.damage.rp_write(buf)?;

		if self.numshots != 1 {
			written += self.numshots.rp_write(buf)?;
			written += self.angle_between_shots.rp_write(buf)?;
		}

		Ok(written)
	}
}

impl<'a> From<EnemyShoot> for ServerPacket<'a> {
	fn from(value: EnemyShoot) -> Self {
		Self::EnemyShoot(value)
	}
}

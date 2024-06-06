use super::ServerPacket;
use crate::{extra_datatypes::WorldPos, read::RPRead};
use std::io::ErrorKind;

#[derive(Debug, Clone, Copy)]
pub struct EnemyShoot {
	pub bullet_id: u16,
	pub owner_id: u32,
	pub bullet_type: u8,
	pub position: WorldPos,
	pub angle: f32,
	pub damage: i16,
	pub numshots: u8,
	pub angle_between_shots: f32,
}

impl RPRead for EnemyShoot {
	fn rp_read<R: std::io::prelude::Read>(data: &mut R) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		let bullet_id = u16::rp_read(data)?;
		let owner_id = u32::rp_read(data)?;
		let bullet_type = u8::rp_read(data)?;
		let position = WorldPos::rp_read(data)?;
		let angle = f32::rp_read(data)?;
		let damage = i16::rp_read(data)?;

		match u8::rp_read(data) {
			Ok(numshots) => {
				let packet = Self {
					bullet_id,
					owner_id,
					bullet_type,
					position,
					angle,
					damage,
					numshots,
					angle_between_shots: f32::rp_read(data)?,
				};
				Ok(packet)
			}
			Err(e) => {
				if e.kind() == ErrorKind::UnexpectedEof {
					let packet = Self {
						bullet_id,
						owner_id,
						bullet_type,
						position,
						angle,
						damage,
						numshots: 1,
						angle_between_shots: 0.0,
					};
					Ok(packet)
				} else {
					Err(e)
				}
			}
		}
	}
}

impl From<EnemyShoot> for ServerPacket {
	fn from(value: EnemyShoot) -> Self {
		Self::EnemyShoot(value)
	}
}

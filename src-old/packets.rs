#![allow(dead_code)]

mod _101_create_success;
mod _103_ground_damage;
mod _10_new_tick;
mod _11_show_effect;
mod _18_goto;
mod _30_player_shoot;
mod _35_enemy_shoot;
mod _42_update;
mod _44_text;
mod _45_reconnect;
mod _62_move;
mod _64_aoe;
mod _67_notification;
mod _90_player_hit;
mod _9_player_text;

use std::borrow::Cow;

pub use _101_create_success::CreateSuccess;
pub use _103_ground_damage::GroundDamage;
pub use _10_new_tick::NewTick;
pub use _11_show_effect::ShowEffect;
pub use _18_goto::GotoPacket;
pub use _30_player_shoot::PlayerShoot;
pub use _35_enemy_shoot::EnemyShoot;
pub use _42_update::{TileData, UpdatePacket};
pub use _44_text::TextPacket;
pub use _45_reconnect::Reconnect;
pub use _62_move::Move;
pub use _64_aoe::AoePacket;
pub use _67_notification::{NotificationPacket, NotificationType};
pub use _90_player_hit::PlayerHit;
pub use _9_player_text::PlayerText;

use crate::{read::RPRead, write::RPWrite};
use anyhow::{Context, Result};
use derivative::Derivative;

/// From client to server
#[non_exhaustive]
#[repr(u8)]
#[derive(Derivative)]
#[derivative(Debug)]
pub enum ClientPacket {
	PlayerText(PlayerText) = 9,
	PlayerShoot(PlayerShoot) = 30,
	Move(Move) = 62,
	PlayerHit(PlayerHit) = 90,
	GroundDamage(GroundDamage) = 103,
	Escape = 105,
	Unknown {
		id: u8,
		#[derivative(Debug = "ignore")]
		bytes: Vec<u8>,
	}, // not necessarilly unknown, just not defined in this program, probably because irrelevant
}

/// From server to client
#[non_exhaustive]
#[repr(u8)]
#[derive(Derivative)]
#[derivative(Debug)]
pub enum ServerPacket {
	NewTick(NewTick) = 10,
	ShowEffect(ShowEffect) = 11,
	Goto(GotoPacket) = 18,
	EnemyShoot(EnemyShoot) = 35,
	Update(UpdatePacket) = 42,
	Text(TextPacket) = 44,
	Reconnect(Reconnect) = 45,
	Aoe(AoePacket) = 64,
	Notification(NotificationPacket) = 67,
	CreateSuccess(CreateSuccess) = 101,
	Unknown {
		id: u8,
		#[derivative(Debug = "ignore")]
		bytes: Vec<u8>,
	}, // not necessarilly unknown, just not defined in this program, probably because irrelevant
}

impl ClientPacket {
	pub fn discriminator(&self) -> u8 {
		// This is safe because the enum is repr(u8)
		unsafe { *(self as *const _ as *const u8) }
	}
}
impl ServerPacket {
	pub fn discriminator(&self) -> u8 {
		// This is safe because the enum is repr(u8)
		unsafe { *(self as *const _ as *const u8) }
	}
}

impl RPRead for ClientPacket {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let packet_id = u8::rp_read(data)?;

		let packet = match packet_id {
			9 => Self::PlayerText(PlayerText::rp_read(data).context("player_text packet")?),
			30 => Self::PlayerShoot(PlayerShoot::rp_read(data).context("player_shoot packet")?),
			62 => Self::Move(Move::rp_read(data).context("move packet")?),
			90 => Self::PlayerHit(PlayerHit::rp_read(data).context("player_hit packet")?),
			103 => Self::GroundDamage(GroundDamage::rp_read(data).context("ground_damage packet")?),
			105 => Self::Escape,
			_ => Self::Unknown {
				id: packet_id,
				bytes: data.to_owned(),
			},
		};

		Ok(packet)
	}
}

impl RPRead for ServerPacket {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let packet_id = u8::rp_read(data)?;

		let packet = match packet_id {
			10 => Self::NewTick(NewTick::rp_read(data).context("new_tick packet")?),
			11 => Self::ShowEffect(ShowEffect::rp_read(data).context("show_effect packet")?),
			18 => Self::Goto(GotoPacket::rp_read(data).context("goto packet")?),
			35 => Self::EnemyShoot(EnemyShoot::rp_read(data).context("enemy_shoot packet")?),
			42 => Self::Update(UpdatePacket::rp_read(data).context("update packet")?),
			44 => Self::Text(TextPacket::rp_read(data).context("text packet")?),
			45 => Self::Reconnect(Reconnect::rp_read(data).context("reconnect packet")?),
			64 => Self::Aoe(AoePacket::rp_read(data).context("aoe packet")?),
			67 => Self::Notification(
				NotificationPacket::rp_read(data).context("notification packet")?,
			),
			101 => {
				Self::CreateSuccess(CreateSuccess::rp_read(data).context("create_success packet")?)
			}
			_ => Self::Unknown {
				id: packet_id,
				bytes: data.to_owned(),
			},
		};

		Ok(packet)
	}
}

impl RPWrite for ClientPacket {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut bytes_written = 0;

		if let Self::Unknown { id, bytes: _ } = self {
			bytes_written += id.rp_write(buf);
		} else {
			let packet_id: u8 = self.discriminator();
			bytes_written += packet_id.rp_write(buf);
		}

		match self {
			Self::PlayerText(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::PlayerShoot(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::Move(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::PlayerHit(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::GroundDamage(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::Escape => {}
			Self::Unknown { id: _, bytes } => {
				buf.extend_from_slice(bytes);
				bytes_written += bytes.len();
			}
		}

		bytes_written
	}
}

impl RPWrite for ServerPacket {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut bytes_written = 0;

		if let Self::Unknown { id, bytes: _ } = self {
			bytes_written += id.rp_write(buf);
		} else {
			let packet_id: u8 = self.discriminator();
			bytes_written += packet_id.rp_write(buf);
		}

		match self {
			Self::Notification(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::NewTick(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::Aoe(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::Goto(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::Update(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::ShowEffect(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::Reconnect(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::Text(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::EnemyShoot(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::CreateSuccess(p) => {
				bytes_written += p.rp_write(buf);
			}
			Self::Unknown { id: _, bytes } => {
				buf.extend_from_slice(bytes);
				bytes_written += bytes.len();
			}
		}

		bytes_written
	}
}

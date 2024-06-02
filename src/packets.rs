mod _101_create_success;
mod _10_new_tick;
mod _35_enemy_shoot;
mod _42_update;
mod _64_aoe;
mod _67_notification;
mod _90_player_hit;
mod _9_player_text;

pub use _101_create_success::CreateSuccess;
pub use _10_new_tick::NewTick;
pub use _35_enemy_shoot::EnemyShoot;
pub use _42_update::UpdatePacket;
pub use _64_aoe::AoePacket;
pub use _67_notification::Notification;
pub use _90_player_hit::PlayerHit;
pub use _9_player_text::PlayerText;

use crate::{read::RPRead, write::RPWrite};

/// From client to server
#[non_exhaustive]
#[repr(u8)]
pub enum ClientPacket {
    PlayerText(PlayerText) = 9,
    PlayerHit(PlayerHit) = 90,
    Escape = 105,
    Unknown { id: u8 }, // not necessarilly unknown, just not defined in this program, probably because irrelevant
}

/// From server to client
#[non_exhaustive]
#[repr(u8)]
pub enum ServerPacket {
    NewTick(NewTick) = 10,
    EnemyShoot(EnemyShoot) = 35,
    UpdatePacket(UpdatePacket) = 42,
    Aoe(AoePacket) = 64,
    Notification(Notification) = 67,
    CreateSuccess(CreateSuccess) = 101,
    Unknown { id: u8 }, // not necessarilly unknown, just not defined in this program, probably because irrelevant
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
    fn rp_read<R: std::io::prelude::Read>(data: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let packet_id = u8::rp_read(data)?;
        let packet = match packet_id {
            9 => Self::PlayerText(PlayerText::rp_read(data)?),
            90 => Self::PlayerHit(PlayerHit::rp_read(data)?),
            105 => Self::Escape,
            _ => Self::Unknown { id: packet_id },
        };

        Ok(packet)
    }
}

impl RPRead for ServerPacket {
    fn rp_read<R: std::io::prelude::Read>(data: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let packet_id = u8::rp_read(data)?;
        let packet = match packet_id {
            10 => Self::NewTick(NewTick::rp_read(data)?),
            35 => Self::EnemyShoot(EnemyShoot::rp_read(data)?),
            42 => Self::UpdatePacket(UpdatePacket::rp_read(data)?),
            64 => Self::Aoe(AoePacket::rp_read(data)?),
            101 => Self::CreateSuccess(CreateSuccess::rp_read(data)?),
            _ => Self::Unknown { id: packet_id },
        };

        Ok(packet)
    }
}

impl RPWrite for ClientPacket {
    fn rp_write<W: std::io::prelude::Write>(&self, buf: &mut W) -> std::io::Result<usize>
    where
        Self: Sized,
    {
        let mut bytes_written = 0;

        let packet_id = self.discriminator();
        bytes_written += (packet_id as u8).rp_write(buf)?;

        match self {
            Self::Escape => {}
            _ => panic!("Packet id {packet_id} writing not implemented!"),
        }

        Ok(bytes_written)
    }
}

impl RPWrite for ServerPacket {
    fn rp_write<W: std::io::prelude::Write>(&self, buf: &mut W) -> std::io::Result<usize>
    where
        Self: Sized,
    {
        let mut bytes_written = 0;

        let packet_id = self.discriminator();
        bytes_written += (packet_id as u8).rp_write(buf)?;

        match self {
            ServerPacket::Notification(notification) => {
                bytes_written += notification.rp_write(buf)?;
            }
            ServerPacket::NewTick(new_tick) => {
                bytes_written += new_tick.rp_write(buf)?;
            }
            _ => panic!("Packet id {packet_id} writing not implemented!"),
        }

        Ok(bytes_written)
    }
}

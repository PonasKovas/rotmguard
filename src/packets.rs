mod _67_notification;
mod _9_player_text;

pub use _67_notification::Notification;
pub use _9_player_text::PlayerText;

use crate::{read::RPRead, write::RPWrite};

/// From client to server
#[non_exhaustive]
#[repr(u8)]
pub enum ClientPacket {
    PlayerText(PlayerText) = 9,
    Unknown { id: u8 }, // not necessarilly unknown, just not defined in this program, probably because irrelevant
}

/// From server to client
#[non_exhaustive]
#[repr(u8)]
pub enum ServerPacket {
    Notification(Notification) = 67,
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
            _ => panic!("Packet id {packet_id} writing not implemented!"),
        }

        Ok(bytes_written)
    }
}

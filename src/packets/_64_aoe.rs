use super::ServerPacket;
use crate::{
    extra_datatypes::{ObjectStatusData, WorldPos},
    read::RPRead,
    write::RPWrite,
};
use std::io::{self, Error, ErrorKind};

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
    fn rp_read<R: std::io::prelude::Read>(data: &mut R) -> std::io::Result<Self>
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

impl From<AoePacket> for ServerPacket {
    fn from(value: AoePacket) -> Self {
        Self::Aoe(value)
    }
}

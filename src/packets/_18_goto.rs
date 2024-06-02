use super::ServerPacket;
use crate::{
    extra_datatypes::{ObjectStatusData, WorldPos},
    read::RPRead,
    write::RPWrite,
};
use std::io::{self, Error, ErrorKind, Read, Write};

#[derive(Debug, Clone)]
pub struct GotoPacket {
    pub object_id: i32,
    pub position: WorldPos,
    pub unknown: i32,
}

impl RPRead for GotoPacket {
    fn rp_read<R: Read>(data: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            object_id: i32::rp_read(data)?,
            position: WorldPos::rp_read(data)?,
            unknown: i32::rp_read(data)?,
        })
    }
}

impl RPWrite for GotoPacket {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        let mut written = 0;

        written += self.object_id.rp_write(buf)?;
        written += self.position.rp_write(buf)?;
        written += self.unknown.rp_write(buf)?;

        Ok(written)
    }
}

impl From<GotoPacket> for ServerPacket {
    fn from(value: GotoPacket) -> Self {
        Self::Goto(value)
    }
}

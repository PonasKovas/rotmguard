use super::ClientPacket;
use crate::{extra_datatypes::WorldPos, read::RPRead};
use std::io::Read;

#[derive(Debug, Clone)]
pub struct Move {
    pub tick_id: u32,
    pub time: u32,
    // [(time, position)]
    pub move_records: Vec<(u32, WorldPos)>,
}

impl RPRead for Move {
    fn rp_read<R: Read>(data: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let tick_id = u32::rp_read(data)?;
        let time = u32::rp_read(data)?;

        let n_records = u16::rp_read(data)?;
        let mut records = Vec::new();

        for _ in 0..n_records {
            records.push((u32::rp_read(data)?, WorldPos::rp_read(data)?));
        }
        Ok(Self {
            tick_id,
            time,
            move_records: records,
        })
    }
}

impl From<Move> for ClientPacket {
    fn from(value: Move) -> Self {
        Self::Move(value)
    }
}

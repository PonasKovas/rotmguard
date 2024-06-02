use crate::{
    read::{read_compressed_int, RPRead},
    write::{write_compressed_int, RPWrite},
};
use std::io::{self, Read, Write};

#[derive(Debug, Clone)]
pub struct StatData {
    pub stat_type: StatType,
    pub stat: Stat,
    pub secondary_stat: i64,
}

#[derive(Debug, Clone)]
pub enum Stat {
    String(String),
    Int(i64),
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
#[non_exhaustive]
pub enum StatType {
    MaxHP = 0,
    HP = 1,
    MaxMP = 3,
    MP = 4,
    Defense = 21,
    Vitality = 26,
    Name = 31,
    CurrentFame = 57,
    ClassQuestFame = 58,
    Other(u8),
}

impl Stat {
    pub fn as_int(&self) -> i64 {
        match self {
            Stat::String(s) => i64::from_str_radix(s, 10).expect("StatType not valid int"),
            Stat::Int(i) => *i,
        }
    }
    pub fn as_str(&self) -> String {
        match self {
            Stat::String(s) => s.clone(),
            Stat::Int(i) => format!("{i}"),
        }
    }
}

impl RPRead for StatData {
    fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let stat_type = u8::rp_read(data)?;

        let stat =
            if [6, 31, 38, 54, 62, 71, 72, 80, 82, 115, 121, 127, 128, 147].contains(&stat_type) {
                // these are string type stats
                Stat::String(String::rp_read(data)?)
            } else {
                // these are normal (int) type stats
                Stat::Int(read_compressed_int(data)?)
            };

        let stat_type = match stat_type {
            0 => StatType::MaxHP,
            1 => StatType::HP,
            3 => StatType::MaxMP,
            4 => StatType::MP,
            21 => StatType::Defense,
            26 => StatType::Vitality,
            31 => StatType::Name,
            57 => StatType::CurrentFame,
            58 => StatType::ClassQuestFame,
            i => StatType::Other(i),
        };

        Ok(Self {
            stat_type,
            stat,
            secondary_stat: read_compressed_int(data)?,
        })
    }
}

impl RPWrite for StatData {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        let mut written = 0;

        let stat_type = match &self.stat_type {
            StatType::Other(i) => *i,
            s => unsafe { *(s as *const _ as *const u8) },
        };

        written += stat_type.rp_write(buf)?;

        match &self.stat {
            Stat::String(s) => {
                written += s.rp_write(buf)?;
            }
            Stat::Int(i) => {
                written += write_compressed_int(i, buf)?;
            }
        }

        written += write_compressed_int(&self.secondary_stat, buf)?;

        Ok(written)
    }
}

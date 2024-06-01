use crate::{
    read::{read_compressed_int, RPRead},
    write::{write_compressed_int, RPWrite},
};
use std::io::{self, Read, Write};

#[derive(Debug, Clone)]
pub struct StatData {
    pub stat_type: u8,
    pub stat: StatType,
    pub secondary_stat: i64,
}

#[derive(Debug, Clone)]
pub enum StatType {
    String(String),
    Int(i64),
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
                StatType::String(String::rp_read(data)?)
            } else {
                // these are normal (int) type stats
                StatType::Int(read_compressed_int(data)?)
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

        written += self.stat_type.rp_write(buf)?;

        match &self.stat {
            StatType::String(s) => {
                written += s.rp_write(buf)?;
            }
            StatType::Int(i) => {
                written += write_compressed_int(i, buf)?;
            }
        }

        written += write_compressed_int(&self.secondary_stat, buf)?;

        Ok(written)
    }
}

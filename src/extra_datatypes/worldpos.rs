use crate::{read::RPRead, write::RPWrite};
use std::io::{self, Read, Write};

#[derive(Debug, Clone, Copy)]
pub struct WorldPos {
    x: f32,
    y: f32,
}

impl RPRead for WorldPos {
    fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            x: f32::rp_read(data)?,
            y: f32::rp_read(data)?,
        })
    }
}

impl RPWrite for WorldPos {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        let mut written = 0;

        written += self.x.rp_write(buf)?;
        written += self.y.rp_write(buf)?;

        Ok(written)
    }
}

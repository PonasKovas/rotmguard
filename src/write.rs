use std::io::{self, Write};

/// Write packet/datatype in rotmg protocol format
pub trait RPWrite {
    // Returns how many bytes were written
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized;
}

impl RPWrite for u8 {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        buf.write_all(&self.to_be_bytes()[..])?;

        Ok(1)
    }
}

impl RPWrite for u16 {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        buf.write_all(&self.to_be_bytes()[..])?;

        Ok(2)
    }
}

impl RPWrite for u32 {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        buf.write_all(&self.to_be_bytes()[..])?;

        Ok(4)
    }
}

impl RPWrite for i8 {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        buf.write_all(&self.to_be_bytes()[..])?;

        Ok(1)
    }
}

impl RPWrite for i16 {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        buf.write_all(&self.to_be_bytes()[..])?;

        Ok(2)
    }
}

impl RPWrite for i32 {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        buf.write_all(&self.to_be_bytes()[..])?;

        Ok(4)
    }
}

impl RPWrite for f32 {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        buf.write_all(&self.to_be_bytes()[..])?;

        Ok(4)
    }
}

impl RPWrite for String {
    fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
    where
        Self: Sized,
    {
        let string_bytes = self.as_bytes();
        let len = string_bytes.len();

        (len as u16).rp_write(buf)?;
        buf.write_all(&string_bytes)?;

        Ok(2 + string_bytes.len())
    }
}

pub fn write_compressed_int<W: Write>(value: &i64, buf: &mut W) -> io::Result<usize> {
    let is_negative = *value < 0;
    let mut value = value.abs();

    let mut byte = (value & 0b00111111) as u8;
    value = value >> 6;
    if value != 0 {
        byte |= 0b10000000;
    }
    if is_negative {
        byte |= 0b01000000;
    }

    let mut written = 0;
    written += byte.rp_write(buf)?;

    while value != 0 {
        let mut byte = (value & 0b01111111) as u8;
        value = value >> 7;
        if value != 0 {
            byte |= 0b10000000;
        }
        written += byte.rp_write(buf)?;
    }

    Ok(written)
}

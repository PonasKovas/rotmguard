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

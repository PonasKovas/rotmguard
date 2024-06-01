use std::io::{self, Read};

/// Read packet/datatype in rotmg protocol format
pub trait RPRead {
    fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
    where
        Self: Sized;
}

impl RPRead for u8 {
    fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut bytes = [0; 1];
        data.read_exact(&mut bytes)?;

        Ok(u8::from_be_bytes(bytes))
    }
}

impl RPRead for u16 {
    fn rp_read<R: Read>(data: &mut R) -> io::Result<Self> {
        let mut bytes = [0; 2];
        data.read_exact(&mut bytes)?;

        Ok(u16::from_be_bytes(bytes))
    }
}

impl RPRead for u32 {
    fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut bytes = [0; 4];
        data.read_exact(&mut bytes)?;

        Ok(u32::from_be_bytes(bytes))
    }
}

impl RPRead for String {
    fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let strlen = u16::rp_read(data)? as usize;

        let mut bytes = vec![0; strlen];
        data.read_exact(&mut bytes)?;

        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

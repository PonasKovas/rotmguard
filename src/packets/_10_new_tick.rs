use super::ServerPacket;
use crate::{extra_datatypes::ObjectStatusData, read::RPRead, write::RPWrite};
use derivative::Derivative;
use std::io::{self, Error, Read, Write};

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct NewTick<'a> {
	pub tick_id: u32,
	pub tick_time: u32,
	pub real_time_ms: u32,
	pub last_real_time_ms: u16,
	#[derivative(Debug = "ignore")]
	pub statuses: Vec<ObjectStatusData<'a>>,
}

impl<'a> RPRead<'a> for NewTick<'a> {
	fn rp_read(data: &mut &'a [u8]) -> io::Result<Self>
	where
		Self: Sized,
	{
		let tick_id = u32::rp_read(data)?;
		let tick_time = u32::rp_read(data)?;
		let real_time_ms = u32::rp_read(data)?;
		let last_real_time_ms = u16::rp_read(data)?;

		let statuses_n = u16::rp_read(data)?;
		if statuses_n > 10000 {
			return Err(Error::new(
				io::ErrorKind::InvalidData,
				format!("Too many statuses ({statuses_n}) in NewTick."),
			));
		}

		let mut statuses = Vec::with_capacity(statuses_n as usize);
		for _ in 0..statuses_n {
			statuses.push(ObjectStatusData::rp_read(data)?);
		}

		Ok(Self {
			tick_id,
			tick_time,
			real_time_ms,
			last_real_time_ms,
			statuses,
		})
	}
}

impl<'a> RPWrite for NewTick<'a> {
	fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.tick_id.rp_write(buf)?;
		written += self.tick_time.rp_write(buf)?;
		written += self.real_time_ms.rp_write(buf)?;
		written += self.last_real_time_ms.rp_write(buf)?;

		written += (self.statuses.len() as u16).rp_write(buf)?;
		for status in &self.statuses {
			written += status.rp_write(buf)?;
		}

		Ok(written)
	}
}

impl<'a> From<NewTick<'a>> for ServerPacket<'a> {
	fn from(value: NewTick<'a>) -> Self {
		Self::NewTick(value)
	}
}

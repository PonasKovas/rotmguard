use super::ClientPacket;
use crate::{extra_datatypes::WorldPos, read::RPRead, write::RPWrite};
use anyhow::Result;
use std::io::{Write};

#[derive(Debug, Clone)]
pub struct Move {
	pub tick_id: u32,
	pub time: u32,
	// [(time, position)]
	pub move_records: Vec<(u32, WorldPos)>,
}

impl<'a> RPRead<'a> for Move {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
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

impl RPWrite for Move {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.tick_id.rp_write(buf)?;
		written += self.time.rp_write(buf)?;
		written += (self.move_records.len() as u16).rp_write(buf)?;

		for record in &self.move_records {
			written += record.0.rp_write(buf)?;
			written += record.1.rp_write(buf)?;
		}

		Ok(written)
	}
}

impl<'a> From<Move> for ClientPacket<'a> {
	fn from(value: Move) -> Self {
		Self::Move(value)
	}
}

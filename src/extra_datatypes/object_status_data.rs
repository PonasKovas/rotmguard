use crate::{
	read::{read_compressed_int, RPRead},
	write::{write_compressed_int, RPWrite},
};
use std::io::{self, Error, Read, Write};

use super::{StatData, WorldPos};

#[derive(Debug, Clone)]
pub struct ObjectStatusData {
	pub object_id: i64,
	pub position: WorldPos,
	pub stats: Vec<StatData>,
}

impl RPRead for ObjectStatusData {
	fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
	where
		Self: Sized,
	{
		let object_id = read_compressed_int(data)?;
		let position = WorldPos::rp_read(data)?;

		let n_stats = read_compressed_int(data)?;
		if n_stats < 0 || n_stats > 10000 {
			return Err(Error::new(
				io::ErrorKind::InvalidData,
				format!("Invalid number of stats ({n_stats}) in ObjectStatusData. (max 10000)"),
			));
		}

		let mut stats = Vec::with_capacity(n_stats as usize);
		for _ in 0..n_stats {
			stats.push(StatData::rp_read(data)?);
		}

		Ok(Self {
			object_id,
			position,
			stats,
		})
	}
}

impl RPWrite for ObjectStatusData {
	fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += write_compressed_int(&self.object_id, buf)?;
		written += self.position.rp_write(buf)?;

		written += write_compressed_int(&(self.stats.len() as i64), buf)?;
		for stat in &self.stats {
			written += stat.rp_write(buf)?;
		}
		Ok(written)
	}
}

use super::{ObjectId, StatData, WorldPos};
use crate::{
	read::{read_compressed_int, RPRead},
	write::{write_compressed_int, RPWrite},
};
use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub struct ObjectStatusData {
	pub object_id: ObjectId,
	pub position: WorldPos,
	pub stats: Vec<StatData>,
}

impl RPRead for ObjectStatusData {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let object_id = ObjectId(read_compressed_int(data)? as u32);
		let position = WorldPos::rp_read(data)?;

		let n_stats = read_compressed_int(data)?;
		if !(0..=10000).contains(&n_stats) {
			bail!("Invalid number of stats ({n_stats}) in ObjectStatusData. (max 10000)");
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
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += write_compressed_int(&(self.object_id.0 as i64), buf);
		written += self.position.rp_write(buf);

		written += write_compressed_int(&(self.stats.len() as i64), buf);
		for stat in &self.stats {
			written += stat.rp_write(buf);
		}

		written
	}
}

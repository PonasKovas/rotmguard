use super::ServerPacket;
use crate::{
	extra_datatypes::{ObjectId, ObjectStatusData, WorldPos},
	read::{read_compressed_int, RPRead},
	write::{write_compressed_int, RPWrite},
};
use anyhow::{bail, Result};
use derivative::Derivative;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct UpdatePacket<'a> {
	pub player_position: WorldPos,
	pub level_type: u8,
	#[derivative(Debug = "ignore")]
	pub tiles: Vec<TileData>, // x, y, type
	#[derivative(Debug = "ignore")]
	pub new_objects: Vec<(u16, ObjectStatusData<'a>)>, // object type, statuses
	#[derivative(Debug = "ignore")]
	pub to_remove: Vec<ObjectId>, // object that left the viewport
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileData {
	pub x: i16,
	pub y: i16,
	pub tile_type: u16,
}

impl<'a> RPRead<'a> for UpdatePacket<'a> {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let player_position = WorldPos::rp_read(data)?;
		let level_type = u8::rp_read(data)?;

		// Tiles
		let tiles_n = read_compressed_int(data)?;
		if !(0..=10000).contains(&tiles_n) {
			bail!("Invalid number of tiles ({tiles_n}) in UpdatePacket. (max 10000)");
		}
		let mut tiles = Vec::with_capacity(tiles_n as usize);
		for _ in 0..tiles_n {
			tiles.push(TileData {
				x: i16::rp_read(data)?,
				y: i16::rp_read(data)?,
				tile_type: u16::rp_read(data)?,
			});
		}

		// New Objects
		let new_objects_n = read_compressed_int(data)?;
		if !(0..=10000).contains(&new_objects_n) {
			bail!("Invalid number of new objects ({new_objects_n}) in UpdatePacket. (max 10000)");
		}

		let mut new_objects = Vec::with_capacity(new_objects_n as usize);
		for _ in 0..new_objects_n {
			new_objects.push((u16::rp_read(data)?, ObjectStatusData::rp_read(data)?));
		}

		// Objects to remove
		let to_remove_n = read_compressed_int(data)?;
		if !(0..=10000).contains(&to_remove_n) {
			bail!(
				"Invalid number of objects to remove ({to_remove_n}) in UpdatePacket. (max 10000)"
			);
		}

		let mut to_remove = Vec::with_capacity(to_remove_n as usize);
		for _ in 0..to_remove_n {
			to_remove.push(ObjectId(read_compressed_int(data)? as u32));
		}

		Ok(Self {
			player_position,
			level_type,
			tiles,
			new_objects,
			to_remove,
		})
	}
}

impl<'a> RPWrite for UpdatePacket<'a> {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.player_position.rp_write(buf);
		written += self.level_type.rp_write(buf);
		written += write_compressed_int(&(self.tiles.len() as i64), buf);
		for tile in &self.tiles {
			written += tile.x.rp_write(buf);
			written += tile.y.rp_write(buf);
			written += tile.tile_type.rp_write(buf);
		}
		written += write_compressed_int(&(self.new_objects.len() as i64), buf);
		for obj in &self.new_objects {
			written += obj.0.rp_write(buf);
			written += obj.1.rp_write(buf);
		}
		written += write_compressed_int(&(self.to_remove.len() as i64), buf);
		for obj in &self.to_remove {
			written += write_compressed_int(&(obj.0 as i64), buf);
		}

		written
	}
}

impl<'a> From<UpdatePacket<'a>> for ServerPacket<'a> {
	fn from(value: UpdatePacket<'a>) -> Self {
		Self::Update(value)
	}
}

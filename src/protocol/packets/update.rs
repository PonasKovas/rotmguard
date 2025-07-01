use super::{newtick::ObjectStatusData, with_context};
use crate::protocol::{
	PACKET_ID, RPReadError, RotmgStr, packets::newtick::read_status, read_compressed_int, read_f32,
	read_str, read_u8, read_u16, read_u32, write_compressed_int, write_f32, write_str, write_u8,
	write_u16, write_u32,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::{iter, mem::take};

pub struct Update {
	pub player_pos_x: f32,
	pub player_pos_y: f32,
	pub level_type: u8,
	pub tiles: Tiles,
	pub new_objects: NewObjects,
	pub to_remove: ToRemove,
}
pub struct Tiles(u32, Bytes);
pub struct TileData {
	pub x: i16,
	pub y: i16,
	pub tile_type: u16,
}

pub struct NewObjects(u32, Bytes);
pub struct ToRemove(u32, Bytes);

impl Update {
	pub const ID: u8 = PACKET_ID::S2C_UPDATE;

	with_context! { "Update packet";
		pub fn parse(bytes: &mut Bytes) -> Result<Update, RPReadError> {
			let player_pos_x = read_f32(bytes, "player_position_x")?;
			let player_pos_y = read_f32(bytes, "player_position_y")?;
			let level_type = read_u8(bytes, "level_type")?;

			let tiles_n = read_compressed_int(bytes, "tiles_n")? as u32;
			let tiles = Tiles(tiles_n, bytes.clone());

			// skip tiles
			for _ in 0..tiles_n {
				read_tile_data(bytes)?;
			}

			let new_objects_n = read_compressed_int(bytes, "new_objects_n")? as u32;
			let new_objects = NewObjects(new_objects_n, bytes.clone());

			// skip new objects
			for _ in 0..new_objects_n {
				read_new_object(bytes)?;
			}

			let to_remove_n = read_compressed_int(bytes, "to_remove_n")? as u32;
			let to_remove = ToRemove(to_remove_n, bytes.clone());

			Ok(Update{ player_pos_x, player_pos_y, level_type, tiles, new_objects, to_remove })
		}
	}
}

fn read_tile_data(bytes: &mut Bytes) -> Result<TileData, RPReadError> {
	fn inner(bytes: &mut Bytes) -> Result<TileData, RPReadError> {
		let x = read_u16(bytes, "x")? as i16;
		let y = read_u16(bytes, "y")? as i16;
		let tile_type = read_u16(bytes, "tile_type")?;

		Ok(TileData { x, y, tile_type })
	}

	inner(bytes).map_err(|e| RPReadError::WithContext {
		ctx: "Tile data".to_owned(),
		inner: Box::new(e),
	})
}

fn read_new_object(bytes: &mut Bytes) -> Result<(u16, ObjectStatusData), RPReadError> {
	fn inner(bytes: &mut Bytes) -> Result<(u16, ObjectStatusData), RPReadError> {
		let object_type = read_u16(bytes, "object_type")?;

		let status = read_status(bytes)?;

		Ok((object_type, status))
	}

	inner(bytes).map_err(|e| RPReadError::WithContext {
		ctx: "New object".to_owned(),
		inner: Box::new(e),
	})
}

fn read_to_remove(bytes: &mut Bytes) -> Result<u32, RPReadError> {
	fn inner(bytes: &mut Bytes) -> Result<u32, RPReadError> {
		let object_id = read_compressed_int(bytes, "object_id")?;

		Ok(object_id as u32)
	}

	inner(bytes).map_err(|e| RPReadError::WithContext {
		ctx: "to remove".to_owned(),
		inner: Box::new(e),
	})
}

impl Tiles {
	pub fn into_iter(&self) -> impl Iterator<Item = Result<TileData, RPReadError>> {
		let mut bytes = self.1.clone();
		let mut i = 0;

		iter::from_fn(move || {
			if i == self.0 {
				return None;
			}
			i += 1;

			Some(read_tile_data(&mut bytes))
		})
	}
}

impl NewObjects {
	pub fn into_iter(&self) -> impl Iterator<Item = Result<(u16, ObjectStatusData), RPReadError>> {
		let mut bytes = self.1.clone();
		let mut i = 0;

		iter::from_fn(move || {
			if i == self.0 {
				return None;
			}
			i += 1;

			Some(read_new_object(&mut bytes))
		})
	}
}

impl ToRemove {
	pub fn into_iter(&self) -> impl Iterator<Item = Result<u32, RPReadError>> {
		let mut bytes = self.1.clone();
		let mut i = 0;

		iter::from_fn(move || {
			if i == self.0 {
				return None;
			}
			i += 1;

			Some(read_to_remove(&mut bytes))
		})
	}
}

pub struct UpdateBuilder<const STAGE: u8> {
	bytes: BytesMut,
	counter_pos: usize,
}

pub fn create_update(player_pos_x: f32, player_pos_y: f32, level_type: u8) -> UpdateBuilder<0> {
	let mut bytes = BytesMut::new();

	write_u8(PACKET_ID::S2C_UPDATE, &mut bytes);

	write_f32(player_pos_x, &mut bytes);
	write_f32(player_pos_y, &mut bytes);
	write_u8(level_type, &mut bytes);

	let counter_pos = bytes.len();
	write_compressed_int(0, &mut bytes); // tiles_n

	UpdateBuilder { bytes, counter_pos }
}

impl UpdateBuilder<0> {
	pub fn add_tile(&mut self, tile: TileData) {}
}

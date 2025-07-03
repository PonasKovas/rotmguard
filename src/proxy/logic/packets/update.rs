use crate::{
	proxy::{
		Proxy,
		logic::cheats::{antipush, autonexus},
	},
	util::{OBJECT_STR_STATS, View, read_compressed_int, read_str},
};
use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};
use tracing::error;

pub async fn update(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	// Update packets add/remove objects and tiles that are on the client screen
	////////////////////////////////////////////////////////////////////////////

	let _player_pos_x = View(b, c).try_get_f32()?;
	let _player_pos_y = View(b, c).try_get_f32()?;
	let _level_type = View(b, c).try_get_u8()?;

	//
	// TILES
	//

	let tiles_n = read_compressed_int(View(b, c))?;
	for _ in 0..tiles_n {
		let x = View(b, c).try_get_i16()?;
		let y = View(b, c).try_get_i16()?;
		let tile_type = View(b, c).try_get_u16()?;

		autonexus::new_tile(proxy, x, y, tile_type);

		if let Some(new_tile_id) = antipush::new_tile(proxy, x, y, tile_type) {
			// replace last 2 bytes
			(&mut b[(*c - 2)..]).put_u16(new_tile_id);
		}
	}

	//
	// NEW OBJECTS
	//
	let objects_n = read_compressed_int(View(b, c))?;
	for _ in 0..objects_n {
		let _object_type = View(b, c).try_get_u16()?;
		let _object_id = read_compressed_int(View(b, c))? as u32;
		let _position_x = View(b, c).try_get_f32()?;
		let _position_y = View(b, c).try_get_f32()?;
		let n_stats = read_compressed_int(View(b, c))? as usize;

		for _ in 0..n_stats {
			let stat_type = View(b, c).try_get_u8()?;

			if OBJECT_STR_STATS.contains(&stat_type) {
				let _stat = read_str(View(b, c))?;
			} else {
				let _stat = read_compressed_int(View(b, c))?;
			}
			let _secondary = read_compressed_int(View(b, c))?;
		}
	}

	//
	// OBJECTS TO REMOVE
	//
	let to_remove_n = read_compressed_int(View(b, c))?;
	for _ in 0..to_remove_n {
		let _object_id = read_compressed_int(View(b, c))?;
	}

	let leftover = View(b, c).slice().len();
	if leftover > 0 {
		error!(
			"Leftover unparsed bytes at UPDATE packet:\n{:?}",
			&View(b, c).slice()[..leftover.min(500)]
		);
	}

	Ok(false)
}

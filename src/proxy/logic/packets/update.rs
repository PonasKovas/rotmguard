use crate::{
	proxy::{
		Proxy,
		logic::cheats::{antidebuffs, antipush, autonexus, fakeslow},
	},
	util::{
		OBJECT_STR_STATS, STAT_TYPE, View, read_compressed_int, read_str, size_as_compressed_int,
		write_compressed_int, write_compressed_int_exact_size,
	},
};
use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};

pub async fn update(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	// Update packets add/remove objects and tiles that are on the client screen
	////////////////////////////////////////////////////////////////////////////

	let _player_pos_x = View(b, c).try_get_f32()?;
	let _player_pos_y = View(b, c).try_get_f32()?;
	let _level_type = View(b, c).try_get_u8()?;

	//
	// TILES
	//

	let tiles_n_pos = *c;
	let tiles_n = read_compressed_int(View(b, c))?;
	let tile_data_pos = *c;
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

	// if antipush not synced, need to add more tile data to replace all previously sent conveyer tiles
	if let Some(extra_tiles) = antipush::extra_tile_data(proxy) {
		// since this involves adding extra bytes to the packet, we need to sadly make a new buffer
		let mut new_buf = BytesMut::with_capacity(b.len() + extra_tiles.len() * 6 + 1);
		// each extra tile is 6 bytes (i16,i16,u16) and we also add an extra byte for potential increase
		// of the size of tiles_n varint

		// first copy all bytes up to tiles
		new_buf.extend_from_slice(&b[..tiles_n_pos]);

		// write the new tiles_n
		write_compressed_int(tiles_n + extra_tiles.len() as i64, &mut new_buf);

		// then copy all original tile data
		new_buf.extend_from_slice(&b[tile_data_pos..*c]);

		// then write extra tile data
		for (x, y, tile) in extra_tiles {
			new_buf.put_i16(x);
			new_buf.put_i16(y);
			new_buf.put_u16(tile);
		}

		// then copy all remaining bytes and replace the original buffer with the new buffer
		let new_cursor = new_buf.len();
		new_buf.extend_from_slice(&b[*c..]);
		*b = new_buf;
		*c = new_cursor;
	}

	//
	// NEW OBJECTS
	//
	let objects_n = read_compressed_int(View(b, c))?;
	let mut new_objects = Vec::with_capacity(objects_n as usize);
	for _ in 0..objects_n {
		let object_type = View(b, c).try_get_u16()?;
		let object_id = read_compressed_int(View(b, c))? as u32;
		let _position_x = View(b, c).try_get_f32()?;
		let _position_y = View(b, c).try_get_f32()?;
		let n_stats = read_compressed_int(View(b, c))? as usize;

		// amazing protocol with this packet first adding new objects
		// and only then removing old, forcing me to allocate a vector here
		// (old object gets removed and a new one gets added with the same id often)
		new_objects.push((object_id, object_type));

		for _ in 0..n_stats {
			let stat_type = View(b, c).try_get_u8()?;

			if OBJECT_STR_STATS.contains(&stat_type) {
				let _stat = read_str(View(b, c))?;
			} else {
				let stat_pos = *c;
				let mut stat = read_compressed_int(View(b, c))?;

				if object_id == proxy.state.my_obj_id {
					let original_stat_size = size_as_compressed_int(stat);

					autonexus::initial_self_stat(proxy, stat_type, &mut stat);
					autonexus::self_stat(proxy, stat_type, stat).await;

					if stat_type == STAT_TYPE::CONDITION {
						antidebuffs::self_condition_stat(proxy, &mut stat);
						fakeslow::self_condition_stat(proxy, &mut stat);

						// overwrite the stat with the potentially modified one
						write_compressed_int_exact_size(
							stat,
							original_stat_size,
							&mut b[stat_pos..],
						);
					}
				}
			}
			let _secondary = read_compressed_int(View(b, c))?;
		}
	}

	//
	// OBJECTS TO REMOVE
	//
	let to_remove_n = read_compressed_int(View(b, c))?;
	for _ in 0..to_remove_n {
		let object_id = read_compressed_int(View(b, c))?;

		autonexus::remove_object(proxy, object_id as u32);
	}

	for (object_id, object_type) in new_objects {
		autonexus::add_object(proxy, object_id, object_type);
	}

	Ok(false)
}

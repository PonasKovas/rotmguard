use crate::{
	proxy::{
		Proxy,
		logic::{antidebuffs, antipush, autonexus, common, damage_monitor, fakeslow},
		packets::common::parse_object_data,
	},
	util::{
		STAT_TYPE, View, read_compressed_int, size_as_compressed_int, write_compressed_int,
		write_compressed_int_exact_size,
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

		autonexus::new_tile(proxy, x, y, tile_type)?;

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
	// skip at first, to first remove the old objects first and then go back
	// so save the current cursor position
	let new_objects_pos = *c;
	let objects_n = read_compressed_int(View(b, c))?;
	for _ in 0..objects_n {
		let _object_type = View(b, c).try_get_u16()?;
		parse_object_data!(b, c;
			object(_, _, _) => {};
			int_stat(_, _) => {};
			str_stat(_, _) => {};
		);
	}

	//
	// OBJECTS TO REMOVE
	//
	let to_remove_n = read_compressed_int(View(b, c))?;
	for _ in 0..to_remove_n {
		let object_id = read_compressed_int(View(b, c))? as u32;

		damage_monitor::remove_object(proxy, object_id);
		common::remove_object(proxy, object_id);
	}
	let end_cursor = *c;

	//
	// GO BACK TO NEW OBJECTS
	//

	*c = new_objects_pos;
	let objects_n = read_compressed_int(View(b, c))?;
	for _ in 0..objects_n {
		let object_type = View(b, c).try_get_u16()?;

		// let mut dmg_monitor_processor;
		parse_object_data!(b, c;
			object(object_id, _pos_x, _pos_y) => {
				// dmg_monitor_processor = damage_monitor::ObjectStatusProcessor::new(object_id, object_type);

				common::add_object(proxy, object_id, object_type);
			};
			int_stat(stat_type, stat) => {
				common::object_int_stat(proxy, object_id, stat_type, stat);
				// dmg_monitor_processor.add_int_stat(stat_type, stat);

				if object_id == proxy.state.common.objects.self_id {
					let mut new_stat = stat;
					let original_stat_size = size_as_compressed_int(stat);

					if stat_type == STAT_TYPE::CONDITION {
						antidebuffs::self_condition_stat(proxy, &mut new_stat);
						fakeslow::self_condition_stat(proxy, &mut new_stat);

						// overwrite the stat with the potentially modified one
						let stat_pos = *c - original_stat_size;
						write_compressed_int_exact_size(
							new_stat,
							original_stat_size,
							&mut b[stat_pos..],
						);
					}
				}
			};
			str_stat(stat_type, stat) => {
				// dmg_monitor_processor.add_str_stat(stat_type, stat);
				common::object_str_stat(proxy, object_id, stat_type, stat);
			};
		);

		// dmg_monitor_processor.finish(proxy);
	}

	*c = end_cursor;

	Ok(false)
}

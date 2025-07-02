use std::io::Cursor;

use crate::{protocol::read_compressed_int, proxy::Proxy};
use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};

// the tile with which all pushing tiles are replaced when antipush enabled
const ANTIPUSH_REPLACEMENT_TILE: u16 = 0x2230; // Spider dirt ground, which reduces walking speed to 35%
// chosen specifically for this reason, because it would be suspicious (even though as far as my testing went,
// the server did not do automatic detection or kicking for this) if you walk against the conveyer at normal speed.
// but there is no way to make you slower or faster in a specific direction, so i make you slower in ALL directions.

pub async fn update_packet(proxy: &mut Proxy, packet_bytes: &mut BytesMut) -> Result<bool> {
	// Update packets add/remove objects and tiles that are on the client screen

	let mut view = Cursor::new(&mut packet_bytes[1..]); // first byte is the packet id

	let _player_pos_x = view.try_get_f32()?;
	let _player_pos_y = view.try_get_f32()?;
	let _level_type = view.try_get_u8()?;

	//
	// TILES
	//

	let tiles_n = read_compressed_int(&mut view)?;
	for _ in 0..tiles_n {
		let x = view.try_get_i16()?;
		let y = view.try_get_i16()?;
		let tile_type = view.try_get_u16()?;

		// add and remove any hazardous/pushing tiles to local memory
		match proxy.rotmguard.assets.hazardous_tiles.get(&tile_type) {
			Some(&dmg) => {
				proxy.state.hazardous_tiles.insert((x, y), dmg);
			}
			None => {
				proxy.state.hazardous_tiles.remove(&(x, y));
			}
		}
		match proxy.rotmguard.assets.conveyor_tiles.get(&tile_type) {
			Some(_) => {
				proxy.state.conveyor_tiles.insert((x, y), tile_type);

				// also IF Anti-push is enabled at this moment, immediatelly replace conveyor tiles in place
				if proxy.state.antipush_enabled {
					// replace last 2 bytes
					let pos = view.position() as usize - 2;
					(&mut view.get_mut()[pos..]).put_u16(ANTIPUSH_REPLACEMENT_TILE);
				}
			}
			None => {
				proxy.state.hazardous_tiles.remove(&(x, y));
			}
		}
	}

	//
	// NEW OBJECTS
	//
	// let objects_n = read_compressed_int(&mut view)?;
	// for _ in 0..objects_n {
	// 	let x = view.try_get_i16()?;
	// 	let y = view.try_get_i16()?;
	// 	let tile_type = view.try_get_u16()?;
	// }

	Ok(false)
}

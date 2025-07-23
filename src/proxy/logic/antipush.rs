use crate::{
	Rotmguard,
	proxy::Proxy,
	util::{GREEN, RED, static_notification},
};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use tracing::error;

// the tile with which all pushing tiles are replaced when antipush enabled
const ANTIPUSH_REPLACEMENT_TILE: &str = "Spider Dirt Web"; // Spider dirt ground, which reduces walking speed to 35%
// chosen specifically for this reason, because it would be suspicious (even though as far as my testing went,
// the server did not automatically detect or kick/ban for this) if you walk against the conveyer at normal speed.
// but there is no way to make you slower or faster in a specific direction, so i make you slower in ALL directions.

pub struct AntiPush {
	replacement_tile: u16,
	conveyor_tiles: BTreeMap<(i16, i16), u16>, // position -> original tile type
	// original tile is stored here so it can be restored when anti-push is disabled
	enabled: bool,
	synced: bool,
}

impl AntiPush {
	pub fn new(rotmguard: &Rotmguard) -> Result<Self> {
		let replacement_tile = rotmguard
			.assets
			.tiles
			.iter()
			.find(|(_id, tile)| tile.name == ANTIPUSH_REPLACEMENT_TILE)
			.context("antipush replacement tile")?;

		Ok(Self {
			replacement_tile: *replacement_tile.0 as u16,
			conveyor_tiles: Default::default(),
			enabled: false,
			synced: true,
		})
	}
}
/// To be called when new tiles enter the player screen or are replaced in the Update packet
/// Returns a new tile id, if we need to replace the tile type immediatelly in place
pub fn new_tile(proxy: &mut Proxy, x: i16, y: i16, tile_type: u16) -> Option<u16> {
	let tile = match proxy.rotmguard.assets.tiles.get(&(tile_type as u32)) {
		Some(x) => x,
		None => {
			error!("New tile with unknown tile type");
			return None;
		}
	};

	if tile.is_conveyor {
		proxy
			.state
			.antipush
			.conveyor_tiles
			.insert((x, y), tile_type);

		// also IF Anti-push is enabled at this moment, immediatelly replace conveyor tiles in place
		if proxy.state.antipush.enabled {
			return Some(proxy.state.antipush.replacement_tile);
		}
	} else {
		// if not a conveyor, remove this tile from the conveyor list
		// (this will do nothing if it wasnt there)
		proxy.state.antipush.conveyor_tiles.remove(&(x, y));
	}

	None
}

/// toggles the antipush cheat
pub async fn toggle(proxy: &mut Proxy) {
	proxy.state.antipush.enabled = !proxy.state.antipush.enabled;
	proxy.state.antipush.synced = false;

	let notification = if proxy.state.antipush.enabled {
		static_notification!("Anti push enabled", GREEN)
	} else {
		static_notification!("Anti push disabled", RED)
	};

	proxy.send_client(notification).await;
}

// checks if antipush is not synced, and if so, returns an iterator of extra tile data
// to add to an update packet. (x, y, tile_id)
pub fn extra_tile_data(
	proxy: &mut Proxy,
) -> Option<impl Iterator<Item = (i16, i16, u16)> + ExactSizeIterator> {
	if proxy.state.antipush.synced {
		return None;
	}

	proxy.state.antipush.synced = true;

	Some(
		proxy
			.state
			.antipush
			.conveyor_tiles
			.iter()
			.map(|(&(x, y), &original_tile_id)| {
				if proxy.state.antipush.enabled {
					(x, y, proxy.state.antipush.replacement_tile)
				} else {
					(x, y, original_tile_id)
				}
			}),
	)
}

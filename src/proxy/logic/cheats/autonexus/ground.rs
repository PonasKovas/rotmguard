use std::collections::BTreeMap;

use super::take_damage_raw;
use crate::proxy::Proxy;
use anyhow::Result;
use anyhow::bail;

#[derive(Default)]
pub struct Ground {
	hazardous_tiles: BTreeMap<(i16, i16), i64>, // position -> damage
}

pub fn new_tile(proxy: &mut Proxy, x: i16, y: i16, tile_type: u16) {
	match proxy.rotmguard.assets.hazardous_tiles.get(&tile_type) {
		Some(&dmg) => {
			proxy
				.state
				.autonexus
				.ground
				.hazardous_tiles
				.insert((x, y), dmg);
		}
		None => {
			proxy.state.autonexus.ground.hazardous_tiles.remove(&(x, y));
		}
	}
}

pub async fn ground_damage(proxy: &mut Proxy, x: i16, y: i16) -> Result<()> {
	let damage = match proxy.state.autonexus.ground.hazardous_tiles.get(&(x, y)) {
		Some(dmg) => *dmg,
		None => {
			bail!(
				"Player claims to take ground damage when not standing on hazardous ground! Maybe your assets are outdated?"
			);
		}
	};

	take_damage_raw(proxy, damage).await;

	Ok(())
}

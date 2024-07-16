use super::{take_damage, PacketFlow};
use crate::{
	gen_this_macro,
	module::{autonexus::nexus, BLOCK},
	packets::{GroundDamage, UpdatePacket},
	proxy::Proxy,
};
use anyhow::{bail, Result};
use std::collections::HashMap;
use tracing::error;

gen_this_macro! {autonexus.ground}

#[derive(Debug, Clone)]
pub struct Ground {
	// all once seen ground tiles that could deal damage. Map<(x, y) -> damage>
	pub hazardous_tiles: HashMap<(i16, i16), i64>,
}

impl Ground {
	pub fn new() -> Self {
		Ground {
			hazardous_tiles: HashMap::new(),
		}
	}
	pub async fn ground_damage(
		proxy: &mut Proxy<'_>,
		packet: &mut GroundDamage,
	) -> Result<PacketFlow> {
		let x = packet.position.x as i16;
		let y = packet.position.y as i16;

		let damage = match ground!(proxy).hazardous_tiles.get(&(x, y)) {
			Some(damage) => *damage,
			None => {
				bail!("Player claims to take ground damage when not standing on hazardous ground! Maybe your assets are outdated?");
			}
		};

		take_damage(proxy, damage).await
	}
	pub fn add_tiles(proxy: &mut Proxy<'_>, update: &mut UpdatePacket<'_>) {
		for tile in &update.tiles {
			let tile_type = tile.tile_type as u32;

			// we care about tiles that can do damage
			if let Some(damage) = proxy.assets.hazardous_grounds.get(&tile_type) {
				// Add the tile
				ground!(proxy)
					.hazardous_tiles
					.insert((tile.x, tile.y), *damage);
			}
		}
	}
}

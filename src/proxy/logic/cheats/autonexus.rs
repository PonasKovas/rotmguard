use crate::proxy::Proxy;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Autonexus {
	hazardous_tiles: BTreeMap<(i16, i16), i64>, // position -> damage
}

pub fn new_tile(proxy: &mut Proxy, x: i16, y: i16, tile_type: u16) {
	match proxy.rotmguard.assets.hazardous_tiles.get(&tile_type) {
		Some(&dmg) => {
			proxy.state.autonexus.hazardous_tiles.insert((x, y), dmg);
		}
		None => {
			proxy.state.autonexus.hazardous_tiles.remove(&(x, y));
		}
	}
}

use crate::proxy::Proxy;
use std::collections::BTreeMap;

pub struct DamageMonitor {
	map_name: String,
	players: BTreeMap<u32, Player>,
	enemies: BTreeMap<u32, Enemy>,
}

struct Enemy {
	name: String,
	hp: u32,
	player_damage: BTreeMap<u32, i64>,
}

struct Player {
	name: String,
	status: PlayerStatus,
}

enum PlayerStatus {
	Present,
	Death,
	Nexus,
}

impl Default for DamageMonitor {
	fn default() -> Self {
		Self {
			map_name: String::new(),
			players: BTreeMap::new(),
			enemies: BTreeMap::new(),
		}
	}
}

pub fn set_map_name(proxy: &mut Proxy, name: &str) {
	proxy.state.damage_monitor.map_name = name.to_owned();
}

use super::common::{bullets::BulletId, objects::Item};
use crate::{
	Rotmguard,
	proxy::Proxy,
	util::{CONDITION_BITFLAG, CONDITION2_BITFLAG, GREEN, RED, create_notification},
};
use serde::Deserialize;
use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
};
use tracing::{error, warn};

mod generate_report;

const TAKEN_DAMAGE_CRITERIA: i64 = 1000; // minimum damage an enemy must take to be shown/saved

pub struct DamageMonitor {
	rotmguard: Arc<Rotmguard>,

	map_name: Option<String>,

	players: HashMap<u64, Player>,
	enemies: HashMap<u64, Enemy>,
}

#[derive(Default)]
struct Enemy {
	name: String,
	object_type: u32,
	player_damage: BTreeMap<u64, i64>,
}

#[derive(Default)]
struct Player {
	name: String,
	is_self: bool,
	status: PlayerStatus,
	items: [Option<Item>; 4],
}

#[derive(Default, PartialEq)]
enum PlayerStatus {
	#[default]
	Present,
	Death,
	Nexus,
}

pub async fn command(proxy: &mut Proxy, mut args: impl Iterator<Item = &str>) {
	let report_type: &str;
	let report_id: usize;

	match args.next() {
		None => {
			// no argument = generate live report
			match generate_report::generate_report(&proxy.state.damage_monitor) {
				Some(report) => {
					report_type = "live";
					report_id = proxy.rotmguard.damage_monitor_http.add_live_report(report);
				}
				None => {
					proxy
						.send_client(create_notification("Map name not set", RED))
						.await;
					return;
				}
			}
		}
		Some(arg) => {
			let id = match arg.parse::<usize>() {
				Ok(0) => {
					// thats just the current dungeon......
					proxy
						.send_client(create_notification(
							"Finalised report not prepared yet. Use /dmg for partial report",
							RED,
						))
						.await;
					return;
				}
				Ok(offset) => proxy
					.rotmguard
					.damage_monitor_http
					.find_memory_by_offset(offset),
				Err(_) => proxy.rotmguard.damage_monitor_http.find_memory_by_name(arg),
			};

			report_type = "memory";
			report_id = match id {
				Some(x) => x,
				None => {
					proxy
						.send_client(create_notification("Report not found", RED))
						.await;
					return;
				}
			};
		}
	}

	let port = proxy.rotmguard.damage_monitor_http.port();
	let url = format!("http://127.0.0.1:{port}/{report_type}/{report_id}");
	if proxy.rotmguard.config.settings.damage_monitor.open_browser {
		if let Err(e) = webbrowser::open(&url) {
			proxy
				.send_client(create_notification(
					&format!("Error opening browser: {e}"),
					RED,
				))
				.await;
		}
	}
	proxy
		.send_client(create_notification(
			&format!("Report generated. Open at\n{url}"),
			GREEN,
		))
		.await;
}

pub fn set_map_name(proxy: &mut Proxy, name: &str) {
	proxy.state.damage_monitor.map_name = Some(name.to_owned());
}

/// saves final object data, therefore vital to be called before common::remove_object
/// Removes the object data from the collected stats if it didnt reach the thresholds for damage
pub fn remove_object(proxy: &mut Proxy, obj_id: u32) {
	let obj = match proxy.state.common.objects.get(obj_id) {
		Some(x) => x,
		None => {
			// this happens all the time, server sends non-sensical data
			// just ignore
			return;
		}
	};
	let id = obj.unique_id;

	if obj.is_player {
		// if player didnt do any damage at all, remove from the list completely
		// otherwise mark as nexused
		if !proxy.state.damage_monitor.has_player_done_damage(id) {
			proxy.state.damage_monitor.players.remove(&id);
		} else {
			// if a player is removed, we mark them as nexused by default
			// if we receive a death notification it will be overwritten.
			let equipped_items = obj.equipped_items.clone();
			let player = get_player(proxy, id);
			if player.status == PlayerStatus::Present {
				player.status = PlayerStatus::Nexus;
			}
			player.items = equipped_items;
		}
	} else {
		// for enemies, just check if they have taken enough damage to be saved
		if !proxy.state.damage_monitor.has_enemy_taken_damage(id) {
			proxy.state.damage_monitor.enemies.remove(&id);
		}
	}
}

pub fn death_notification(proxy: &mut Proxy, json: &str) {
	#[derive(Deserialize)]
	struct DeathNotification {
		k: String,
		t: T,
	}
	#[derive(Deserialize)]
	struct T {
		player: String,
		#[allow(dead_code)]
		level: String,
		#[allow(dead_code)]
		enemy: String,
	}

	match json5::from_str::<DeathNotification>(json) {
		Ok(notification) => {
			if notification.k != "s.death" {
				warn!(
					"Unexpected notification for player death. k not equal to 's.death': {json:?}"
				);
				return;
			}

			let player_name = notification.t.player.split(',').next().unwrap();

			// mark the player as dead, if hes in our records
			for (_, player) in &mut proxy.state.damage_monitor.players {
				if player.name == player_name {
					player.status = PlayerStatus::Death;
					break;
				}
			}
		}
		Err(_) => {
			warn!("Unexpected notification format for player death: {json:?}");
			return;
		}
	}
}

pub fn do_damage(proxy: &mut Proxy, target_obj_id: u32, damage_amount: u16, owner_id: u32) {
	let target = match proxy.state.common.objects.get(target_obj_id) {
		Some(x) => x,
		None => return, // just accept that the server sends nonsensical data all the time...
	};
	let shooter = match proxy.state.common.objects.get(owner_id) {
		Some(x) => x,
		None => return, // just accept that the server sends nonsensical data all the time...
	};

	// only interested in players shooting enemies, not the other way around
	if target.is_player || !shooter.is_player {
		return;
	}

	let shooter_id = shooter.unique_id;
	let target_id = target.unique_id;

	get_player(proxy, shooter_id);
	*get_enemy(proxy, target_id)
		.player_damage
		.entry(shooter_id)
		.or_default() += damage_amount as i64;
}

pub async fn enemyhit(proxy: &mut Proxy, bullet_id: u16, shooter_id: u32, target_id: u32) {
	let bullet = match proxy.state.common.bullets.cache.get(&BulletId {
		id: bullet_id,
		owner_id: shooter_id,
	}) {
		Some(x) => *x,
		None => return, // welp... nothing we can do here...
	};

	let my_id = proxy.state.common.objects.self_id;
	match bullet.summoner_id {
		Some(summoner_id) => {
			if summoner_id != my_id {
				error!("enemyhit packet but summoner id is not my own");
				return;
			}
		}
		None => {
			if shooter_id != my_id {
				error!("enemyhit packet but shooter id is not my own");
				return;
			}
		}
	}

	let target_obj = match proxy.state.common.objects.get(target_id) {
		Some(x) => x,
		None => {
			return;
		}
	};

	let bullet_properties = proxy
		.rotmguard
		.assets
		.objects
		.get(&(bullet.object_type as u32))
		.unwrap()
		.projectiles
		.get(&bullet.bullet_type)
		.unwrap();

	let mut total_damage = bullet.damage;

	// no damage if enemy invincible or invulnerable or stasis
	if (target_obj.stats.conditions
		& (CONDITION_BITFLAG::INVULNERABLE
			| CONDITION_BITFLAG::INVINCIBLE
			| CONDITION_BITFLAG::STASIS))
		!= 0
	{
		total_damage = 0;
	}

	if !bullet_properties.armor_piercing
		&& (target_obj.stats.conditions & CONDITION_BITFLAG::ARMOR_BROKEN) == 0
	{
		let mut def = target_obj.stats.def;
		if (target_obj.stats.conditions & CONDITION_BITFLAG::ARMORED) != 0 {
			def += def / 2; // x1.5
		}

		let potential_damage = total_damage as i64 - def;
		// a bullet must always deal at least 10% of its damage, doesnt matter the def
		let min_damage = total_damage as i64 / 10;

		total_damage = potential_damage.max(min_damage) as u16;
	}

	if (target_obj.stats.conditions2 & CONDITION2_BITFLAG::EXPOSED) != 0 {
		total_damage += 20;
	}
	if (target_obj.stats.conditions2 & CONDITION2_BITFLAG::CURSED) != 0 {
		total_damage += total_damage / 4; // x 1.25
	}
	if (target_obj.stats.conditions2 & CONDITION2_BITFLAG::PETRIFIED) != 0 {
		total_damage -= total_damage / 10; // x 0.9
	}

	do_damage(proxy, target_id, total_damage, my_id);
}

impl DamageMonitor {
	pub fn new(rotmguard: &Arc<Rotmguard>) -> Self {
		Self {
			rotmguard: Arc::clone(rotmguard),
			map_name: Default::default(),
			players: Default::default(),
			enemies: Default::default(),
		}
	}
	// checks whether a player has done any damage to any recorded enemy
	fn has_player_done_damage(&self, player_id: u64) -> bool {
		for (_, enemy) in &self.enemies {
			if enemy.player_damage.contains_key(&player_id) {
				return true;
			}
		}

		false
	}
	// calculates the total damage an enemy has taken
	fn enemy_total_damage(&self, enemy_id: u64) -> i64 {
		match self.enemies.get(&enemy_id) {
			Some(enemy) => {
				let mut total_damage = 0;
				for (_player_id, damage) in &enemy.player_damage {
					total_damage += damage;
				}
				total_damage
			}
			None => 0,
		}
	}
	// checks whether an enemy has taken enough damage to be recorded/shown
	fn has_enemy_taken_damage(&self, enemy_id: u64) -> bool {
		self.enemy_total_damage(enemy_id) >= TAKEN_DAMAGE_CRITERIA
	}
}

fn get_obj_type_name(rotmguard: &Rotmguard, obj_type: u32) -> &str {
	match rotmguard.assets.objects.get(&obj_type) {
		Some(obj) => &obj.name,
		None => "{unknown}",
	}
}

/// returns a mut reference to a player or creates a default one
fn get_player(proxy: &mut Proxy, unique_id: u64) -> &mut Player {
	proxy
		.state
		.damage_monitor
		.players
		.entry(unique_id)
		.or_insert_with(|| {
			let (&obj_id, obj) = proxy
				.state
				.common
				.objects
				.objects
				.iter()
				.find(|obj| obj.1.unique_id == unique_id)
				.unwrap();

			Player {
				name: match &obj.name {
					Some(x) => x.clone(),
					None => get_obj_type_name(&proxy.rotmguard, obj.type_id as u32).to_owned(),
				},
				is_self: obj_id == proxy.state.common.objects.self_id,
				status: PlayerStatus::Present,
				items: obj.equipped_items.clone(),
			}
		})
}

/// returns a mut reference to an enemy or creates a default one
fn get_enemy(proxy: &mut Proxy, unique_id: u64) -> &mut Enemy {
	proxy
		.state
		.damage_monitor
		.enemies
		.entry(unique_id)
		.or_insert_with(|| {
			let (&_obj_id, obj) = proxy
				.state
				.common
				.objects
				.objects
				.iter()
				.find(|obj| obj.1.unique_id == unique_id)
				.unwrap();

			Enemy {
				name: match &obj.name {
					Some(x) => x.clone(),
					None => get_obj_type_name(&proxy.rotmguard, obj.type_id as u32).to_owned(),
				},
				object_type: obj.type_id as u32,
				player_damage: Default::default(),
			}
		})
}

// adds a finalised report to memory on drop
impl Drop for DamageMonitor {
	fn drop(&mut self) {
		if let Some(report) = generate_report::generate_report(self) {
			self.rotmguard.damage_monitor_http.add_final_report(report);
		}
	}
}

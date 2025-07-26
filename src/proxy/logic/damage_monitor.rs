use crate::{
	assets::ProjectileInfo,
	proxy::Proxy,
	util::{CONDITION_BITFLAG, STAT_TYPE},
};
use anyhow::bail;
use base64::{Engine, engine::general_purpose::URL_SAFE};
use bytes::Buf;
use lru::LruCache;
use rng::Rng;
use serde::Deserialize;
use slab::Slab;
use std::{collections::BTreeMap, num::NonZeroUsize};
use tracing::{error, warn};

mod generate_report;
mod rng;

pub use generate_report::generate_report;

const TAKEN_DAMAGE_CRITERIA: i64 = 1000; // minimum damage an enemy must take to be shown/saved
const MY_SHOTS_CACHE_SIZE: usize = 5000; // number of fired shots by self-player to keep in cache at once

pub struct DamageMonitor {
	map_name: String,
	// mapping current server object ids to my own ids
	// object-id -> (is_player, id)
	object_ids: BTreeMap<u32, (bool, usize)>,

	players: Slab<Player>,
	enemies: Slab<Enemy>,

	rng: Rng,
	// (bullet_id, shooter_id) -> (damage, info)
	// shooter_id may not necessarily be my own object id due to summons
	my_shots: LruCache<(u16, u32), (i16, ProjectileInfo)>,
}

#[derive(Default)]
struct Enemy {
	name: String,
	object_type: u32,
	player_damage: BTreeMap<usize, i64>,
}

#[derive(Default)]
struct Player {
	name: String,
	is_self: bool,
	status: PlayerStatus,
	items: [PlayerItem; 4],
}

#[derive(Default, Clone, Copy)]
struct PlayerItem {
	item_id: Option<u32>,           // none if empty slot
	enchantments: [Option<u16>; 4], // same
}

#[derive(Default, PartialEq)]
enum PlayerStatus {
	#[default]
	Present,
	Death,
	Nexus,
}

impl Default for DamageMonitor {
	fn default() -> Self {
		Self {
			map_name: Default::default(),
			object_ids: Default::default(),
			players: Default::default(),
			enemies: Default::default(),
			rng: Rng::new(0),
			my_shots: LruCache::new(NonZeroUsize::new(MY_SHOTS_CACHE_SIZE).unwrap()),
		}
	}
}

pub fn set_map_name(proxy: &mut Proxy, name: &str) {
	proxy.state.damage_monitor.map_name = name.to_owned();
}

pub fn set_rng_seed(proxy: &mut Proxy, seed: u32) {
	proxy.state.damage_monitor.rng = Rng::new(seed);
}

pub fn remove_object(proxy: &mut Proxy, obj_id: u32) {
	let &(is_player, id) = match proxy.state.damage_monitor.object_ids.get(&obj_id) {
		Some(d) => d,
		None => {
			// this happens all the time, server sends non-sensical data
			// just ignore
			return;
		}
	};

	proxy.state.damage_monitor.object_ids.remove(&obj_id);

	if is_player {
		if !proxy.state.damage_monitor.has_player_done_damage(id) {
			proxy.state.damage_monitor.players.remove(id);
		} else {
			// if a player is removed, we mark them as nexused by default
			// if we receive a death notification it will be overwritten.
			let player = &mut proxy.state.damage_monitor.players[id];
			if player.status == PlayerStatus::Present {
				player.status = PlayerStatus::Nexus;
			}
		}
	} else {
		if !proxy.state.damage_monitor.has_enemy_taken_damage(id) {
			proxy.state.damage_monitor.enemies.remove(id);
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

pub fn damage(proxy: &mut Proxy, target_obj_id: u32, damage_amount: u16, owner_id: u32) {
	let &(target_is_player, target_id) =
		match proxy.state.damage_monitor.object_ids.get(&target_obj_id) {
			Some(id) => id,
			None => return, // just accept that the server sends nonsensical data all the time...
		};
	let &(shooter_is_player, shooter_id) =
		match proxy.state.damage_monitor.object_ids.get(&owner_id) {
			Some(id) => id,
			None => return, // just accept that the server sends nonsensical data all the time...
		};

	if target_is_player || !shooter_is_player {
		return;
	}

	*proxy.state.damage_monitor.enemies[target_id]
		.player_damage
		.entry(shooter_id)
		.or_default() += damage_amount as i64;
}

pub fn playershoot(proxy: &mut Proxy, bullet_id: u16, weapon_id: u32, mut projectile_type: u8) {
	if projectile_type == (-1i8) as u8 {
		projectile_type = 0;
	}

	let projectile = match proxy.rotmguard.assets.objects.get(&weapon_id) {
		Some(obj) => match obj.projectiles.get(&projectile_type) {
			Some(x) => x,
			None => {
				error!("playershoot with unknown projectile id? ({weapon_id}, {projectile_type})");
				return; // welp... nothing we can do here...
			}
		},
		None => {
			error!("playershoot with unknown weapon id ({weapon_id})");
			return; // welp... nothing we can do here...
		}
	};

	// calculate damage :)
	let base_damage = match projectile.damage {
		either::Either::Left(fixed) => fixed,
		either::Either::Right((min, max)) => {
			let rng = proxy.state.damage_monitor.rng.next();
			min + (rng % (max - min) as u32) as i32
		}
	};
	let multiplier = calculate_damage_multiplier(proxy);
	let damage = (base_damage as f32 * multiplier) as i16;

	proxy.state.damage_monitor.my_shots.push(
		(bullet_id, proxy.state.common.objects.self_id),
		(damage, projectile.clone()), // todo NOT GOOT CLONE
	);
}

pub fn enemyhit(proxy: &mut Proxy, bullet_id: u16, shooter_id: u32, target_id: u32) {
	let &(dmg, projectile) = match proxy
		.state
		.damage_monitor
		.my_shots
		.get(&(bullet_id, shooter_id))
	{
		Some(x) => x,
		None => return, // welp... nothing we can do here...
	};

	damage(
		proxy,
		target_id,
		dmg as u16,
		proxy.state.common.objects.self_id,
	);
}

#[derive(Default)]
pub struct ObjectStatusProcessor {
	object_id: u32,
	// new objects will always have this set
	object_type: Option<u16>,
	name: Option<String>,
	has_level: bool,
	// equipped item ids
	slots: [Option<i64>; 4],
	// equipped item enchantments
	enchantments: Option<[[Option<u16>; 4]; 4]>,
}
impl ObjectStatusProcessor {
	// when a new object is added
	pub fn new(object_id: u32, object_type: u16) -> Self {
		Self {
			object_id,
			object_type: Some(object_type),
			name: None,
			..Default::default()
		}
	}
	// when an object is updated (newtick)
	pub fn update(object_id: u32) -> Self {
		Self {
			object_id,
			name: None,
			..Default::default()
		}
	}
	pub fn add_int_stat(&mut self, stat_type: u8, stat: i64) {
		match stat_type {
			STAT_TYPE::INVENTORY_0 => {
				self.slots[0] = Some(stat);
			}
			STAT_TYPE::INVENTORY_1 => {
				self.slots[1] = Some(stat);
			}
			STAT_TYPE::INVENTORY_2 => {
				self.slots[2] = Some(stat);
			}
			STAT_TYPE::INVENTORY_3 => {
				self.slots[3] = Some(stat);
			}
			STAT_TYPE::LEVEL => {
				// if has LEVEL stat we can assume its a player
				self.has_level = true;
			}
			_ => {}
		}
	}
	pub fn add_str_stat(&mut self, stat_type: u8, stat: &str) {
		match stat_type {
			STAT_TYPE::NAME => {
				// there might be base64 title ids coming after the name separated by ','
				// but frankly we dont care
				self.name = Some(stat.split(',').next().unwrap().to_owned());
			}
			STAT_TYPE::UNIQUE_DATA_STRING => match parse_enchantments(stat) {
				Ok(e) => {
					self.enchantments = Some(e);
				}
				Err(e) => error!("error parsing enchantments: {e}. full data: {stat:?}"),
			},
			_ => {}
		}
	}
	pub fn finish(self, proxy: &mut Proxy) {
		if let Some(object_type) = self.object_type {
			// add a new object

			if proxy
				.state
				.damage_monitor
				.object_ids
				.get(&self.object_id)
				.is_some()
			{
				// adding an object with an already existing id?
				// unfortunately this happens all the time, server sends non-sensical data
				// just remove the old object
				remove_object(proxy, self.object_id);
			}

			if self.has_level {
				// this is most certainly a player

				let mut player = Player::default();

				if self.object_id == proxy.state.common.objects.self_id {
					player.is_self = true;
				}

				let id = proxy.state.damage_monitor.players.insert(player);

				proxy
					.state
					.damage_monitor
					.object_ids
					.insert(self.object_id, (true, id));
			} else {
				// an enemy....

				let mut enemy = Enemy::default();
				enemy.object_type = object_type as u32;

				// preferrably set the name from the assets
				if let Some(obj_data) = proxy.rotmguard.assets.objects.get(&(object_type as u32)) {
					enemy.name = obj_data.name.clone();
				}

				let id = proxy.state.damage_monitor.enemies.insert(enemy);

				proxy
					.state
					.damage_monitor
					.object_ids
					.insert(self.object_id, (false, id));
			}
		}

		let &(is_player, id) = match proxy.state.damage_monitor.object_ids.get(&self.object_id) {
			None => {
				// this happens all the time, server sends non-sensical data
				// just ignore
				return;
			}
			Some(d) => d,
		};

		if !is_player && self.has_level {
			error!(
				"has level but not recorded as player. must have received LEVEL only in NEWTICK,
not in UPDATE, when it was already recorded as an enemy."
			);
		}

		let entry_name;
		if is_player {
			let entry = &mut proxy.state.damage_monitor.players[id];

			// update the items
			for (i, slot) in self.slots.into_iter().enumerate() {
				let item_id = match slot {
					Some(s) => s,
					None => continue,
				};

				entry.items[i].item_id = (item_id != -1).then_some(item_id as u32);
			}
			if let Some(item_slots) = self.enchantments {
				for (item_slot, enchantments) in item_slots.into_iter().enumerate() {
					entry.items[item_slot].enchantments = enchantments;
				}
			}

			entry_name = &mut entry.name;
		} else {
			// is enemy

			entry_name = &mut proxy.state.damage_monitor.enemies[id].name
		}

		if let Some(name) = self.name {
			*entry_name = name;
		}

		if entry_name.is_empty() {
			let s = if is_player { "player" } else { "enemy" };
			let obj_type = match self.object_type {
				Some(object_type) => format!("{object_type}"),
				None => "?".to_owned(),
			};

			*entry_name = format!(
				"unknown_{s}(type={obj_type}, obj_id={obj_id})",
				obj_id = self.object_id,
			);
		}
	}
}

impl DamageMonitor {
	// checks whether a player has done any damage to any recorded enemy
	fn has_player_done_damage(&self, player_id: usize) -> bool {
		for (_, enemy) in &self.enemies {
			if enemy.player_damage.contains_key(&player_id) {
				return true;
			}
		}

		false
	}
	// calculates the total damage an enemy has taken
	fn enemy_total_damage(&self, enemy_id: usize) -> i64 {
		let mut total_damage = 0;
		for (_player_id, damage) in &self.enemies[enemy_id].player_damage {
			total_damage += damage;
		}
		total_damage
	}
	// checks whether an enemy has taken enough damage to be recorded/shown
	fn has_enemy_taken_damage(&self, enemy_id: usize) -> bool {
		self.enemy_total_damage(enemy_id) >= TAKEN_DAMAGE_CRITERIA
	}
}

fn parse_enchantments(unique_data_str: &str) -> anyhow::Result<[[Option<u16>; 4]; 4]> {
	let mut enchantments = [[None; 4]; 4];

	// there may be more than 4 columns but we only care about the equipped items (first 4 slots)
	for (i, column) in unique_data_str.split(',').take(4).enumerate() {
		if column.is_empty() {
			continue;
		}

		let decoded = match URL_SAFE.decode(column) {
			Ok(d) => d,
			Err(e) => bail!("Error decoding base64: {e}. ({column:?})"),
		};
		let mut slice = &decoded[..];

		slice.try_get_u8()?;
		if slice.try_get_u16_le()? != 0x402 {
			continue;
		}

		for enchant_idx in 0..4 {
			let id = slice.try_get_i16_le()?;
			match id {
				-3 => break,
				-2 | -1 => continue,
				id => {
					enchantments[i][enchant_idx] = Some(id as u16);
				}
			}
		}
	}

	Ok(enchantments)
}

fn calculate_damage_multiplier(proxy: &mut Proxy) -> f32 {
	let tick = proxy.state.autonexus.ticks.current();

	let weak = (tick.stats.conditions & CONDITION_BITFLAG::WEAK) != 0;

	0.5
}

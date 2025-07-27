use crate::{proxy::Proxy, util::STAT_TYPE};
use anyhow::{Result, bail};
use arrayvec::ArrayVec;
use base64::{Engine, engine::general_purpose::URL_SAFE};
use bytes::Buf;
use std::collections::BTreeMap;
use tracing::error;

#[derive(Default)]
pub struct Objects {
	pub self_id: u32,
	unique_id_incr: u64,
	pub objects: BTreeMap<u32, Object>,
}

#[derive(Default)]
pub struct Object {
	pub name: Option<String>,
	pub type_id: u16,
	pub unique_id: u64,
	pub is_player: bool,
	pub stats: Stats,
	pub equipped_items: [Option<Item>; 4],
}

#[derive(Clone, Copy, Default)]
pub struct Stats {
	pub hp: i64,
	pub max_hp: i64,
	pub atk: i64,
	pub def: i64,
	pub vit: i64,
	pub conditions: u64,
	pub conditions2: u64,
	pub exalt_bonus_dmg: i64,
	pub breath: Option<i64>,
	pub blizzard: Option<i64>,
}

#[derive(Default, Clone)]
pub struct Item {
	pub id: u32,
	pub enchantments: ArrayVec<u32, 4>,
}

impl Objects {
	pub fn get(&self, id: u32) -> Option<&Object> {
		self.objects.get(&id)
	}
	pub fn get_mut(&mut self, id: u32) -> Option<&mut Object> {
		self.objects.get_mut(&id)
	}
	pub fn get_self(&mut self) -> &mut Object {
		self.objects.entry(self.self_id).or_default()
	}
}

pub fn add_object(proxy: &mut Proxy, object_id: u32, type_id: u16) {
	let unique_id = proxy.state.common.objects.unique_id_incr;
	proxy.state.common.objects.unique_id_incr += 1;

	proxy.state.common.objects.objects.insert(
		object_id,
		Object {
			name: None,
			type_id,
			unique_id,
			is_player: false, // default is false, will determine later
			stats: Stats::default(),
			equipped_items: Default::default(),
		},
	);
	// server sends duplicate object ids all the time. ignore errors
}

pub fn remove_object(proxy: &mut Proxy, object_id: u32) {
	proxy.state.common.objects.objects.remove(&object_id);
	// server sends objects to remove that werent even added all the time. ignore errors
}

pub async fn object_int_stat(proxy: &mut Proxy, object_id: u32, stat_type: u8, stat: i64) {
	let object = match proxy.state.common.objects.get_mut(object_id) {
		Some(x) => x,
		None => return, // bruh
	};

	match stat_type {
		STAT_TYPE::MAX_HP => {
			object.stats.max_hp = stat;
		}
		STAT_TYPE::HP => {
			object.stats.hp = stat;
		}
		STAT_TYPE::ATTACK => {
			object.stats.atk = stat;
		}
		STAT_TYPE::DEFENSE => {
			object.stats.def = stat;
		}
		STAT_TYPE::VITALITY => {
			object.stats.vit = stat;
		}
		STAT_TYPE::CONDITION => {
			object.stats.conditions = stat as u64;
		}
		STAT_TYPE::CONDITION2 => {
			object.stats.conditions2 = stat as u64;
		}
		STAT_TYPE::EXALTATION_BONUS_DAMAGE => {
			object.stats.exalt_bonus_dmg = stat;
		}
		STAT_TYPE::LEVEL => {
			// we can assume the object is a player
			object.is_player = true;
		}
		STAT_TYPE::INVENTORY_0 => {
			if stat == -1 {
				object.equipped_items[0] = None;
			} else {
				object.equipped_items[0].get_or_insert_default().id = stat as u32;
			}
		}
		STAT_TYPE::INVENTORY_1 => {
			if stat == -1 {
				object.equipped_items[1] = None;
			} else {
				object.equipped_items[1].get_or_insert_default().id = stat as u32;
			}
		}
		STAT_TYPE::INVENTORY_2 => {
			if stat == -1 {
				object.equipped_items[2] = None;
			} else {
				object.equipped_items[2].get_or_insert_default().id = stat as u32;
			}
		}
		STAT_TYPE::INVENTORY_3 => {
			if stat == -1 {
				object.equipped_items[3] = None;
			} else {
				object.equipped_items[3].get_or_insert_default().id = stat as u32;
			}
		}
		STAT_TYPE::BREATH => {
			object.stats.breath = Some(stat);
		}
		STAT_TYPE::BLIZZARD => {
			object.stats.blizzard = Some(stat);
		}
		_ => {}
	}
}

pub fn object_str_stat(proxy: &mut Proxy, object_id: u32, stat_type: u8, stat: &str) {
	let object = match proxy.state.common.objects.get_mut(object_id) {
		Some(x) => x,
		None => return, // bruh
	};

	match stat_type {
		STAT_TYPE::NAME => {
			// there might be base64 title ids coming after the name separated by ','
			// but frankly we dont care
			object.name = Some(
				match stat.split_once(',') {
					Some((x, _)) => x,
					None => stat,
				}
				.to_owned(),
			);
		}
		STAT_TYPE::UNIQUE_DATA_STRING => match parse_enchantments(stat) {
			Ok(slots) => {
				for (i, slot) in slots.into_iter().enumerate() {
					if slot.is_empty() {
						if let Some(x) = &mut object.equipped_items[i] {
							x.enchantments = slot;
						}
					} else {
						object.equipped_items[i]
							.get_or_insert_default()
							.enchantments = slot;
					}
				}
			}
			Err(e) => error!("error parsing enchantments: {e}. full data: {stat:?}"),
		},
		_ => {}
	}
}

fn parse_enchantments(unique_data_str: &str) -> Result<[ArrayVec<u32, 4>; 4]> {
	let mut enchantments = [const { ArrayVec::new_const() }; 4];

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

		for _ in 0..4 {
			let id = slice.try_get_i16_le()?;
			match id {
				-3 => break,
				-2 | -1 => continue,
				id => {
					enchantments[i].push(id as u32);
				}
			}
		}
	}

	Ok(enchantments)
}

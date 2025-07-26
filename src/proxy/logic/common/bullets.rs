use crate::{
	assets::{EnchantmentEffect, ProjectileInfo},
	proxy::Proxy,
	util::CONDITION_BITFLAG,
};
use anyhow::{Result, bail};
use lru::LruCache;
use rng::Rng;
use std::num::NonZeroUsize;
use tracing::error;

mod rng;

// How many bullets to keep track of at the same time, since we are not detecting when they disappear
const BULLETS_CACHE: usize = 100_000; // just to be safe

pub struct Bullets {
	rng: Rng,
	pub cache: LruCache<BulletId, Bullet>,
}

// this game is so coherent that it has duplicating bullet ids, you need them together with the owner id
// to tell them apart
#[derive(PartialEq, Clone, Copy, Debug, Hash, Eq)]
pub struct BulletId {
	pub id: u16,
	pub owner_id: u32,
}

#[derive(PartialEq, Clone, Copy, Debug, Hash, Eq)]
pub struct Bullet {
	pub damage: u16,
	pub summoner_id: Option<u32>,
	pub object_type: u32,
	pub bullet_type: u8,
}

impl Default for Bullets {
	fn default() -> Self {
		Self {
			rng: Rng::new(0),
			cache: LruCache::new(NonZeroUsize::new(BULLETS_CACHE).unwrap()),
		}
	}
}

impl Bullet {
	pub fn get_properties<'a>(&self, proxy: &'a Proxy) -> Result<&'a ProjectileInfo> {
		let object_bullet_types = match proxy.rotmguard.assets.objects.get(&self.object_type) {
			Some(t) => &t.projectiles,
			None => bail!(
				"Bullet shot by enemy ({}) of which assets are not registered. Maybe your assets are outdated?",
				self.object_type
			),
		};

		match object_bullet_types.get(&self.bullet_type) {
			Some(t) => Ok(t),
			None => bail!(
				"Bullet type shot (object {}, bullet {}) of which assets are not registered. Maybe your assets are outdated?",
				self.object_type,
				self.bullet_type
			),
		}
	}
}

pub fn enemyshoot(
	proxy: &mut Proxy,
	bullet_id: u16,
	owner_id: u32,
	bullet_type: u8,
	damage: i16,
	numshots: u8,
) -> Result<()> {
	let object_type = match proxy.state.common.objects.get(owner_id) {
		Some(obj) => obj.type_id as u32,
		// this happens all the time, server sends info about bullets that are not even in visible range
		// its safe to assume that the client ignores these too
		None => return Ok(()),
	};

	// create N bullets with incremental IDs where N is the number of shots
	for i in 0..numshots {
		proxy.state.common.bullets.cache.put(
			BulletId {
				id: bullet_id + i as u16,
				owner_id,
			},
			Bullet {
				damage: damage as u16,
				summoner_id: None,
				object_type,
				bullet_type,
			},
		);
	}

	Ok(())
}

pub fn playershoot(proxy: &mut Proxy, bullet_id: u16, weapon_id: u32, mut projectile_type: u8) {
	// 🙏
	if projectile_type as i8 == -1 {
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
	let (mut min, mut max) = match projectile.damage {
		either::Either::Left(fixed) => (fixed, fixed),
		either::Either::Right((min, max)) => (min, max),
	};

	// apply equipped items enchantments
	for slot in &proxy.state.common.objects.get_self().equipped_items {
		let slot = match slot {
			Some(x) => x,
			None => continue,
		};
		for enchant_id in &slot.enchantments {
			let enchant = match proxy.rotmguard.assets.enchantments.get(enchant_id) {
				Some(x) => x,
				None => {
					error!("player item has unknown enchantment id {enchant_id}");
					continue;
				}
			};
			for effect in &enchant.effects {
				match effect {
					EnchantmentEffect::MinDamageMult(mult) => {
						min = (min as f32 * mult).round() as i32;
					}
					EnchantmentEffect::MaxDamageMult(mult) => {
						max = (max as f32 * mult).round() as i32;
					}
					_ => {}
				}
			}
		}
	}
	let base_damage = {
		let rng = proxy.state.common.bullets.rng.next();

		min + (rng % (max - min) as u32) as i32
	};
	let multiplier = calculate_damage_multiplier(proxy, proxy.state.common.objects.self_id);
	let damage = (base_damage as f32 * multiplier) as i16;

	proxy.state.common.bullets.cache.put(
		BulletId {
			id: bullet_id,
			owner_id: proxy.state.common.objects.self_id,
		},
		Bullet {
			damage: damage as u16,
			summoner_id: None,
			object_type: weapon_id,
			bullet_type: projectile_type,
		},
	);
}

pub fn serverplayershoot(
	proxy: &mut Proxy,
	bullet_id: u16,
	shooter_id: u32,
	summoner_id: u32,
	damage: u16,
	projectile_type: Option<u8>,
	bullet_count: Option<u8>,
) {
	// 🙏
	let projectile_type = match projectile_type {
		Some(x) if x as i8 == -1 => 0,
		None => 0,
		Some(x) => x,
	};

	let shooter_type = match proxy.state.common.objects.get(shooter_id) {
		Some(x) => x.type_id as u32,
		None => {
			error!("serverplayershoot from unknown object");
			return;
		}
	};

	let num_shots = bullet_count.unwrap_or(1);
	for i in 0..num_shots {
		proxy.state.common.bullets.cache.put(
			BulletId {
				id: bullet_id + i as u16,
				owner_id: shooter_id,
			},
			Bullet {
				damage,
				summoner_id: Some(summoner_id),
				object_type: shooter_type,
				bullet_type: projectile_type,
			},
		);
	}
}

fn calculate_damage_multiplier(proxy: &mut Proxy, object_id: u32) -> f32 {
	let obj = match proxy.state.common.objects.get(object_id) {
		Some(x) => x,
		None => {
			// bruh
			error!("Tried calculating damage multiplier of non existing object? {object_id}");
			return 0.5; // whatever man
		}
	};

	let weak = (obj.stats.conditions & CONDITION_BITFLAG::WEAK) != 0;
	let damaging = (obj.stats.conditions & CONDITION_BITFLAG::DAMAGING) != 0;
	let atk = obj.stats.atk as f32;
	let exalt_dmg_bonus = obj.stats.exalt_bonus_dmg as f32 / 1000.0;

	if weak {
		return 0.5;
	}

	let mut mult = (atk + 25.0) * 0.02;

	if damaging {
		mult *= 1.25;
	}

	mult * exalt_dmg_bonus
}

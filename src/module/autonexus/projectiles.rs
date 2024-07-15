use super::{take_damage, PacketFlow, FORWARD};
use crate::{
	assets::ProjectileInfo,
	extra_datatypes::{ObjectId, ProjectileId},
	gen_this_macro,
	module::BLOCK,
	packets::{EnemyShoot, PlayerHit, UpdatePacket},
	proxy::Proxy,
};
use lru::LruCache;
use std::{collections::BTreeMap, io::Result, num::NonZero};
use tracing::{error, trace};

gen_this_macro! {autonexus.projectiles}

// How many bullets to keep track of at the same time.
// If you set this too low autonexus might fail when there are too many bullets on the screen
// simultaneously and you will die
const BULLET_CACHE_SIZE: usize = 10_000;

#[derive(Debug, Clone)]
pub struct Projectiles {
	// all currently visible bullets. key is (bullet id, owner id)
	pub bullets: LruCache<ProjectileId, Bullet>,
	// maps the object id of a currently visible object to it's type id
	pub objects: BTreeMap<ObjectId, u16>,
}

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
	pub damage: i16,
	pub info: ProjectileInfo,
}

impl Projectiles {
	pub fn new() -> Self {
		Projectiles {
			bullets: LruCache::new(NonZero::new(BULLET_CACHE_SIZE).unwrap()),
			objects: BTreeMap::new(),
		}
	}
	pub fn add_remove_objects(proxy: &mut Proxy<'_>, update: &UpdatePacket) {
		// remove objects that left the visible area
		for object in &update.to_remove {
			projectiles!(proxy).objects.remove(object);
		}

		// Add new objects
		for object in &update.new_objects {
			projectiles!(proxy)
				.objects
				.insert(object.1.object_id, object.0);
		}
	}
	pub fn add_bullet(proxy: &mut Proxy<'_>, enemy_shoot: &EnemyShoot) -> PacketFlow {
		let shooter_id = enemy_shoot.bullet_id.owner_id;
		let shooter_object_type = match projectiles!(proxy).objects.get(&shooter_id) {
			Some(object_type) => *object_type as u32,
			None => {
				trace!("EnemyShoot packet with non-visible owner");

				// this happens all the time, server sends info about bullets that are not even in visible range
				// its safe to assume that the client ignores these too
				return PacketFlow::Forward;
			}
		};

		let shooter_projectile_types = match proxy.assets.projectiles.get(&shooter_object_type) {
			Some(types) => types,
			None => {
				error!("Bullet shot by enemy of which assets are not registered. Maybe your assets are outdated?");

				return PacketFlow::Block; // i guess dont forward the packet, better get DCed than die
			}
		};

		let info = match shooter_projectile_types.get(&(enemy_shoot.bullet_type as u32)) {
			Some(info) => *info,
			None => {
				error!("Bullet type shot of which assets are not registered. Maybe your assets are outdated?");

				return PacketFlow::Block; // i guess dont forward the packet, better get DCed than die
			}
		};

		trace!(
			?info,
			"Adding bullets with ids {}..{} (owner {})",
			enemy_shoot.bullet_id.id,
			enemy_shoot.bullet_id.id + enemy_shoot.numshots as u16,
			enemy_shoot.bullet_id.owner_id.0
		);

		// create N bullets with incremental IDs where N is the number of shots
		for i in 0..=enemy_shoot.numshots {
			projectiles!(proxy).bullets.put(
				ProjectileId {
					id: enemy_shoot.bullet_id.id + i as u16,
					owner_id: enemy_shoot.bullet_id.owner_id,
				},
				Bullet {
					damage: enemy_shoot.damage,
					info,
				},
			);
		}

		PacketFlow::Forward
	}
	pub async fn player_hit(proxy: &mut Proxy<'_>, player_hit: &PlayerHit) -> Result<PacketFlow> {
		let bullet_info = match projectiles!(proxy).bullets.pop(&player_hit.bullet_id) {
			Some(info) => info,
			None => {
				error!(
					owner = ?projectiles!(proxy).objects.get(&player_hit.bullet_id.owner_id),
					"Player claims that he got hit by bullet which is not visible."
				);

				return BLOCK; // Dont forward the packet then, better get DCed than die.
			}
		};

		let stats = proxy.modules.stats.get();

		let conditions = stats.conditions;
		let conditions2 = stats.conditions2;

		// we check invulnerable here since it doesnt protect from ground damage
		// while invincible is checked in take_damage() because it always applies
		if conditions.invulnerable() {
			trace!("Player hit while invulnerable.");
			return FORWARD; // ignore if invulnerable
		}

		let mut damage = if bullet_info.info.armor_piercing || conditions.armor_broken() {
			bullet_info.damage as i64
		} else {
			let mut def = stats.stats.def;
			if conditions.armored() {
				def += def / 2; // x1.5
			}

			let damage = bullet_info.damage as i64 - def;
			// a bullet must always deal at least 10% of its damage, doesnt matter the def
			let min_damage = bullet_info.damage as i64 / 10;

			damage.max(min_damage)
		};

		if conditions2.exposed() {
			damage += 20;
		}
		if conditions2.cursed() {
			damage += damage / 4; // x 1.25
		}

		// probably should force conditions until server sends those same conditions or something

		// apply any status effects (conditions) if this bullet inflicts
		// if bullet_info.info.inflicts_cursed {
		// 	proxy.modules.stats.last_tick.conditions2.set_cursed(true);
		// }
		// if bullet_info.info.inflicts_exposed {
		// 	proxy.modules.stats.last_tick.conditions2.set_exposed(true);
		// }
		// if bullet_info.info.inflicts_sick {
		// 	proxy.modules.stats.last_tick.conditions.set_sick(true);
		// }
		// if bullet_info.info.inflicts_bleeding {
		// 	proxy.modules.stats.last_tick.conditions.set_bleeding(true);
		// }
		// if bullet_info.info.inflicts_armor_broken {
		// 	proxy
		// 		.modules
		// 		.stats
		// 		.last_tick
		// 		.conditions
		// 		.set_armor_broken(true);
		// }

		trace!(?bullet_info, "Player hit with bullet");

		take_damage(proxy, damage).await
	}
}

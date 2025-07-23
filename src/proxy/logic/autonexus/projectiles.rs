use super::take_damage;
use crate::{assets::ProjectileInfo, proxy::Proxy};
use anyhow::{Result, bail};
use lru::LruCache;
use std::{collections::BTreeMap, num::NonZeroUsize};

// How many bullets to keep track of at the same time.
// If you set this too low autonexus might fail when there are too many bullets on the screen
// simultaneously and you will die
const BULLET_CACHE_SIZE: usize = 10_000;

pub struct Projectiles {
	// maps the object id of a currently visible object to it's type id
	objects: BTreeMap<u32, u16>,

	// all currently visible bullets. (bullet id, owner id) -> (damage, projectile info)
	bullets: LruCache<(u16, u32), (i16, ProjectileInfo)>,
}

impl Default for Projectiles {
	fn default() -> Self {
		Self {
			objects: BTreeMap::new(),
			bullets: LruCache::new(NonZeroUsize::new(BULLET_CACHE_SIZE).unwrap()),
		}
	}
}

pub fn add_object(proxy: &mut Proxy, object_id: u32, object_type: u16) {
	proxy
		.state
		.autonexus
		.projectiles
		.objects
		.insert(object_id, object_type);
	// server sends duplicate object ids all the time. ignore errors
}

pub fn remove_object(proxy: &mut Proxy, object_id: u32) {
	proxy.state.autonexus.projectiles.objects.remove(&object_id);
	// server sends objects to remove that werent even added all the time. ignore errors
}

pub async fn new_bullet(
	proxy: &mut Proxy,
	bullet_id: u16,
	owner_id: u32,
	bullet_type: u8,
	damage: i16,
	numshots: u8,
) -> Result<()> {
	let object_type = match proxy.state.autonexus.projectiles.objects.get(&owner_id) {
		Some(t) => *t as u32,
		// this happens all the time, server sends info about bullets that are not even in visible range
		// its safe to assume that the client ignores these too
		None => return Ok(()),
	};

	let object_bullet_types = match proxy.rotmguard.assets.objects.get(&object_type) {
		Some(t) => &t.projectiles,
		None => bail!(
			"Bullet shot by enemy ({object_type}) of which assets are not registered. Maybe your assets are outdated?"
		),
	};

	let projectile = match object_bullet_types.get(&bullet_type) {
		Some(t) => *t,
		None => bail!(
			"Bullet type shot (object {object_type}, bullet {bullet_type}) of which assets are not registered. Maybe your assets are outdated?"
		),
	};

	// create N bullets with incremental IDs where N is the number of shots
	for i in 0..numshots {
		proxy
			.state
			.autonexus
			.projectiles
			.bullets
			.put((bullet_id + i as u16, owner_id), (damage, projectile));
	}

	Ok(())
}

pub async fn player_hit(proxy: &mut Proxy, bullet_id: u16, owner_id: u32) -> Result<()> {
	let (damage, projectile) = match proxy
		.state
		.autonexus
		.projectiles
		.bullets
		.pop(&(bullet_id, owner_id))
	{
		Some(s) => s,
		None => bail!("Player claims that he got hit by bullet which is not visible."),
	};

	take_damage(proxy, damage as i64, projectile.armor_piercing).await;

	// immediatelly apply any status effects (conditions) if this bullet inflicts
	proxy.state.autonexus.ticks.for_each(|tick| {
		tick.stats.conditions2 |= projectile.inflicts_condition2;
		tick.stats.conditions |= projectile.inflicts_condition;
	});

	Ok(())
}

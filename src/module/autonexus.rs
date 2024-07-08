use derivative::Derivative;
use lru::LruCache;
use tracing::{error, trace};

use super::Module;
use crate::{
	asset_extract::{self, ProjectileInfo},
	extra_datatypes::{ObjectId, ProjectileId},
	packets::{ClientPacket, ServerPacket},
	proxy::Proxy,
};
use std::{
	collections::{BTreeMap, HashMap},
	io::Result,
	num::NonZeroUsize,
};

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Autonexus {
	pub hp: f64,

	// the id of the last tick when damage was taken
	pub tick_when_last_hit: u32,

	// all currently visible bullets. Key is (bullet id, owner id)
	// This is an LRU cache because we don't really remove bullets
	// as no information about their removal is sent in the protocol
	// so we just keep info about an X number of bullets, discarding the oldest
	// as new are added
	#[derivative(Debug = "ignore")]
	pub bullets: LruCache<ProjectileId, Bullet>,
	// maps the object id of a currently visible object to it's type
	#[derivative(Debug = "ignore")]
	pub objects: BTreeMap<ObjectId, u16>,
	// all ground tiles in this map that could deal damage. Map<(x, y) -> damage>
	#[derivative(Debug = "ignore")]
	pub hazardous_tiles: HashMap<(i16, i16), i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
	pub damage: i16,
	pub info: ProjectileInfo,
}

impl Module for Autonexus {
	fn new() -> Self {
		Autonexus {
			hp: 0.0,
			tick_when_last_hit: 0,
			bullets: LruCache::new(NonZeroUsize::new(10000).unwrap()),
			objects: BTreeMap::new(),
			hazardous_tiles: HashMap::new(),
		}
	}
	async fn client_packet(
		&mut self,
		proxy: &mut Proxy,
		packet: &mut ClientPacket,
	) -> Result<bool> {
		Ok(true)
	}
	async fn server_packet(
		&mut self,
		proxy: &mut Proxy,
		packet: &mut ServerPacket,
	) -> Result<bool> {
		match packet {
			ServerPacket::EnemyShoot(enemy_shoot) => {
				let shooter_id = enemy_shoot.bullet_id.owner_id;
				let shooter_object_type = match self.objects.get(&shooter_id) {
					Some(object_type) => *object_type as u32,
					None => {
						let dst = ((enemy_shoot.position.x - proxy.rotmguard.previous_tick.pos.x)
							.powi(2) + (enemy_shoot.position.y
							- proxy.rotmguard.previous_tick.pos.y)
							.powi(2))
						.sqrt();
						trace!(distance = dst, "EnemyShoot packet with non-visible owner");

						// this happens all the time, server sends info about bullets that are not even in visible range
						// its safe to assume that the client ignores these too
						return Ok(true);
					}
				};

				let projectiles_assets_lock = asset_extract::PROJECTILES.lock().unwrap();

				let shooter_projectile_types =
					match projectiles_assets_lock.get(&shooter_object_type) {
						Some(types) => types,
						None => {
							error!("Bullet shot by enemy of which assets are not registered. Maybe your assets are outdated?");

							return Ok(false); // i guess dont forward the packet, better get DCed than die
						}
					};

				let info = match shooter_projectile_types.get(&(enemy_shoot.bullet_type as u32)) {
					Some(info) => *info,
					None => {
						error!("Bullet type shot of which assets are not registered. Maybe your assets are outdated?");

						return Ok(false); // i guess dont forward the packet, better get DCed than die
					}
				};

				trace!(
					?info,
					"Adding bullets with ids {}..{} (owner {})",
					enemy_shoot.bullet_id.id,
					enemy_shoot.bullet_id.id + enemy_shoot.numshots as u16,
					enemy_shoot.bullet_id.owner_id.0,
				);

				// create N bullets with incremental IDs where N is the number of shots
				for i in 0..=enemy_shoot.numshots {
					self.bullets.put(
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
			}

			// This packet only adds/removes new objects, doesnt update existing ones
			ServerPacket::Update(update) => {
				// remove objects that left the visible area
				for object in &update.to_remove {
					self.objects.remove(object);
				}

				// Add new objects
				for object in &update.new_objects {
					self.objects.insert(object.1.object_id, object.0);
				}

				{
					// Add hazardous tiles if any are visible
					let hazard_tile_register = asset_extract::HAZARDOUS_GROUNDS.lock().unwrap();
					for tile in &mut update.tiles {
						let tile_type = tile.tile_type as u32;

						// we care about tiles that can do damage
						if let Some(damage) = hazard_tile_register.get(&tile_type) {
							// Add the tile
							self.hazardous_tiles.insert((tile.x, tile.y), *damage);
						}
					}
				}
			}
			ServerPacket::NewTick(new_tick) => {
				let tick_time = new_tick.tick_time as f64 / 1000.0; // in seconds
			}
			_ => {}
		}
		Ok(true)
	}
	async fn disconnect(&mut self, proxy: &mut Proxy, by_server: bool) -> Result<()> {
		Ok(())
	}
}

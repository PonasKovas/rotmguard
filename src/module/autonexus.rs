use super::{Module, ModuleInstance, PacketFlow, ProxySide, BLOCK};
use crate::{
	asset_extract::ProjectileInfo,
	config::{Config, Debuffs},
	extra_datatypes::{
		ObjectId, ObjectStatusData, PlayerConditions, ProjectileId, Stat, StatData, StatType,
		WorldPos,
	},
	logging::save_logs,
	module::FORWARD,
	packets::{AoePacket, ClientPacket, NotificationPacket, ServerPacket, ShowEffect},
	proxy::Proxy,
	util::Notification,
};
use derivative::Derivative;
use lru::LruCache;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::{
	collections::{BTreeMap, HashMap},
	io::Result,
	mem::swap,
	num::NonZero,
	sync::Arc,
};
use tracing::{debug, error, info, instrument, trace, warn};

// How many bullets to keep track of at the same time.
// If you set this too low autonexus might fail when there are too many bullets on the screen
// simultaneously and you will die
const BULLET_CACHE_SIZE: usize = 10_000;

#[derive(Debug, Clone)]
pub struct Autonexus {}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct AutonexusInst {
	// the client side simulated HP
	pub hp: f64,
	// the tick id when player last took damage
	pub tick_last_hit: u32,
	// all currently visible bullets. key is (bullet id, owner id)
	#[derivative(Debug = "ignore")]
	pub bullets: LruCache<ProjectileId, Bullet>,
	// maps the object id of a currently visible object to it's type id
	#[derivative(Debug = "ignore")]
	pub objects: BTreeMap<ObjectId, u16>,
	// all once seen ground tiles that could deal damage. Map<(x, y) -> damage>
	#[derivative(Debug = "ignore")]
	pub hazardous_tiles: HashMap<(i16, i16), i64>,
	// Buffer for collecting AOEs and then checking on NextTick packet
	pub aoes_buffer: Vec<AoePacket>,
}

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
	pub damage: i16,
	pub info: ProjectileInfo,
}

impl Module for Autonexus {
	type Instance = AutonexusInst;

	fn new() -> Self {
		Autonexus {}
	}
	fn instance(&self) -> Self::Instance {
		AutonexusInst {
			hp: 0.0,
			tick_last_hit: 0,
			bullets: LruCache::new(NonZero::new(BULLET_CACHE_SIZE).unwrap()),
			objects: BTreeMap::new(),
			hazardous_tiles: HashMap::new(),
			aoes_buffer: Vec::new(),
		}
	}
}

impl ModuleInstance for AutonexusInst {
	#[instrument(skip(proxy), fields(modules = ?proxy.modules))]
	async fn client_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		let autonexus = &mut proxy.modules.autonexus;
		match packet {
			ClientPacket::PlayerText(player_text) => {
				let text = &player_text.text;
				// `/autonexus <HP>` sets the autonexus HP threshold (can be negative if you want to die)
				if text.starts_with("/autonexus") {
					let hp = match text.split(' ').nth(1) {
						Some(h) => h,
						None => {
							Notification::new(format!(
								"/autonexus <HP>\nCurrent value: {}",
								*proxy.config.settings.autonexus_hp.lock().unwrap()
							))
							.blue()
							.send(&mut proxy.write)
							.await?;

							return BLOCK;
						}
					};
					let hp = match hp.parse::<i64>() {
						Ok(h) => h,
						Err(e) => {
							Notification::new(format!("/autonexus <HP>\nError parsing HP: {e}"))
								.red()
								.send(&mut proxy.write)
								.await?;
							error!("Error parsing /autonexus command HP: {e:?}");

							return BLOCK;
						}
					};

					*proxy.config.settings.autonexus_hp.lock().unwrap() = hp;

					Notification::new(format!("Autonexus threshold set to {hp}."))
						.green()
						.send(&mut proxy.write)
						.await?;

					return BLOCK; // dont forward this :)
				}
			}
			ClientPacket::PlayerHit(player_hit) => {
				let bullet_info = match autonexus.bullets.pop(&player_hit.bullet_id) {
					Some(info) => info,
					None => {
						error!(
							owner = ?autonexus.objects.get(&player_hit.bullet_id.owner_id),
							"Player claims that he got hit by bullet which is not visible."
						);

						return BLOCK; // Dont forward the packet then, better get DCed than die.
					}
				};

				let conditions = proxy.modules.stats.last_tick.conditions;
				let conditions2 = proxy.modules.stats.last_tick.conditions2;

				// we check invulnerable here since it doesnt protect from ground damage
				// while invincible is checked in take_damage() because it always applies
				if conditions.invulnerable() {
					trace!("Player hit while invulnerable.");
					return FORWARD; // ignore if invulnerable
				}

				let mut damage = if bullet_info.info.armor_piercing || conditions.armor_broken() {
					bullet_info.damage as i64
				} else {
					let mut def = proxy.modules.stats.last_tick.stats.def;
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

				// todo not sure if should apply conditions immediatelly or for next tick
				// if immediatelly, should uncomment
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

				return take_damage(proxy, damage).await;
			}
			ClientPacket::GroundDamage(ground_damage) => {
				let x = ground_damage.position.x as i16;
				let y = ground_damage.position.y as i16;

				let damage = match autonexus.hazardous_tiles.get(&(x, y)) {
					Some(damage) => *damage,
					None => {
						error!("Player claims to take ground damage when not standing on hazardous ground! Maybe your assets are outdated?");
						nexus(proxy).await?;

						return BLOCK;
					}
				};

				return take_damage(proxy, damage).await;
			}
			ClientPacket::Move(_move_packet) => {
				// if server hp is lower than client hp it's very bad, it means potential death
				// but not if server hp is full, which happens when your max hp reduces
				let hp_delta = autonexus.hp - proxy.modules.stats.last_tick.stats.hp as f64;
				if hp_delta > 5.0
					&& proxy.modules.stats.last_tick.stats.hp
						!= proxy.modules.stats.last_tick.stats.max_hp
				{
					error!(client_hp = autonexus.hp, "server hp lower than client hp");

					// flash the character and give notification for debugging purposes
					if *proxy.config.settings.dev_mode.lock().unwrap() {
						let intensity = ((hp_delta - 5.0).min(40.0)) / 40.0;
						let color = (120.0 * intensity) as u32;
						let color = 0x7D6666 + (color << 16);
						Notification::new(format!("positive delta {hp_delta}"))
							.color(color)
							.send(&mut proxy.write)
							.await?;

						save_logs();

						let packet = ShowEffect {
							effect_type: 18,
							target_object_id: Some(proxy.modules.general.my_object_id),
							pos1: WorldPos { x: 1.0, y: 0.0 },
							pos2: WorldPos { x: 1.0, y: 1.0 },
							color: Some(0xffffff),
							duration: Some(1.0),
							unknown: None,
						};
						proxy.write.send_client(&packet.into()).await?;
					}
				}

				// Only sync HP with the server if no shots have been taken for 10 ticks straight (2 theoretical seconds)
				// to make sure they're actually in sync.
				// OR if server hp is lower than client HP
				let should_sync = proxy.modules.general.tick_id - autonexus.tick_last_hit >= 10;
				if should_sync || autonexus.hp > proxy.modules.stats.last_tick.stats.hp as f64 {
					autonexus.hp = proxy.modules.stats.last_tick.stats.hp as f64;
				}
			}
			_ => {}
		}

		FORWARD
	}
	#[instrument(skip(proxy), fields(modules = ?proxy.modules))]
	async fn server_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		let autonexus = &mut proxy.modules.autonexus;
		match packet {
			ServerPacket::Update(update) => {
				// remove objects that left the visible area
				for object in &update.to_remove {
					autonexus.objects.remove(object);
				}

				// if myself added, set initial hp
				if let Some(me) = update
					.new_objects
					.iter()
					.find(|obj| obj.1.object_id == proxy.modules.general.my_object_id)
				{
					if let Some(my_hp) =
						me.1.stats
							.iter()
							.find(|stat| stat.stat_type == StatType::HP)
					{
						autonexus.hp = my_hp.stat.as_int() as f64;
					}
				}

				// Add new objects
				for object in &update.new_objects {
					autonexus.objects.insert(object.1.object_id, object.0);
				}

				// Add hazardous tiles if any are visible
				for tile in &update.tiles {
					let tile_type = tile.tile_type as u32;

					// we care about tiles that can do damage
					if let Some(damage) = proxy.assets.hazardous_grounds.get(&tile_type) {
						// Add the tile
						autonexus.hazardous_tiles.insert((tile.x, tile.y), *damage);
					}
				}
			}
			ServerPacket::EnemyShoot(enemy_shoot) => {
				let shooter_id = enemy_shoot.bullet_id.owner_id;
				let shooter_object_type = match autonexus.objects.get(&shooter_id) {
					Some(object_type) => *object_type as u32,
					None => {
						let dst = ((enemy_shoot.position.x - proxy.modules.stats.last_tick.pos.x)
							.powi(2) + (enemy_shoot.position.y
							- proxy.modules.stats.last_tick.pos.y)
							.powi(2))
						.sqrt();
						trace!(distance = dst, "EnemyShoot packet with non-visible owner");

						// this happens all the time, server sends info about bullets that are not even in visible range
						// its safe to assume that the client ignores these too
						return FORWARD;
					}
				};

				let shooter_projectile_types =
					match proxy.assets.projectiles.get(&shooter_object_type) {
						Some(types) => types,
						None => {
							error!("Bullet shot by enemy of which assets are not registered. Maybe your assets are outdated?");

							return BLOCK; // i guess dont forward the packet, better get DCed than die
						}
					};

				let info = match shooter_projectile_types.get(&(enemy_shoot.bullet_type as u32)) {
					Some(info) => *info,
					None => {
						error!("Bullet type shot of which assets are not registered. Maybe your assets are outdated?");

						return BLOCK; // i guess dont forward the packet, better get DCed than die
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
					autonexus.bullets.put(
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
			ServerPacket::Aoe(aoe) => {
				autonexus.aoes_buffer.push(aoe.clone());
			}
			ServerPacket::Notification(NotificationPacket::ObjectText {
				message,
				object_id,
				color: 0x00ff00, // green means heal
			}) => {
				// only interested in ourselves
				if *object_id != proxy.modules.general.my_object_id {
					return FORWARD;
				}

				// of course they add a sprinkle of invalid JSON to the protocol
				#[derive(Deserialize)]
				struct H {
					k: String,
					t: T,
				}
				#[derive(Deserialize)]
				struct T {
					amount: String,
				}

				let amount_healed = match json5::from_str::<H>(message) {
					Ok(h) => {
						if h.k != "s.plus_symbol" {
							error!("Unexpected object notification for heal. k not equal to 's.plus_symbol'");
							return FORWARD;
						}
						match h.t.amount.parse::<i64>() {
							Ok(n) => n,
							Err(e) => {
								error!("Error parsing heal notification amount: {e:?}");
								return FORWARD;
							}
						}
					}
					Err(e) => {
						error!("Error parsing object notification: {e:?}");
						return FORWARD;
					}
				};

				autonexus.hp = (autonexus.hp + amount_healed as f64)
					.min(proxy.modules.stats.last_tick.stats.max_hp as f64);

				debug!(heal_amount = amount_healed, new_hp = autonexus.hp, "Healed");
			}
			ServerPacket::NewTick(new_tick) => {
				let tick_time = new_tick.tick_time as f64 / 1000.0; // in seconds

				let mut aoes = Vec::new();
				swap(&mut autonexus.aoes_buffer, &mut aoes);

				// this remapping is so that it can be logged conveniently (which aoes hit)
				let mut aoes: Vec<(AoePacket, bool)> =
					aoes.into_iter().map(|a| (a, false)).collect();

				// invincible is checked at take_damage because it applies to everything
				// while invulnerable doesnt apply to ground damage
				if !proxy.modules.stats.last_tick.conditions.invulnerable() {
					for (aoe, affects_me) in &mut aoes {
						let distance =
							((aoe.position.x - proxy.modules.stats.last_tick.pos.x).powi(2)
								+ (aoe.position.y - proxy.modules.stats.last_tick.pos.y).powi(2))
							.sqrt();

						if distance <= aoe.radius {
							*affects_me = true;

							let conditions = proxy.modules.stats.last_tick.conditions;
							let conditions2 = proxy.modules.stats.last_tick.conditions2;

							let mut damage = if aoe.armor_piercing || conditions.armor_broken() {
								aoe.damage as i64
							} else {
								let mut def = proxy.modules.stats.last_tick.stats.def;
								if conditions.armored() {
									def += def / 2; // x1.5
								}
								let damage = aoe.damage as i64 - def;
								// a bullet must always deal at least 10% of its damage, doesnt matter the def
								let min_damage = aoe.damage as i64 / 10;

								damage.max(min_damage)
							};

							if conditions2.exposed() {
								damage += 20;
							}
							if conditions2.cursed() {
								damage = (damage as f64 * 1.25).floor() as i64;
							}

							// todo same idk if should apply conditions immediatelly or for next tick
							// if immediatelly uncomment. But i have a feeling it should be next tick
							// because otherwise if you got hit by 2 AOEs at the same time one of them
							// would get condition of other arbitrarily
							// match aoe.effect {
							// 	5 => {
							// 		proxy.modules.stats.last_tick.conditions.set_sick(true);
							// 	}
							// 	16 => {
							// 		proxy.modules.stats.last_tick.conditions.set_bleeding(true);
							// 	}
							// 	_ => {}
							// }

							if take_damage(proxy, damage).await? == PacketFlow::Block {
								return BLOCK; // dont forward if nexusing
							}
						}
					}

					if !aoes.is_empty() {
						trace!(?aoes, "AOEs");
					}
				}
				let autonexus = &mut proxy.modules.autonexus;

				// make sure our client hp is not more than max hp (in case it was reduced)
				autonexus.hp = autonexus
					.hp
					.min(proxy.modules.stats.last_tick.stats.max_hp as f64);

				// apply bleeding/healing if there are to client hp now
				if proxy.modules.stats.last_tick.conditions.bleeding() {
					let bleed_amount = 20.0 * tick_time;
					autonexus.hp -= bleed_amount;
					autonexus.hp = autonexus.hp.max(1.0); // bleeding stops at 1
					trace!(bleed_amount, "Applying bleeding");
				} else if !proxy.modules.stats.last_tick.conditions.sick() {
					// if not bleeding, nor sickened

					if proxy.modules.stats.last_tick.conditions.healing() {
						let heal_amount = 20.0 * tick_time;
						autonexus.hp += heal_amount;
						trace!(heal_amount, "Applying healing effect");
					}

					// vit regeneration
					let mut regen_amount =
						tick_time * (1.0 + 0.24 * proxy.modules.stats.last_tick.stats.vit as f64);
					if proxy.modules.stats.last_tick.conditions.in_combat() {
						regen_amount /= 2.0;
					};
					autonexus.hp += regen_amount;
					trace!(regen_amount, "VIT regeneration");

					// again make sure our client hp is not more than max hp
					autonexus.hp = autonexus
						.hp
						.min(proxy.modules.stats.last_tick.stats.max_hp as f64);
				}

				let my_status = match new_tick
					.statuses
					.iter_mut()
					.find(|s| s.object_id == proxy.modules.general.my_object_id)
				{
					Some(i) => i,
					None => {
						// no updates for myself, so add manually
						new_tick.statuses.push(ObjectStatusData {
							object_id: proxy.modules.general.my_object_id,
							position: proxy.modules.stats.last_tick.pos,
							stats: Vec::new(),
						});
						let i = new_tick.statuses.len() - 1;

						&mut new_tick.statuses[i]
					}
				};

				// Replace fame bar with client hp if developer mode
				if *proxy.config.settings.dev_mode.lock().unwrap() {
					// remove fame update if there is one
					my_status.stats.retain(|s| {
						s.stat_type != StatType::CurrentFame
							&& s.stat_type != StatType::ClassQuestFame
					});

					my_status.stats.push(StatData {
						stat_type: StatType::CurrentFame,
						stat: Stat::Int(autonexus.hp.floor() as i64),
						secondary_stat: -1,
					});
					my_status.stats.push(StatData {
						stat_type: StatType::ClassQuestFame,
						stat: Stat::Int(proxy.modules.stats.last_tick.stats.max_hp),
						secondary_stat: -1,
					});
				}
			}
			ServerPacket::Unknown {
				id: 46,
				bytes: _bytes,
			} => {
				error!("DEATH 💀"); // 🪦 願您在天使的懷抱中找到永恆的和平與安寧。安息。
				save_logs();
			}

			_ => {}
		}

		FORWARD
	}
	#[instrument(skip( proxy), fields(modules = ?proxy.modules))]
	async fn disconnect(proxy: &mut Proxy<'_>, _by: ProxySide) -> Result<()> {
		Ok(())
	}
}

async fn take_damage(proxy: &mut Proxy<'_>, damage: i64) -> Result<PacketFlow> {
	if proxy.modules.stats.last_tick.conditions.invincible() {
		trace!(damage, "Player would have taken damage but invincible.");
		return FORWARD;
	}

	proxy.modules.autonexus.tick_last_hit = proxy.modules.general.tick_id;
	proxy.modules.stats.next_tick.conditions.set_in_combat(true); // probably unnecessary bcs server will set it

	proxy.modules.autonexus.hp -= damage as f64;

	debug!(damage, "Damage taken");

	if proxy.modules.autonexus.hp <= *proxy.config.settings.autonexus_hp.lock().unwrap() as f64 {
		// AUTONEXUS ENGAGE!!!
		nexus(proxy).await?;
		return BLOCK; // dont forward!!!! !!1
	}
	if *proxy.config.settings.dev_mode.lock().unwrap() {
		Notification::new(format!("DAMAGE {}", damage))
			.color(0x888888)
			.send(&mut proxy.write)
			.await?;
	}

	FORWARD
}

async fn nexus(proxy: &mut Proxy<'_>) -> Result<()> {
	proxy.write.send_server(&ClientPacket::Escape).await?;

	warn!("Nexusing");

	Ok(())
}

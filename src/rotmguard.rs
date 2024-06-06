use crate::{
	asset_extract::{self, ProjectileInfo},
	config,
	extra_datatypes::{ObjectStatusData, Stat, StatData, StatType, WorldPos},
	packets::{
		AoePacket, ClientPacket, EnemyShoot, GotoPacket, NotificationPacket, Reconnect,
		ServerPacket, ShowEffect,
	},
	proxy::Proxy,
	read::RPRead,
	rotmguard,
};
use anyhow::{bail, Context, Result};
use derivative::Derivative;
use lru::LruCache;
use phf::phf_map;
use rand::prelude::*;
use serde::Deserialize;
use std::{
	collections::{BTreeMap, HashMap},
	hash::{DefaultHasher, Hash, Hasher},
	num::NonZero,
	time::{Duration, Instant},
};
use tracing::{debug, error, event, instrument, span, trace, warn, Level};
use util::Notification;

mod commands;
mod util;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct RotmGuard {
	// the player's object id
	my_object_id: i64,
	// the simulated HP of the player
	hp: f64,
	// all important stats of the player
	player_stats: PlayerStats,
	// all current important condition effects of the player, such as exposed, cursed, bleeding etc
	conditions: PlayerConditions,
	// the current world position of the player
	position: WorldPos,
	// the time instant when last hit was taken
	last_hit_instant: Instant,

	// shows a fake name for screenshots
	fake_name: Option<String>,

	// all currently visible bullets. key is (bullet id, owner id)
	#[derivative(Debug = "ignore")]
	bullets: LruCache<(u16, u32), Bullet>,
	// maps the object id of a currently visible object to it's type id
	#[derivative(Debug = "ignore")]
	objects: BTreeMap<i64, u16>,
	// all once seen ground tiles that could deal damage. Map<(x, y) -> damage>
	#[derivative(Debug = "ignore")]
	hazardous_tiles: HashMap<(i16, i16), i64>,

	// for packet investigation
	// saves all packets server->client or client->server respectively until the given instant
	#[derivative(Debug = "ignore")]
	record_sc_until: Option<Instant>,
	#[derivative(Debug = "ignore")]
	record_cs_until: Option<Instant>,
}

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
	pub damage: i16,
	pub info: ProjectileInfo,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerStats {
	server_hp: i64,
	max_hp: i64,
	def: i64,
	vit: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerConditions {
	cursed: bool,
	exposed: bool,
	sick: bool,
	bleeding: bool,
	healing: bool,
	armor_broken: bool,
	in_combat: bool,
}

impl RotmGuard {
	pub fn new() -> Self {
		Self {
			hp: 1.0,
			last_hit_instant: Instant::now(),
			my_object_id: 0,
			bullets: LruCache::new(NonZero::new(1000).unwrap()),
			objects: BTreeMap::new(),
			player_stats: PlayerStats {
				server_hp: 0,
				max_hp: 0,
				def: 0,
				vit: 0,
			},
			hazardous_tiles: HashMap::new(),
			conditions: PlayerConditions {
				cursed: false,
				exposed: false,
				sick: false,
				bleeding: false,
				healing: false,
				armor_broken: false,
				in_combat: false,
			},
			fake_name: config().settings.lock().unwrap().fakename.clone(),
			position: WorldPos { x: 0.0, y: 0.0 },
			record_sc_until: None,
			record_cs_until: None,
		}
	}
	// True to forward packet, false to block
	#[instrument(skip(proxy), fields(rotmguard = ?proxy.rotmguard))]
	pub async fn handle_client_packet(proxy: &mut Proxy, packet: &ClientPacket) -> Result<bool> {
		match packet {
			ClientPacket::PlayerText(player_text) => {
				return commands::command(proxy, &player_text.text).await;
			}
			ClientPacket::PlayerHit(player_hit) => {
				let bullet_info = match proxy
					.rotmguard
					.bullets
					.pop(&(player_hit.bullet_id, player_hit.owner_id))
				{
					Some(info) => info,
					None => {
						error!(
							owner = ?proxy.rotmguard.objects.get(&(player_hit.owner_id as i64)),
							"Player claims that he got hit by bullet which is not visible."
						);

						return Ok(false); // Dont forward the packet then, better get DCed than die.
					}
				};

				let mut damage =
					if bullet_info.info.armor_piercing || proxy.rotmguard.conditions.armor_broken {
						bullet_info.damage as i64
					} else {
						let mut def = proxy.rotmguard.player_stats.def;
						if proxy.rotmguard.conditions.exposed {
							def -= 20;
						}
						(bullet_info.damage as i64 - def).max(bullet_info.damage as i64 / 10)
					};

				if proxy.rotmguard.conditions.cursed {
					damage = (damage as f64 * 1.25).floor() as i64;
				}

				// instantly apply any status effects (conditions) if this bullet inflicts
				if bullet_info.info.inflicts_cursed {
					proxy.rotmguard.conditions.cursed = true;
				}
				if bullet_info.info.inflicts_exposed {
					proxy.rotmguard.conditions.exposed = true;
				}
				if bullet_info.info.inflicts_sick {
					proxy.rotmguard.conditions.sick = true;
				}
				if bullet_info.info.inflicts_bleeding {
					proxy.rotmguard.conditions.bleeding = true;
				}
				if bullet_info.info.inflicts_armor_broken {
					proxy.rotmguard.conditions.armor_broken = true;
				}

				trace!(?bullet_info, conditions = ?proxy.rotmguard.conditions, "Acquired bullet info");

				return RotmGuard::take_damage(proxy, damage).await;
			}
			ClientPacket::GroundDamage(ground_damage) => {
				let x = ground_damage.position.x as i16;
				let y = ground_damage.position.y as i16;

				let damage = match proxy.rotmguard.hazardous_tiles.get(&(x, y)) {
					Some(damage) => damage,
					None => {
						error!("Player claims to take ground damage when not standing on hazardous ground! Maybe your assets are outdated?");
						warn!("Nexusing");

						proxy.send_server(&ClientPacket::Escape).await?;
						return Ok(false);
					}
				};

				return RotmGuard::take_damage(proxy, *damage).await;
			}
			ClientPacket::Move(move_packet) => {
				if let Some(last_record) = move_packet.move_records.last() {
					proxy.rotmguard.position = last_record.1;
				}
			}
			ClientPacket::Unknown { id, bytes } => {
				if [81, 31].contains(id) {
					// skip some common spammy useless packets
					return Ok(true);
				}
				trace!("Unknown packet");
				if let Some(until) = proxy.rotmguard.record_cs_until {
					if Instant::now() < until {
						let mut hasher = DefaultHasher::new();
						until.hash(&mut hasher);

						let path = format!("recorded_cs/{}", hasher.finish());
						std::fs::create_dir_all(&path)?;

						let n = std::fs::read_dir(&path)?.count();
						std::fs::write(format!("{path}/{id}-{n}"), bytes)?;
					}
				}
			}
			_ => {}
		}

		Ok(true)
	}

	// True to forward packet, false to block
	#[instrument(skip(proxy), fields(rotmguard = ?proxy.rotmguard))]
	pub async fn handle_server_packet(proxy: &mut Proxy, packet: &ServerPacket) -> Result<bool> {
		match packet {
			ServerPacket::EnemyShoot(enemy_shoot) => {
				let shooter_id = enemy_shoot.owner_id as i64;
				let shooter_object_type = match proxy.rotmguard.objects.get(&shooter_id) {
					Some(object_type) => *object_type as u32,
					None => {
						trace!("EnemyShoot packet with non-visible owner");
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
					"Adding bullets with ids {}..{}",
					enemy_shoot.bullet_id,
					enemy_shoot.bullet_id + enemy_shoot.numshots as u16
				);

				// create N bullets with incremental IDs where N is the number of shots
				for i in 0..=enemy_shoot.numshots {
					proxy.rotmguard.bullets.put(
						(enemy_shoot.bullet_id + i as u16, enemy_shoot.owner_id),
						Bullet {
							damage: enemy_shoot.damage,
							info,
						},
					);
				}
			}
			ServerPacket::CreateSuccess(create_success) => {
				proxy.rotmguard.my_object_id = create_success.object_id as i64;
			}
			// This packet only adds/removes new objects, doesnt update existing ones
			ServerPacket::Update(update) => {
				// remove objects that left the visible area
				for object in &update.to_remove {
					proxy.rotmguard.objects.remove(object);
				}

				// Add new objects
				for object in &update.new_objects {
					// handle my stats
					if object.1.object_id == proxy.rotmguard.my_object_id {
						for stat in &object.1.stats {
							if stat.stat_type == StatType::HP {
								proxy.rotmguard.hp = stat.stat.as_int() as f64;
								proxy.rotmguard.player_stats.server_hp = stat.stat.as_int();
							} else if stat.stat_type == StatType::MaxHP {
								proxy.rotmguard.player_stats.max_hp = stat.stat.as_int();
							} else if stat.stat_type == StatType::Defense {
								proxy.rotmguard.player_stats.def = stat.stat.as_int();
							} else if stat.stat_type == StatType::Vitality {
								proxy.rotmguard.player_stats.vit = stat.stat.as_int();
							}
						}
					}

					proxy.rotmguard.objects.insert(object.1.object_id, object.0);
				}

				// Add hazardous tiles if any are visible
				let hazard_tile_register = asset_extract::HAZARDOUS_GROUNDS.lock().unwrap();
				let mut added_tiles = Vec::new(); // logging purposes
				for tile in &update.tiles {
					match hazard_tile_register.get(&(tile.tile_type as u32)) {
						Some(damage) => {
							// Add the tile
							proxy
								.rotmguard
								.hazardous_tiles
								.insert((tile.x, tile.y), *damage);
							added_tiles.push(((tile.x, tile.y), damage));
						}
						None => {} // dont care about normal tiles
					}
				}

				trace!(
					objects = ?update
						.new_objects
						.iter()
						.map(|o| (o.1.object_id, o.0))
						.collect::<Vec<_>>(),
					tiles = ?added_tiles,
					"Adding objects and hazardous tiles"
				);
			}
			// This packet updates existing objects
			ServerPacket::NewTick(new_tick) => {
				let tick_time = new_tick.tick_time as f64 / 1000.0; // in seconds

				if let Some(until) = proxy.rotmguard.record_sc_until {
					if Instant::now() >= until {
						proxy.rotmguard.record_sc_until = None;
						Notification::new(format!("Finished recording"))
							.color(0x33ff33)
							.send(proxy)
							.await?;
					}
				}
				if let Some(until) = proxy.rotmguard.record_cs_until {
					if Instant::now() >= until {
						proxy.rotmguard.record_cs_until = None;
						Notification::new(format!("Finished recording"))
							.color(0x33ff33)
							.send(proxy)
							.await?;
					}
				}

				// apply bleeding/healing if there are to client hp now
				if proxy.rotmguard.conditions.bleeding {
					proxy.rotmguard.hp -= 20.0 * tick_time;
					proxy.rotmguard.hp = proxy.rotmguard.hp.max(1.0); // bleeding stops at 1
					trace!(bleed_amount = 20.0 * tick_time, "Applying bleeding");
				} else if !proxy.rotmguard.conditions.sick {
					if proxy.rotmguard.conditions.healing {
						proxy.rotmguard.hp += 20.0 * tick_time;
						trace!(heal_amount = 20.0 * tick_time, "Applying healing effect");
					}

					// vit regeneration
					let regen_amount = if proxy.rotmguard.conditions.in_combat {
						tick_time * (0.27 * proxy.rotmguard.player_stats.vit as f64) / 2.0
					} else {
						tick_time * (0.27 * proxy.rotmguard.player_stats.vit as f64)
					};
					proxy.rotmguard.hp += regen_amount;
					proxy.rotmguard.hp = proxy
						.rotmguard
						.hp
						.min(proxy.rotmguard.player_stats.max_hp as f64);
					// cant heal more than max hp

					trace!(regen_amount, "VIT regeneration");
				}

				let my_status_i = match new_tick
					.statuses
					.iter()
					.position(|s| s.object_id == proxy.rotmguard.my_object_id)
				{
					Some(i) => i,
					None => {
						// no updates for myself, so just forward the original packet
						return Ok(true);
					}
				};

				// We clone the packet so we can mutate it and forward a modified one instead of the original
				let mut new_tick = new_tick.clone();
				let my_status = &mut new_tick.statuses[my_status_i];

				// Add fake name if set
				if let Some(n) = &proxy.rotmguard.fake_name {
					my_status.stats.push(StatData {
						stat_type: StatType::Name,
						stat: Stat::String(n.clone()),
						secondary_stat: -1,
					});
				}

				// Save the important stats and status effects
				for stat in &mut my_status.stats {
					if stat.stat_type == StatType::MaxHP {
						proxy.rotmguard.player_stats.max_hp = stat.stat.as_int();
					} else if stat.stat_type == StatType::Defense {
						proxy.rotmguard.player_stats.def = stat.stat.as_int();
					} else if stat.stat_type == StatType::Vitality {
						proxy.rotmguard.player_stats.vit = stat.stat.as_int();
					}

					if stat.stat_type == StatType::Condition {
						let bitmask = stat.stat.as_int();
						proxy.rotmguard.conditions.sick = (bitmask & 0x10) != 0;
						proxy.rotmguard.conditions.bleeding = (bitmask & 0x8000) != 0;
						proxy.rotmguard.conditions.healing = (bitmask & 0x20000) != 0;
						proxy.rotmguard.conditions.in_combat = (bitmask & 0x100000) != 0;
						proxy.rotmguard.conditions.armor_broken = (bitmask & 0x4000000) != 0;
					}
					if stat.stat_type == StatType::Condition2 {
						let bitmask = stat.stat.as_int();
						proxy.rotmguard.conditions.cursed = (bitmask & 0x40) != 0;
						proxy.rotmguard.conditions.exposed = (bitmask & 0x20000) != 0;
					}
				}

				if let Some(server_hp) =
					my_status.stats.iter().find(|s| s.stat_type == StatType::HP)
				{
					proxy.rotmguard.player_stats.server_hp = server_hp.stat.as_int();

					// if server hp is lower than client hp it's very bad, it means potential death
					// but not if server hp is full, which happens when your max hp reduces
					if (proxy.rotmguard.hp - proxy.rotmguard.player_stats.server_hp as f64) > 10.0
						&& proxy.rotmguard.player_stats.server_hp
							!= proxy.rotmguard.player_stats.max_hp
					{
						error!("server hp lower than client hp");
						// flash the character and give notification for debugging purposes
						if config().settings.lock().unwrap().dev_mode {
							Notification::new(format!(
								"positive delta {}",
								proxy.rotmguard.hp - proxy.rotmguard.player_stats.server_hp as f64
							))
							.color(0xff3333)
							.send(proxy)
							.await?;

							let packet = ShowEffect {
								effect_type: 18,
								target_object_id: Some(proxy.rotmguard.my_object_id),
								pos1: WorldPos { x: 1.0, y: 0.0 },
								pos2: WorldPos { x: 1.0, y: 1.0 },
								color: Some(0xffffff),
								duration: Some(1.0),
								unknown: None,
							};
							proxy.send_client(&packet.into()).await?;
						}
					}

					// Only sync HP with the server if no shots have been taken for 1 second straight
					// to make sure they're actually in sync.
					// OR if server hp is lower than client HP
					if proxy.rotmguard.last_hit_instant.elapsed().as_secs_f32() > 1.0
						|| proxy.rotmguard.hp > server_hp.stat.as_int() as f64
					{
						proxy.rotmguard.hp = server_hp.stat.as_int() as f64;
					}

					// Replace fame bar with client hp if developer mode
					if config().settings.lock().unwrap().dev_mode {
						// remove fame updates if there are
						my_status.stats.retain(|s| {
							s.stat_type != StatType::CurrentFame
								&& s.stat_type != StatType::ClassQuestFame
						});

						my_status.stats.push(StatData {
							stat_type: StatType::CurrentFame,
							stat: Stat::Int(proxy.rotmguard.hp.floor() as i64),
							secondary_stat: -1,
						});
						my_status.stats.push(StatData {
							stat_type: StatType::ClassQuestFame,
							stat: Stat::Int(proxy.rotmguard.player_stats.max_hp),
							secondary_stat: -1,
						});
					}
				}

				proxy.send_client(&new_tick.into()).await?;

				return Ok(false);
			}
			ServerPacket::Notification(notification) => {
				if let NotificationPacket::ObjectText {
					message,
					object_id,
					color: 0x00ff00, // green means heal
				} = notification
				{
					// only interested in ourselves
					if *object_id as i64 != proxy.rotmguard.my_object_id {
						return Ok(true);
					}

					// of course they add a sprinkle of JSON to the protocol
					// and of course its invalid JSON too (trailing commas)
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
						Ok(h) => match i64::from_str_radix(&h.t.amount, 10) {
							Ok(n) => n,
							Err(e) => {
								error!("Error parsing heal notification amount: {e:?}");
								return Ok(true);
							}
						},
						Err(e) => {
							error!("Error parsing object notification: {e:?}");
							return Ok(true);
						}
					};

					proxy.rotmguard.hp = (proxy.rotmguard.hp + amount_healed as f64)
						.min(proxy.rotmguard.player_stats.max_hp as f64);

					debug!(
						heal_amount = amount_healed,
						hp_left = proxy.rotmguard.hp,
						"Healed"
					);
				}
			}
			ServerPacket::Aoe(aoe) => {
				// first check if this AOE will affect us
				let my_pos = proxy.rotmguard.position;
				let aoe_pos = aoe.position;

				let distance =
					((my_pos.x - aoe_pos.x).powi(2) + (my_pos.y - aoe_pos.y).powi(2)).sqrt();

				trace!(
					player_in_radius = distance <= aoe.radius,
					player_pos = ?my_pos,
					"AOE"
				);

				if distance <= aoe.radius {
					let mut damage =
						if aoe.armor_piercing || proxy.rotmguard.conditions.armor_broken {
							aoe.damage as i64
						} else {
							let mut def = proxy.rotmguard.player_stats.def;
							if proxy.rotmguard.conditions.exposed {
								def -= 20;
							}
							(aoe.damage as i64 - def).max(aoe.damage as i64 / 10)
						};

					if proxy.rotmguard.conditions.cursed {
						damage = (damage as f64 * 1.25).floor() as i64;
					}

					match aoe.effect {
						5 => {
							proxy.rotmguard.conditions.sick = true;
						}
						16 => {
							proxy.rotmguard.conditions.bleeding = true;
						}
						_ => {}
					}

					return RotmGuard::take_damage(proxy, damage).await;
				}
			}
			ServerPacket::Unknown {
				id: 46,
				bytes: _bytes,
			} => {
				error!("DEATH");
			}
			ServerPacket::Unknown { id, bytes } => {
				if [8].contains(id) {
					// skip some common spammy useless packets
					return Ok(true);
				}
				if let Some(until) = proxy.rotmguard.record_sc_until {
					if Instant::now() < until {
						let mut hasher = DefaultHasher::new();
						until.hash(&mut hasher);

						let path = format!("recorded_sc/{}", hasher.finish());
						std::fs::create_dir_all(&path)?;

						let n = std::fs::read_dir(&path)?.count();
						std::fs::write(format!("{path}/{id}-{n}"), bytes)?;
					}
				}
			}
			_ => {}
		}

		Ok(true)
	}
	// Modifies the client hp and nexuses if necessary
	// This does not consider defense or any status effects.
	pub async fn take_damage(proxy: &mut Proxy, damage: i64) -> Result<bool> {
		proxy.rotmguard.last_hit_instant = Instant::now();
		proxy.rotmguard.conditions.in_combat = true;

		proxy.rotmguard.hp -= damage as f64;

		debug!(damage = damage, "Damage taken");

		if proxy.rotmguard.hp <= config().settings.lock().unwrap().autonexus_hp as f64 {
			// AUTONEXUS ENGAGE!!!
			proxy.send_server(&ClientPacket::Escape).await?;
			warn!("Nexusing");
			return Ok(false); // dont forward!!
		}
		if config().settings.lock().unwrap().dev_mode {
			Notification::new(format!("DAMAGE {}", damage))
				.color(0x888888)
				.send(proxy)
				.await?;
		}

		Ok(true)
	}
}

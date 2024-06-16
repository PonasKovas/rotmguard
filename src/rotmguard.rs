use crate::{
	asset_extract::{self, ProjectileInfo},
	config,
	extra_datatypes::{Stat, StatData, StatType, WorldPos},
	logging::save_logs,
	packets::{AoePacket, ClientPacket, NotificationPacket, ServerPacket, ShowEffect},
	proxy::Proxy,
};
use anyhow::Result;
use commands::{RECORD_CS_UNTIL, RECORD_SC_UNTIL};
use derivative::Derivative;
use lru::LruCache;
use serde::Deserialize;
use std::{
	collections::{BTreeMap, HashMap},
	hash::{DefaultHasher, Hash, Hasher},
	mem::swap,
	num::NonZero,
	time::Instant,
};
use tracing::{debug, error, info, instrument, trace, warn};
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
	// position of the player
	position: WorldPos,
	// whether the player took damage in this client-side tick
	hit_this_tick: bool,
	// the id of the last clientside tick when damage was taken
	tick_when_last_hit: u32,

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

	// basically we add aoes to this buffer when we receive them, and then move all of them to the corresponding tick vector once NewTick
	// is received, and then check them at next Move packet
	last_tick_aoes_buffer: Vec<AoePacket>,
	aoes: BTreeMap<u32, Vec<AoePacket>>,
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
	armored: bool,
	armor_broken: bool,
	in_combat: bool,
}

impl RotmGuard {
	pub fn new() -> Self {
		Self {
			hp: 1.0,
			my_object_id: 0,
			hit_this_tick: false,
			tick_when_last_hit: 0,
			position: WorldPos { x: 0.0, y: 0.0 },
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
				armored: false,
				armor_broken: false,
				in_combat: false,
			},
			fake_name: config().settings.lock().unwrap().fakename.clone(),

			last_tick_aoes_buffer: Vec::new(),
			aoes: BTreeMap::new(),
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
						let def = proxy.rotmguard.calculate_def();
						let damage = bullet_info.damage as i64 - def;
						// a bullet must always deal at least 10% of its damage, doesnt matter the def
						let min_damage = bullet_info.damage as i64 / 10;

						damage.max(min_damage)
					};

				if proxy.rotmguard.conditions.exposed {
					damage += 20;
				}
				if proxy.rotmguard.conditions.cursed {
					damage += damage / 4; // x 1.25
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
						RotmGuard::nexus(proxy).await?;

						return Ok(false);
					}
				};

				return RotmGuard::take_damage(proxy, *damage).await;
			}
			ClientPacket::Move(move_packet) => {
				// this is basically client-side version of the tick

				if let Some(pos) = move_packet.move_records.last() {
					proxy.rotmguard.position = pos.1;
				}

				if proxy.rotmguard.hit_this_tick {
					proxy.rotmguard.tick_when_last_hit = move_packet.tick_id;
					proxy.rotmguard.hit_this_tick = false;
				}

				let aoes = match proxy.rotmguard.aoes.remove(&move_packet.tick_id) {
					Some(a) => a,
					None => {
						error!(
							"Player sending Move packet for tick id which doesn't exist: {}",
							move_packet.tick_id
						);
						return Ok(false);
					}
				};

				for aoe in aoes {
					// first check if this AOE will affect us

					let mut affects_me = false;

					if move_packet.move_records.len() > 0 {
						// check all positions that happened in the last tick
						for pos in &move_packet.move_records {
							let distance = ((aoe.position.x - pos.1.x).powi(2)
								+ (aoe.position.y - pos.1.y).powi(2))
							.sqrt();

							if distance <= aoe.radius {
								affects_me = true;
								break;
							}
						}
					} else {
						// check the last known position
						let distance = ((aoe.position.x - proxy.rotmguard.position.x).powi(2)
							+ (aoe.position.y - proxy.rotmguard.position.y).powi(2))
						.sqrt();
						if distance <= aoe.radius {
							affects_me = true;
						}
					}

					trace!(affects_me, ?aoe, "AOE");

					if affects_me {
						let mut damage =
							if aoe.armor_piercing || proxy.rotmguard.conditions.armor_broken {
								aoe.damage as i64
							} else {
								let def = proxy.rotmguard.calculate_def();
								let damage = aoe.damage as i64 - def;
								// a bullet must always deal at least 10% of its damage, doesnt matter the def
								let min_damage = aoe.damage as i64 / 10;

								damage.max(min_damage)
							};

						if proxy.rotmguard.conditions.exposed {
							damage += 20;
						}
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

						if RotmGuard::take_damage(proxy, damage).await? == false {
							return Ok(false);
						}
					}
				}
			}
			ClientPacket::Unknown { id, bytes } => {
				if [81, 31].contains(id) {
					// skip some common spammy useless packets
					return Ok(true);
				}
				trace!("Unknown packet");
				if let Some(until) = *RECORD_CS_UNTIL.lock().unwrap() {
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
						let dst = ((enemy_shoot.position.x - proxy.rotmguard.position.x).powi(2)
							+ (enemy_shoot.position.y - proxy.rotmguard.position.y).powi(2))
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
					// we only care about tiles that can do damage
					if let Some(damage) = hazard_tile_register.get(&(tile.tile_type as u32)) {
						// Add the tile
						proxy
							.rotmguard
							.hazardous_tiles
							.insert((tile.x, tile.y), *damage);
						added_tiles.push(((tile.x, tile.y), damage));
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

				proxy.rotmguard.aoes.insert(new_tick.tick_id, Vec::new());
				swap(
					proxy.rotmguard.aoes.get_mut(&new_tick.tick_id).unwrap(),
					&mut proxy.rotmguard.last_tick_aoes_buffer,
				);

				let rec_sc_until = *RECORD_SC_UNTIL.lock().unwrap();
				if let Some(until) = rec_sc_until {
					if Instant::now() >= until {
						*RECORD_SC_UNTIL.lock().unwrap() = None;
						Notification::new("Finished recording".to_owned())
							.color(0x33ff33)
							.send(proxy)
							.await?;
					}
				}
				let rec_cs_until = *RECORD_CS_UNTIL.lock().unwrap();
				if let Some(until) = rec_cs_until {
					if Instant::now() >= until {
						*RECORD_CS_UNTIL.lock().unwrap() = None;
						Notification::new("Finished recording".to_owned())
							.color(0x33ff33)
							.send(proxy)
							.await?;
					}
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
					// remove guild name also
					my_status.stats.push(StatData {
						stat_type: StatType::GuildName,
						stat: Stat::String("".to_owned()),
						secondary_stat: -1,
					});
				}

				// apply bleeding/healing if there are to client hp now
				if proxy.rotmguard.conditions.bleeding {
					proxy.rotmguard.hp -= 20.0 * tick_time;
					proxy.rotmguard.hp = proxy.rotmguard.hp.max(1.0); // bleeding stops at 1
					trace!(bleed_amount = 20.0 * tick_time, "Applying bleeding");
				} else if !proxy.rotmguard.conditions.sick
					&& proxy.rotmguard.player_stats.server_hp != proxy.rotmguard.player_stats.max_hp
				{
					// if server hp full dont regenerate yet
					if proxy.rotmguard.conditions.healing {
						proxy.rotmguard.hp += 20.0 * tick_time;
						trace!(heal_amount = 20.0 * tick_time, "Applying healing effect");
					}

					// vit regeneration
					let regen_amount = if proxy.rotmguard.conditions.in_combat {
						tick_time * (1.0 + 0.24 * proxy.rotmguard.player_stats.vit as f64) / 2.0
					} else {
						tick_time * (1.0 + 0.24 * proxy.rotmguard.player_stats.vit as f64)
					};
					proxy.rotmguard.hp += regen_amount;

					trace!(regen_amount, "VIT regeneration");
				}

				// Save the important stats and status effects
				for stat in &mut my_status.stats {
					match stat.stat_type {
						StatType::MaxHP => {
							proxy.rotmguard.player_stats.max_hp = stat.stat.as_int();
						}
						StatType::Defense => {
							proxy.rotmguard.player_stats.def = stat.stat.as_int();
						}
						StatType::Vitality => {
							proxy.rotmguard.player_stats.vit = stat.stat.as_int();
						}
						StatType::HP => {
							proxy.rotmguard.player_stats.server_hp = stat.stat.as_int();
						}
						StatType::Condition => {
							let mut bitmask = stat.stat.as_int();
							proxy.rotmguard.conditions.sick = (bitmask & 0x10) != 0;
							proxy.rotmguard.conditions.bleeding = (bitmask & 0x8000) != 0;
							proxy.rotmguard.conditions.healing = (bitmask & 0x20000) != 0;
							proxy.rotmguard.conditions.in_combat = (bitmask & 0x100000) != 0;
							proxy.rotmguard.conditions.armored = (bitmask & 0x2000000) != 0;
							proxy.rotmguard.conditions.armor_broken = (bitmask & 0x4000000) != 0;

							// remove client-side debuffs
							let cfg_debuffs = &config().settings.lock().unwrap().debuffs;
							if cfg_debuffs.blind {
								bitmask = bitmask & !0x80;
							}
							if cfg_debuffs.hallucinating {
								bitmask = bitmask & !0x100;
							}
							if cfg_debuffs.drunk {
								bitmask = bitmask & !0x200;
							}
							if cfg_debuffs.confused {
								bitmask = bitmask & !0x400;
							}
							if cfg_debuffs.unstable {
								bitmask = bitmask & !0x20000000;
							}
							if cfg_debuffs.darkness {
								bitmask = bitmask & !0x40000000;
							}
							stat.stat = Stat::Int(bitmask);
						}
						StatType::Condition2 => {
							let bitmask = stat.stat.as_int();
							proxy.rotmguard.conditions.cursed = (bitmask & 0x40) != 0;
							proxy.rotmguard.conditions.exposed = (bitmask & 0x20000) != 0;
						}
						_ => {}
					}
				}

				// make sure our hp is not above max hp after healing or max hp decrease
				proxy.rotmguard.hp = proxy
					.rotmguard
					.hp
					.min(proxy.rotmguard.player_stats.max_hp as f64);

				// if server hp is lower than client hp it's very bad, it means potential death
				// but not if server hp is full, which happens when your max hp reduces
				if (proxy.rotmguard.hp - proxy.rotmguard.player_stats.server_hp as f64) > 5.0
					&& proxy.rotmguard.player_stats.server_hp != proxy.rotmguard.player_stats.max_hp
				{
					error!(
						server_hp = proxy.rotmguard.player_stats.server_hp,
						"server hp lower than client hp"
					);
					// flash the character and give notification for debugging purposes
					if config().settings.lock().unwrap().dev_mode {
						Notification::new(format!(
							"positive delta {}",
							proxy.rotmguard.hp - proxy.rotmguard.player_stats.server_hp as f64
						))
						.color(0xff3333)
						.send(proxy)
						.await?;

						save_logs();

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

				// Only sync HP with the server if no shots have been taken for 10 ticks straight (2 theoretical seconds)
				// to make sure they're actually in sync.
				// OR if server hp is lower than client HP
				let should_sync = (new_tick.tick_id - proxy.rotmguard.tick_when_last_hit >= 10)
					&& !proxy.rotmguard.hit_this_tick;
				if should_sync || proxy.rotmguard.hp > proxy.rotmguard.player_stats.server_hp as f64
				{
					proxy.rotmguard.hp = proxy.rotmguard.player_stats.server_hp as f64;
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

				proxy.send_client(&new_tick.into()).await?;

				return Ok(false);
			}
			ServerPacket::Notification(NotificationPacket::ObjectText {
				message,
				object_id,
				color: 0x00ff00, // green means heal
			}) => {
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
					Ok(h) => {
						if h.k != "s.plus_symbol" {
							error!("Unexpected object notification for heal. k not equal to 's.plus_symbol'");
							return Ok(true);
						}
						match h.t.amount.parse::<i64>() {
							Ok(n) => n,
							Err(e) => {
								error!("Error parsing heal notification amount: {e:?}");
								return Ok(true);
							}
						}
					}
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
			ServerPacket::Aoe(aoe) => {
				proxy.rotmguard.last_tick_aoes_buffer.push(aoe.clone());
			}
			ServerPacket::Text(text) => {
				// if chat message is from me and fake name set, replace the name
				if let Some(fake_name) = &proxy.rotmguard.fake_name {
					if text.object_id as i64 == proxy.rotmguard.my_object_id {
						let mut text = text.clone();
						text.name = fake_name.clone();
						proxy.send_client(&text.into()).await?;
						return Ok(false);
					}
				}
			}
			ServerPacket::Unknown {
				id: 46,
				bytes: _bytes,
			} => {
				error!("DEATH 💀"); // 🪦
				save_logs();
			}
			ServerPacket::Unknown { id, bytes } => {
				if [8].contains(id) {
					// skip some common spammy useless packets
					return Ok(true);
				}
				if let Some(until) = *RECORD_SC_UNTIL.lock().unwrap() {
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
		proxy.rotmguard.hit_this_tick = true;
		proxy.rotmguard.conditions.in_combat = true;

		proxy.rotmguard.hp -= damage as f64;

		debug!(damage = damage, "Damage taken");

		if proxy.rotmguard.hp <= config().settings.lock().unwrap().autonexus_hp as f64 {
			// AUTONEXUS ENGAGE!!!
			RotmGuard::nexus(proxy).await?;
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
	/// Nexuses
	pub async fn nexus(proxy: &mut Proxy) -> Result<()> {
		proxy.send_server(&ClientPacket::Escape).await?;

		warn!("Nexusing");

		Ok(())
	}
	fn calculate_def(&self) -> i64 {
		let mut def = self.player_stats.def;
		if self.conditions.armored {
			def += def / 2; // x 1.5
		}

		def
	}
}

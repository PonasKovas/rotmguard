use super::{Module, ModuleInstance, PacketFlow, ProxySide, BLOCK};
use crate::{
	asset_extract::ProjectileInfo,
	config::{Config, Debuffs},
	extra_datatypes::{
		ObjectId, ObjectStatusData, PlayerConditions, ProjectileId, Stat, StatData, StatType,
		WorldPos,
	},
	gen_this_macro,
	logging::save_logs,
	module::FORWARD,
	packets::{
		AoePacket, ClientPacket, NotificationPacket, NotificationType, ServerPacket, ShowEffect,
	},
	proxy::Proxy,
	util::Notification,
};
use aoes::AOEs;
use derivative::Derivative;
use ground::Ground;
use heals::Heals;
use lru::LruCache;
use passive::Passive;
use projectiles::Projectiles;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::{
	collections::{BTreeMap, HashMap, VecDeque},
	io::Result,
	mem::swap,
	num::NonZero,
	sync::Arc,
};
use tracing::{debug, error, info, instrument, trace, warn};

gen_this_macro! {autonexus}

mod aoes;
mod ground;
mod heals;
mod passive;
mod projectiles;

#[derive(Debug, Clone)]
pub struct Autonexus {}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct AutonexusInst {
	// the client side simulated HP
	pub hp: f64,
	// the tick id when player last took damage
	pub tick_last_hit: u32,
	// for handling ground damage such as lava
	#[derivative(Debug = "ignore")]
	pub ground: Ground,
	// for handling projectiles and their damage
	#[derivative(Debug = "ignore")]
	pub projectiles: Projectiles,
	// for handling AOEs - explosions
	#[derivative(Debug = "ignore")]
	pub aoes: AOEs,
	// for handling passive effects such as VIT regeneration, bleeding/healing effects
	#[derivative(Debug = "ignore")]
	pub passive: Passive,
	// for heals, from pet, priest, etc
	#[derivative(Debug = "ignore")]
	pub heals: Heals,
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
			ground: Ground::new(),
			projectiles: Projectiles::new(),
			aoes: AOEs::new(),
			passive: Passive::new(),
			heals: Heals::new(),
		}
	}
}

impl ModuleInstance for AutonexusInst {
	async fn client_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
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

					return BLOCK; // dont forward this ;)
				}

				FORWARD
			}
			ClientPacket::PlayerHit(player_hit) => Projectiles::player_hit(proxy, player_hit).await,
			ClientPacket::GroundDamage(ground_damage) => {
				Ground::ground_damage(proxy, ground_damage).await
			}
			ClientPacket::Move(_move_packet) => AOEs::check_aoes(proxy).await,
			_ => FORWARD,
		}
	}
	async fn server_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		match packet {
			ServerPacket::Update(update) => {
				// // if myself added, set initial hp
				// if let Some(me) = update
				// 	.new_objects
				// 	.iter()
				// 	.find(|obj| obj.1.object_id == proxy.modules.general.my_object_id)
				// {
				// 	if let Some(my_hp) =
				// 		me.1.stats
				// 			.iter()
				// 			.find(|stat| stat.stat_type == StatType::HP)
				// 	{
				// 		autonexus.hp = my_hp.stat.as_int() as f64;
				// 	}
				// }

				Projectiles::add_remove_objects(proxy, update);

				Ground::add_tiles(proxy, update);
			}
			ServerPacket::EnemyShoot(enemy_shoot) => {
				Projectiles::add_bullet(proxy, enemy_shoot);
			}
			ServerPacket::Aoe(aoe) => {
				AOEs::add_aoe(proxy, aoe);
			}
			ServerPacket::Notification(NotificationPacket {
				extra: _,
				notification:
					NotificationType::ObjectText {
						message,
						object_id,
						color: 0x00ff00, // green means heal
					},
			}) => {
				// only interested in ourselves
				if *object_id == proxy.modules.general.my_object_id {
					Heals::heal(proxy, message)
				}
			}
			ServerPacket::NewTick(new_tick) => {
				AOEs::flush(proxy);

				let tick_time = new_tick.tick_time as f64 / 1000.0; // in seconds
				Passive::apply_passive(proxy, tick_time);

				let server_hp = proxy.modules.stats.get_newest().stats.hp;

				// Sync HP with the server if no shots have been taken for 10 ticks straight (2 seconds)
				// to make sure they're actually in sync.
				// OR if server hp is lower than client HP
				let no_shots_taken =
					proxy.modules.general.client_tick_id - autonexus!(proxy).tick_last_hit >= 10;
				if no_shots_taken || autonexus!(proxy).hp > server_hp as f64 {
					autonexus!(proxy).hp = server_hp as f64;
				}

				// if server hp is lower than client hp it's very bad, it means potential death
				// but not if server hp is full, which happens when your max hp reduces
				let hp_delta = autonexus!(proxy).hp - server_hp as f64;
				if hp_delta > 1.0 && server_hp != proxy.modules.stats.get_newest().stats.max_hp {
					error!(
						client_hp = autonexus!(proxy).hp,
						"server hp lower than client hp"
					);

					// flash the character and give notification for debugging purposes
					if *proxy.config.settings.dev_mode.lock().unwrap() {
						let intensity = ((hp_delta - 1.0).min(40.0)) / 40.0;
						let color = (125.0 + 120.0 * intensity) as u32;
						let color = 0x006666 | (color << 16);
						Notification::new(format!("pdelta {hp_delta}"))
							.color(color)
							.send(&mut proxy.write)
							.await?;
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

						save_logs();
					}
				}

				// Replace fame bar with client hp if developer mode
				if *proxy.config.settings.dev_mode.lock().unwrap() {
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
								position: proxy.modules.stats.pos,
								stats: Vec::new(),
							});
							let i = new_tick.statuses.len() - 1;

							&mut new_tick.statuses[i]
						}
					};

					// remove fame update if there is one
					my_status.stats.retain(|s| {
						s.stat_type != StatType::CurrentFame
							&& s.stat_type != StatType::ClassQuestFame
					});

					my_status.stats.push(StatData {
						stat_type: StatType::CurrentFame,
						stat: Stat::Int(autonexus!(proxy).hp.floor() as i64),
						secondary_stat: -1,
					});
					my_status.stats.push(StatData {
						stat_type: StatType::ClassQuestFame,
						stat: Stat::Int(proxy.modules.stats.get_newest().stats.max_hp),
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
}

// Takes given damage. Does not calculate defense or any status effects except for invincible
// Returns BLOCK if nexused
async fn take_damage(proxy: &mut Proxy<'_>, damage: i64) -> Result<PacketFlow> {
	if proxy.modules.stats.get().conditions.invincible() {
		trace!(damage, "Player would have taken damage but invincible.");
		return FORWARD;
	}

	proxy.modules.autonexus.tick_last_hit = proxy.modules.general.client_tick_id;

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

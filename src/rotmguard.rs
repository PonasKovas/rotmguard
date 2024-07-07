use crate::{
	extra_datatypes::{
		ObjectId, PlayerConditions, PlayerConditions2, StatType,
		WorldPos,
	},
	module::Module,
	packets::{AoePacket, ClientPacket, ServerPacket},
	proxy::Proxy,
};
use anyhow::Result;
use std::sync::Arc;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct RotmGuard {
	// the player's object id
	pub my_object_id: ObjectId,
	pub previous_tick: Tick,
	pub current_tick: Tick,
}

#[derive(Debug, Clone)]
pub struct Tick {
	// the tick id
	pub id: u32,
	// all important stats of the player
	pub stats: PlayerStats,
	// all current important condition effects of the player, such as exposed, cursed, bleeding etc
	pub conditions: PlayerConditions,
	pub conditions2: PlayerConditions2,
	// position of the player
	pub pos: WorldPos,
	pub aoes: Vec<AoePacket>,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerStats {
	// this is server side HP!
	pub hp: i64,
	pub max_hp: i64,
	pub def: i64,
	pub vit: i64,
	pub spd: i64,
}

impl RotmGuard {
	pub fn new() -> Self {
		Self {
			my_object_id: ObjectId(0),
			previous_tick: Tick::empty(),
			current_tick: Tick::empty(),
		}
	}
	// True to forward packet, false to block
	#[instrument(skip(proxy), fields(rotmguard = ?proxy.rotmguard))]
	pub async fn handle_client_packet(
		proxy: &mut Proxy,
		packet: &mut ClientPacket,
	) -> Result<bool> {
		if let ClientPacket::Move(move_packet) = &packet {
			// this is basically client acknowledging a tick.
			// so we can start preparing the next one

			proxy.pause_server_read = false; // allow reading again
								 // and essentially move on to the next tick

			proxy.rotmguard.current_tick.pos = match move_packet.move_records.last() {
				Some(record) => record.1,
				None => proxy.rotmguard.previous_tick.pos,
			};
		}
		for module in &mut *Arc::clone(&proxy.modules).lock().await {
			if !module.client_packet(proxy, packet).await? {
				return Ok(false);
			}
		}

		Ok(true)
	}

	// True to forward packet, false to block
	#[instrument(skip(proxy), fields(rotmguard = ?proxy.rotmguard))]
	pub async fn handle_server_packet(
		proxy: &mut Proxy,
		packet: &mut ServerPacket,
	) -> Result<bool> {
		match packet {
			ServerPacket::CreateSuccess(create_success) => {
				proxy.rotmguard.my_object_id = create_success.object_id;
			}
			// This packet only adds/removes new objects, doesnt update existing ones
			ServerPacket::Update(update) => {
				// handle my stats
				if let Some(me) = update
					.new_objects
					.iter()
					.find(|obj| obj.1.object_id == proxy.rotmguard.my_object_id)
				{
					for stat in &me.1.stats {
						let s = {
							let stats = &mut proxy.rotmguard.current_tick.stats;
							match stat.stat_type {
								StatType::HP => &mut stats.hp,
								StatType::MaxHP => &mut stats.max_hp,
								StatType::Defense => &mut stats.def,
								StatType::Vitality => &mut stats.vit,
								StatType::Speed => &mut stats.spd,
								_ => continue,
							}
						};
						*s = stat.stat.as_int();
					}
				}
			}
			// This packet updates existing objects
			ServerPacket::NewTick(new_tick) => {
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

				let my_status = &mut new_tick.statuses[my_status_i];

				// Save the important stats and status effects
				for stat in &mut my_status.stats {
					match stat.stat_type {
						StatType::MaxHP => {
							proxy.rotmguard.current_tick.stats.max_hp = stat.stat.as_int();
						}
						StatType::Defense => {
							proxy.rotmguard.current_tick.stats.def = stat.stat.as_int();
						}
						StatType::Vitality => {
							proxy.rotmguard.current_tick.stats.vit = stat.stat.as_int();
						}
						StatType::HP => {
							proxy.rotmguard.current_tick.stats.hp = stat.stat.as_int();
						}
						StatType::Speed => {
							proxy.rotmguard.current_tick.stats.spd = stat.stat.as_int();
						}
						StatType::Condition => {
							proxy.rotmguard.current_tick.conditions = PlayerConditions {
								bitmask: stat.stat.as_int() as u64,
							};
						}
						StatType::Condition2 => {
							proxy.rotmguard.current_tick.conditions2 = PlayerConditions2 {
								bitmask: stat.stat.as_int() as u64,
							};
						}
						_ => {}
					}
				}

				proxy.rotmguard.previous_tick = proxy.rotmguard.current_tick.clone();
				proxy.pause_server_read = true; // pause reading further from server until client acknowledges this tick
			}
			ServerPacket::Aoe(aoe) => {
				proxy.rotmguard.current_tick.aoes.push(aoe.clone());
			}
			_ => {}
		}

		for module in &mut *Arc::clone(&proxy.modules).lock().await {
			if !module.server_packet(proxy, packet).await? {
				return Ok(false);
			}
		}

		Ok(true)
	}
}

impl Tick {
	fn empty() -> Self {
		Self {
			id: 0,
			stats: PlayerStats {
				hp: 0,
				max_hp: 0,
				def: 0,
				vit: 0,
				spd: 0,
			},
			conditions: PlayerConditions { bitmask: 0 },
			conditions2: PlayerConditions2 { bitmask: 0 },
			pos: WorldPos { x: 0.0, y: 0.0 },
			aoes: Vec::new(),
		}
	}
}

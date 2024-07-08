use super::{Module, ModuleInstance};
use crate::{
	extra_datatypes::{PlayerConditions, PlayerConditions2, StatType, WorldPos},
	packets::{ClientPacket, ServerPacket},
	proxy::Proxy,
};
use std::io::Result;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct Stats {}

#[derive(Debug, Clone)]
pub struct StatsInst {
	// always read stats from here, the other one is not complete (its used to build this one)
	last_tick: TickStats,
	next_tick: TickStats,
}

#[derive(Debug, Clone)]
pub struct TickStats {
	// all important stats of the player
	pub stats: PlayerStats,
	// all current important condition effects of the player, such as exposed, cursed, bleeding etc
	pub conditions: PlayerConditions,
	pub conditions2: PlayerConditions2,
	// position of the player
	pub pos: WorldPos,
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

impl Module for Stats {
	type Instance = StatsInst;

	fn new() -> Self {
		Stats {}
	}

	fn instance(&self) -> Self::Instance {
		StatsInst {
			last_tick: TickStats::empty(),
			next_tick: TickStats::empty(),
		}
	}
}

impl ModuleInstance for StatsInst {
	#[instrument(skip(proxy), fields(modules = ?proxy.modules))]
	async fn client_packet(proxy: &mut Proxy, packet: &mut ClientPacket) -> Result<bool> {
		match packet {
			ClientPacket::Move(move_packet) => {
				// Update player position
				proxy.modules.stats.next_tick.pos = match move_packet.move_records.last() {
					Some(record) => record.1,
					None => proxy.modules.stats.last_tick.pos,
				};
			}
			_ => {}
		}

		Ok(true)
	}
	#[instrument(skip(proxy), fields(modules = ?proxy.modules))]
	async fn server_packet(proxy: &mut Proxy, packet: &mut ServerPacket) -> Result<bool> {
		match packet {
			// This packet only adds/removes new objects, doesnt update existing ones
			ServerPacket::Update(update) => {
				// only interested in my own stats
				let me = match update
					.new_objects
					.iter()
					.find(|obj| obj.1.object_id == proxy.modules.general.my_object_id)
				{
					Some(me) => me,
					None => return Ok(true),
				};

				for stat in &me.1.stats {
					let s = {
						let stats = &mut proxy.modules.stats.next_tick.stats;
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
			// This packet updates existing objects
			ServerPacket::NewTick(new_tick) => {
				let my_status_i = match new_tick
					.statuses
					.iter()
					.position(|s| s.object_id == proxy.modules.general.my_object_id)
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
							proxy.modules.stats.next_tick.stats.max_hp = stat.stat.as_int();
						}
						StatType::Defense => {
							proxy.modules.stats.next_tick.stats.def = stat.stat.as_int();
						}
						StatType::Vitality => {
							proxy.modules.stats.next_tick.stats.vit = stat.stat.as_int();
						}
						StatType::HP => {
							proxy.modules.stats.next_tick.stats.hp = stat.stat.as_int();
						}
						StatType::Speed => {
							proxy.modules.stats.next_tick.stats.spd = stat.stat.as_int();
						}
						StatType::Condition => {
							proxy.modules.stats.next_tick.conditions = PlayerConditions {
								bitmask: stat.stat.as_int() as u64,
							};
						}
						StatType::Condition2 => {
							proxy.modules.stats.next_tick.conditions2 = PlayerConditions2 {
								bitmask: stat.stat.as_int() as u64,
							};
						}
						_ => {}
					}
				}

				proxy.modules.stats.last_tick = proxy.modules.stats.next_tick.clone();
			}
			_ => {}
		}

		Ok(true)
	}
	#[instrument(skip(proxy), fields(modules = ?proxy.modules))]
	async fn disconnect(proxy: &mut Proxy, _by_server: bool) -> Result<()> {
		Ok(())
	}
}

impl TickStats {
	fn empty() -> Self {
		Self {
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
		}
	}
}

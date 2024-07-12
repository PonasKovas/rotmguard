use super::{Module, ModuleInstance, PacketFlow, ProxySide, FORWARD};
use crate::{
	config::Config,
	extra_datatypes::{PlayerConditions, PlayerConditions2, StatType, WorldPos},
	gen_this_macro,
	packets::{ClientPacket, ServerPacket},
	proxy::Proxy,
};
use std::{
	collections::VecDeque,
	io::{Error, Result},
	sync::Arc,
};
use tracing::instrument;

gen_this_macro! {stats}

#[derive(Debug, Clone)]
pub struct Stats {}

#[derive(Debug, Clone)]
pub struct StatsInst {
	// the first one is the current tick (the last one that the client acknowledged)
	// second (if exists) is the next tick that the client hasnt acknowledged yet,
	// and so on..
	// There is always guaranteed to be at least one tick
	pub ticks: VecDeque<TickStats>,
	// player position as last reported by a Move packet
	pub pos: WorldPos,
}

#[derive(Debug, Clone)]
pub struct TickStats {
	// all important stats of the player
	pub stats: PlayerStats,
	// all current important condition effects of the player, such as exposed, cursed, bleeding etc
	pub conditions: PlayerConditions,
	pub conditions2: PlayerConditions2,
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
			ticks: VecDeque::from([TickStats::empty()]),
			pos: WorldPos { x: 0.0, y: 0.0 },
		}
	}
}

impl ModuleInstance for StatsInst {
	async fn client_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		match packet {
			ClientPacket::Move(move_packet) => {
				// Update player position
				if let Some(pos) = move_packet.move_records.last() {
					stats!(proxy).pos = pos.1;
				}

				stats!(proxy).ticks.pop_front();

				if stats!(proxy).ticks.is_empty() {
					// the only way there are no ticks in the VecDeque is if there have been more Move packets than NewTick
					// which should never happen
					return Err(Error::other(
						"client acknowledged tick that wasnt yet received",
					));
				}
			}
			_ => {}
		}

		FORWARD
	}
	async fn server_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		match packet {
			ServerPacket::NewTick(new_tick) => {
				stats!(proxy)
					.ticks
					.push_back(stats!(proxy).ticks.back().unwrap().clone());
				let new_tick_data = stats!(proxy).ticks.back_mut().unwrap();

				let my_status = match new_tick
					.statuses
					.iter_mut()
					.find(|s| s.object_id == proxy.modules.general.my_object_id)
				{
					Some(i) => i,
					None => {
						// no updates for myself, so just forward the original packet
						return FORWARD;
					}
				};

				// Save the important stats and status effects
				for stat in &mut my_status.stats {
					match stat.stat_type {
						StatType::MaxHP => {
							new_tick_data.stats.max_hp = stat.stat.as_int();
						}
						StatType::Defense => {
							new_tick_data.stats.def = stat.stat.as_int();
						}
						StatType::Vitality => {
							new_tick_data.stats.vit = stat.stat.as_int();
						}
						StatType::HP => {
							new_tick_data.stats.hp = stat.stat.as_int();
						}
						StatType::Speed => {
							new_tick_data.stats.spd = stat.stat.as_int();
						}
						StatType::Condition => {
							new_tick_data.conditions = PlayerConditions {
								bitmask: stat.stat.as_int() as u64,
							};
						}
						StatType::Condition2 => {
							new_tick_data.conditions2 = PlayerConditions2 {
								bitmask: stat.stat.as_int() as u64,
							};
						}
						_ => {}
					}
				}
			}
			_ => {}
		}

		FORWARD
	}
}

impl StatsInst {
	// gets the current stats
	pub fn get(&self) -> &TickStats {
		self.ticks.front().unwrap()
	}
	// gets the most recent stats
	pub fn get_newest(&self) -> &TickStats {
		self.ticks.back().unwrap()
	}
}

impl TickStats {
	fn empty() -> Self {
		Self {
			stats: PlayerStats {
				hp: 0,
				max_hp: 10000,
				def: 0,
				vit: 0,
				spd: 0,
			},
			conditions: PlayerConditions { bitmask: 0 },
			conditions2: PlayerConditions2 { bitmask: 0 },
		}
	}
}

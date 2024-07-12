use super::{Module, ModuleInstance, PacketFlow, ProxySide, FORWARD};
use crate::{
	config::{Config, Debuffs},
	extra_datatypes::{ObjectId, PlayerConditions, Stat, StatType, WorldPos},
	gen_this_macro,
	packets::{ClientPacket, ServerPacket, ShowEffect},
	proxy::Proxy,
	util::Notification,
};
use rand::{thread_rng, Rng};
use std::{io::Result, sync::Arc};
use tracing::{error, info, instrument};

gen_this_macro! {antidebuffs}

#[derive(Debug, Clone)]
pub struct Antidebuffs {}

#[derive(Debug, Clone)]
pub struct AntidebuffsInst {}

impl Module for Antidebuffs {
	type Instance = AntidebuffsInst;

	fn new() -> Self {
		Antidebuffs {}
	}
	fn instance(&self) -> Self::Instance {
		AntidebuffsInst {}
	}
}

impl ModuleInstance for AntidebuffsInst {
	#[instrument(skip(proxy), fields(modules = ?proxy.modules))]
	async fn server_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		match packet {
			ServerPacket::Update(update) => {
				// only interested in my own stats
				let my_status = match update
					.new_objects
					.iter_mut()
					.find(|obj| obj.1.object_id == proxy.modules.general.my_object_id)
				{
					Some(me) => &mut me.1,
					None => return FORWARD,
				};

				if let Some(conditions) = my_status
					.stats
					.iter_mut()
					.find(|s| s.stat_type == StatType::Condition)
				{
					let mut cond = PlayerConditions {
						bitmask: conditions.stat.as_int() as u64,
					};
					remove_debuffs(proxy, &mut cond);
					conditions.stat = Stat::Int(cond.bitmask as i64);
				}
			}
			// This packet updates existing objects
			ServerPacket::NewTick(new_tick) => {
				let my_status = match new_tick
					.statuses
					.iter_mut()
					.find(|s| s.object_id == proxy.modules.general.my_object_id)
				{
					Some(i) => i,
					None => {
						return FORWARD;
					}
				};

				if let Some(conditions) = my_status
					.stats
					.iter_mut()
					.find(|s| s.stat_type == StatType::Condition)
				{
					let mut cond = PlayerConditions {
						bitmask: conditions.stat.as_int() as u64,
					};
					remove_debuffs(proxy, &mut cond);
					conditions.stat = Stat::Int(cond.bitmask as i64);
				}
			}
			_ => {}
		}

		FORWARD
	}
}

fn remove_debuffs(proxy: &mut Proxy, condition: &mut PlayerConditions) {
	if proxy.config.settings.debuffs.blind {
		condition.set_blind(false);
	}
	if proxy.config.settings.debuffs.hallucinating {
		condition.set_hallucinating(false);
	}
	if proxy.config.settings.debuffs.drunk {
		condition.set_drunk(false);
	}
	if proxy.config.settings.debuffs.confused {
		condition.set_confused(false);
	}
	if proxy.config.settings.debuffs.unstable {
		condition.set_unstable(false);
	}
	if proxy.config.settings.debuffs.darkness {
		condition.set_darkness(false);
	}
}

use super::{Module, ModuleInstance, PacketFlow, BLOCK, FORWARD};
use crate::{
	extra_datatypes::{ObjectStatusData, Stat, StatData, StatType},
	gen_this_macro,
	packets::{ClientPacket, ServerPacket},
	proxy::Proxy,
	util::Notification,
};
use std::io::Result;

gen_this_macro! {fake_slow}

#[derive(Debug, Clone)]
pub struct FakeSlow {}

#[derive(Debug, Clone)]
pub struct FakeSlowInst {
	enabled: bool,
	synced: bool,
}

impl Module for FakeSlow {
	type Instance = FakeSlowInst;

	fn new() -> Self {
		FakeSlow {}
	}
	fn instance(&self) -> Self::Instance {
		FakeSlowInst {
			enabled: false,
			synced: true,
		}
	}
}

impl ModuleInstance for FakeSlowInst {
	async fn client_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		if let ClientPacket::PlayerText(text) = packet {
			let text = &text.text;
			// `/slow` toggles a permanent slow effect
			if text.starts_with("/slow") {
				fake_slow!(proxy).enabled = !fake_slow!(proxy).enabled;
				let msg = if fake_slow!(proxy).enabled {
					"Slow enabled."
				} else {
					"Slow disabled."
				};
				fake_slow!(proxy).synced = false;

				Notification::new(msg.to_owned())
					.color(0xff33ff)
					.send(&mut proxy.write)
					.await?;

				return BLOCK;
			}
		}

		FORWARD
	}
	async fn server_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		if let ServerPacket::NewTick(new_tick) = packet {
			let mut conditions = proxy.modules.stats.get_newest().conditions;
			if conditions.slow() {
				// slow already originally, so just forward, cant do anything
				return FORWARD;
			}
			conditions.set_slow(fake_slow!(proxy).enabled);

			let conditions_stat = StatData {
				stat_type: StatType::Condition,
				stat: Stat::Int(conditions.bitmask as i64),
				secondary_stat: -1,
			};

			// check if theres a new status for me
			match new_tick
				.statuses
				.iter_mut()
				.find(|s| s.object_id == proxy.modules.general.my_object_id)
			{
				Some(me) => {
					// If condition already present, replace it
					if let Some(cond) = me
						.stats
						.iter_mut()
						.find(|s| s.stat_type == StatType::Condition)
					{
						*cond = conditions_stat;
					} else if !fake_slow!(proxy).synced {
						// if not present, add it (but only if need to sync)
						me.stats.push(conditions_stat);
					}
				}
				None if fake_slow!(proxy).synced => {
					// no updates for me but its synced so no need to change anything
					return FORWARD;
				}
				None => {
					// no updates for myself, but need to sync so add manually
					new_tick.statuses.push(ObjectStatusData {
						object_id: proxy.modules.general.my_object_id,
						position: proxy.modules.stats.pos,
						stats: vec![conditions_stat],
					});
				}
			}

			fake_slow!(proxy).synced = true;
		}

		FORWARD
	}
}

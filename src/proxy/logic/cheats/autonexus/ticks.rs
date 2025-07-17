use super::devmode;
use crate::{
	proxy::{
		Proxy,
		logic::packets::{ExtraObject, StatData},
	},
	util::{CONDITION_BITFLAG, STAT_TYPE, create_effect, create_notification},
};
use either::Either;
use serde::Deserialize;
use std::collections::VecDeque;
use tracing::{error, warn};

pub struct Ticks {
	queue: VecDeque<Tick>,
	// this will have only server-sent data
	// and will be used as a base to create new ticks
	base: Tick,
}

#[derive(Clone, Copy, Default)]
pub struct Tick {
	pub id: u32,
	pub stats: Stats,
	// total summed heals on self during that tick
	pub heals: i64,
	pub time: u32,
}

#[derive(Clone, Copy, Default)]
pub struct Stats {
	pub hp: i64,
	pub max_hp: i64,
	pub def: i64,
	pub vit: i64,
	pub conditions: u64,
	pub conditions2: u64,
}

impl Default for Ticks {
	fn default() -> Self {
		Self {
			queue: VecDeque::from([Tick::default()]),
			base: Tick::default(),
		}
	}
}

impl Ticks {
	pub fn current(&self) -> Tick {
		*self.queue.front().unwrap()
	}
	pub fn for_each(&mut self, mut f: impl FnMut(&mut Tick)) {
		for tick in &mut self.queue {
			f(tick);
		}
	}
}

pub fn new_tick_start(proxy: &mut Proxy, tick_id: u32, tick_time: u32) {
	proxy.state.autonexus.ticks.base.id = tick_id;
	proxy.state.autonexus.ticks.base.time = tick_time;
}

pub async fn self_stat(proxy: &mut Proxy, stat_type: u8, stat: i64) {
	let last_tick = &mut proxy.state.autonexus.ticks.base;
	match stat_type {
		STAT_TYPE::MAX_HP => {
			last_tick.stats.max_hp = stat;
		}
		STAT_TYPE::HP => {
			last_tick.stats.hp = stat;
		}
		STAT_TYPE::DEFENSE => {
			last_tick.stats.def = stat;
		}
		STAT_TYPE::VITALITY => {
			last_tick.stats.vit = stat;
		}
		STAT_TYPE::CONDITION => {
			last_tick.stats.conditions = stat as u64;
		}
		STAT_TYPE::CONDITION2 => {
			last_tick.stats.conditions2 = stat as u64;
		}
		_ => {}
	}
}

pub async fn object_notification(proxy: &mut Proxy, message: &str, object_id: u32, color: u32) {
	if color != 0x00ff00 {
		return; // green means heal
	}
	if object_id != proxy.state.my_obj_id {
		return; // only interested in myself
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
				warn!("Unexpected object notification for heal. k not equal to 's.plus_symbol'");
				return;
			}
			match h.t.amount.parse::<i64>() {
				Ok(n) => n,
				Err(e) => {
					error!("Error parsing heal notification amount: {e:?}");
					return;
				}
			}
		}
		Err(_) => {
			return;
		}
	};

	proxy.state.autonexus.ticks.base.heals += amount_healed;
}

pub async fn new_tick_finish(proxy: &mut Proxy) {
	proxy
		.state
		.autonexus
		.ticks
		.queue
		.push_back(proxy.state.autonexus.ticks.base);
	proxy.state.autonexus.ticks.base.heals = 0;
}

pub async fn client_tick_acknowledge(proxy: &mut Proxy) {
	proxy.state.autonexus.ticks.queue.pop_front();

	let tick = proxy.state.autonexus.ticks.current();

	// apply heals and passive effects

	// heals
	proxy.state.autonexus.hp =
		(proxy.state.autonexus.hp + tick.heals as f32).min(tick.stats.max_hp as f32);
	if devmode(proxy) && tick.heals != 0 {
		let notification = create_notification(&format!("HEAL +{}", tick.heals), 0xaaaaaa);
		proxy.send_client(notification).await;
	}

	// passive
	let time_seconds = tick.time as f32 / 1000.0;
	if (tick.stats.conditions & CONDITION_BITFLAG::BLEEDING) != 0 {
		let bleed_amount = 20.0 * time_seconds;

		// bleeding stops at 1
		proxy.state.autonexus.hp = (proxy.state.autonexus.hp - bleed_amount).max(1.0);
	} else if (tick.stats.conditions & CONDITION_BITFLAG::SICK) == 0 {
		// only regenerate if server side hp is lower than max
		if tick.stats.hp < tick.stats.max_hp {
			// vit regeneration
			let vit = tick.stats.vit as f32;
			// these values have been found by doing precise analysis of the packets.
			// the actual range for them can be found in assets/vit_regen_possible_values.png
			// we can assume 2.0 base regen because the polygon falls right on it and its so clean
			// and the cleanest (least decimal places) slope that can be paired with it
			// happens to be 0.2407. so here we are.
			let mut regen_amount = time_seconds * (2.0 + 0.2407 * vit);
			if (tick.stats.conditions & CONDITION_BITFLAG::IN_COMBAT) != 0 {
				regen_amount /= 2.0;
			}

			if (tick.stats.conditions & CONDITION_BITFLAG::HEALING) != 0 {
				regen_amount += 20.0 * time_seconds;
			}

			proxy.state.autonexus.hp =
				(proxy.state.autonexus.hp + regen_amount).min(tick.stats.max_hp as f32);
		}
	}

	let hp_delta = tick.stats.hp - proxy.state.autonexus.hp.round() as i64;

	if devmode(proxy) && hp_delta <= -2 {
		proxy
			.send_client(create_notification(
				&format!("negdelta {hp_delta}"),
				0xff2222,
			))
			.await;
		proxy
			.send_client(create_effect(
				18,
				Some(proxy.state.my_obj_id),
				(0.0, 0.0),
				(0.0, 0.0),
				Some(0xffffff),
				Some(1.0),
			))
			.await;
	}

	let ticks_since_damage = tick.id - proxy.state.autonexus.last_damage_tick;
	if (ticks_since_damage >= 10 && hp_delta != 0) || hp_delta <= -1 {
		if devmode(proxy) {
			proxy
				.send_client(create_notification(&format!("SYNC {hp_delta}"), 0xff88ff))
				.await;
		}

		proxy.state.autonexus.hp = tick.stats.hp as f32;
	}
}

// if devmode enabled will replace the fame bar with simulated hp
pub fn extra_object_status(
	proxy: &mut Proxy,
) -> impl Iterator<Item = ExtraObject<impl Iterator<Item = StatData<'_>> + ExactSizeIterator>> {
	if !devmode(proxy) {
		return None.into_iter();
	}

	Some(ExtraObject {
		obj_id: proxy.state.my_obj_id,
		pos_x: 0.0,
		pos_y: 0.0,
		stats: [
			StatData {
				stat_type: STAT_TYPE::CURRENT_FAME,
				data: Either::Right(proxy.state.autonexus.hp as i64),
				secondary: -1,
			},
			StatData {
				stat_type: STAT_TYPE::CLASS_QUEST_FAME,
				data: Either::Right(proxy.state.autonexus.ticks.current().stats.max_hp),
				secondary: -1,
			},
		]
		.into_iter(),
	})
	.into_iter()
}

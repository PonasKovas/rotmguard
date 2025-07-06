use crate::{
	proxy::{Proxy, logic::packets::CONDITION_STAT_ID},
	util::{GREEN, RED, static_notification},
};
use std::iter::once;

use super::antidebuffs;

const SLOW_BIT: u64 = 0x8;

pub struct FakeSlow {
	condition: u64,
	enabled: bool,
	synced: bool,
}

impl Default for FakeSlow {
	fn default() -> Self {
		Self {
			condition: 0,
			enabled: false,
			synced: true,
		}
	}
}

/// To be called in NewTick when the condition stat about self is read
/// may modify the stat
pub fn self_condition_stat(proxy: &mut Proxy, stat: &mut i64) {
	proxy.state.fakeslow.condition = *stat as u64;
	proxy.state.fakeslow.synced = true;

	if proxy.state.fakeslow.enabled {
		*stat = ((*stat as u64) | SLOW_BIT) as i64;
	}
}

/// toggles the antipush cheat
pub async fn toggle(proxy: &mut Proxy) {
	proxy.state.fakeslow.enabled = !proxy.state.fakeslow.enabled;
	proxy.state.fakeslow.synced = false;

	let notification = if proxy.state.fakeslow.enabled {
		static_notification!("Fake slow enabled", GREEN)
	} else {
		static_notification!("Fake slow disabled", RED)
	};

	proxy.send_client(notification).await;
}

// checks if fakeslow is not synced, and if so, returns an extra self object status
// to add to a newtick packet. (object id, position, stats)
pub fn extra_object_status(
	proxy: &mut Proxy,
) -> Option<(
	u32,
	(f32, f32),
	impl Iterator<Item = (u8, i64, i64)> + ExactSizeIterator,
)> {
	if proxy.state.fakeslow.synced {
		return None;
	}
	proxy.state.fakeslow.synced = true;

	let mut new_condition = proxy.state.fakeslow.condition as i64;
	antidebuffs::self_condition_stat(proxy, &mut new_condition);
	if proxy.state.fakeslow.enabled {
		new_condition |= SLOW_BIT as i64;
	}

	Some((
		proxy.state.my_obj_id,
		(0.0, 0.0),
		once((CONDITION_STAT_ID, new_condition, -1)),
	))
}

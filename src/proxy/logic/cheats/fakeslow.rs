use super::antidebuffs;
use crate::{
	proxy::{
		Proxy,
		logic::packets::{ExtraObject, StatData},
	},
	util::{CONDITION_BITFLAG, GREEN, RED, STAT_TYPE, static_notification},
};
use either::Either;
use std::iter::once;

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
		*stat = ((*stat as u64) | CONDITION_BITFLAG::SLOW) as i64;
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
// to add to a newtick packet.
pub fn extra_object_status(
	proxy: &mut Proxy,
) -> impl Iterator<Item = ExtraObject<impl Iterator<Item = StatData<'_>> + ExactSizeIterator>> {
	if proxy.state.fakeslow.synced {
		return None.into_iter();
	}
	proxy.state.fakeslow.synced = true;

	let mut new_condition = proxy.state.fakeslow.condition as i64;
	antidebuffs::self_condition_stat(proxy, &mut new_condition);
	if proxy.state.fakeslow.enabled {
		new_condition |= CONDITION_BITFLAG::SLOW as i64;
	}

	Some(ExtraObject {
		obj_id: proxy.state.my_obj_id,
		pos_x: 0.0, // literally doesnt matter
		pos_y: 0.0,
		stats: once(StatData {
			stat_type: STAT_TYPE::CONDITION,
			data: Either::Right(new_condition),
			secondary: -1,
		}),
	})
	.into_iter()
}

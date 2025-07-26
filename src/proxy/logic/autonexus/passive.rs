use super::{devmode, get_conditions};
use crate::{
	proxy::Proxy,
	util::{CONDITION_BITFLAG, create_effect, create_notification},
};

pub async fn new_tick(proxy: &mut Proxy, _id: u32, time: u32) {
	let time_seconds = time as f32 / 1000.0;

	let (conditions, _conditions2) = get_conditions(proxy);
	let stats = proxy.state.common.objects.get_self().stats;

	if (conditions & CONDITION_BITFLAG::BLEEDING) != 0 {
		let bleed_amount = 20.0 * time_seconds;

		// bleeding stops at 1
		proxy.state.autonexus.hp = (proxy.state.autonexus.hp - bleed_amount).max(1.0);
	} else if (conditions & CONDITION_BITFLAG::SICK) == 0 {
		// if not sick

		// only regenerate if server side hp is lower than max
		if stats.hp < stats.max_hp {
			// vit regeneration
			let vit = stats.vit as f32;
			// these values have been found by doing precise analysis of the packets.
			// the actual range for them can be found in assets/vit_regen_possible_values.png
			// we can assume 2.0 base regen because the polygon falls right on it and its so clean
			// and the cleanest (least decimal places) slope that can be paired with it
			// happens to be 0.2407. so here we are.
			let mut regen_amount = time_seconds * (2.0 + 0.2407 * vit);
			if (stats.conditions & CONDITION_BITFLAG::IN_COMBAT) != 0 {
				regen_amount /= 2.0;
			}

			if (stats.conditions & CONDITION_BITFLAG::HEALING) != 0 {
				regen_amount += 20.0 * time_seconds;
			}

			proxy.state.autonexus.hp =
				(proxy.state.autonexus.hp + regen_amount).min(stats.max_hp as f32);
		}
	}

	let hp_delta = stats.hp - proxy.state.autonexus.hp.round() as i64;

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
				Some(proxy.state.common.objects.self_id),
				(0.0, 0.0),
				(0.0, 0.0),
				Some(0xffffff),
				Some(1.0),
			))
			.await;
	}

	if hp_delta <= -1 {
		if devmode(proxy) {
			proxy
				.send_client(create_notification(&format!("SYNC {hp_delta}"), 0xff88ff))
				.await;
		}

		proxy.state.autonexus.hp = stats.hp as f32;
	}
}

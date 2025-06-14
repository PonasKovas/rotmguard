use crate::proxy::Proxy;

pub fn apply_passive(proxy: &mut Proxy, time: f64) {
	// use previous tick stats
	let stats = proxy.modules.stats.get_newest();

	// apply bleeding/healing if there are to client hp now
	if stats.conditions.bleeding() {
		let bleed_amount = 20.0 * time;

		autonexus!(proxy).hp = (autonexus!(proxy).hp - bleed_amount).max(1.0); // bleeding stops at 1
	} else if !stats.conditions.sick() {
		// if not bleeding, nor sickened
		// also do not regenerate if previous tick server hp was full
		let prev_tick = &proxy.modules.stats.ticks[proxy.modules.stats.ticks.len() - 2];

		if prev_tick.stats.hp == prev_tick.stats.max_hp {
			return;
		}

		if stats.conditions.healing() {
			let heal_amount = 20.0 * time;
			autonexus!(proxy).hp += heal_amount;
		} else {
			// vit regeneration
			let vit = stats.stats.vit;
			let mut regen_amount = time * (1.0 + 0.24 * vit as f64);
			if stats.conditions.in_combat() {
				regen_amount /= 2.0;
			};
			autonexus!(proxy).hp += regen_amount;
		}

		autonexus!(proxy).hp = autonexus!(proxy).hp.min(stats.stats.max_hp as f64); // make sure our client hp is not more than max hp
	}
}

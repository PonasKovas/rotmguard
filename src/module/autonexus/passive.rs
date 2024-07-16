use crate::proxy::Proxy;
use tracing::trace;

pub fn apply_passive(proxy: &mut Proxy<'_>, time: f64) {
	let stats = proxy.modules.stats.get_newest();

	// apply bleeding/healing if there are to client hp now
	if stats.conditions.bleeding() {
		let bleed_amount = 20.0 * time;

		autonexus!(proxy).hp = (autonexus!(proxy).hp - bleed_amount).max(1.0); // bleeding stops at 1
		trace!(?proxy.modules, bleed_amount, "Applied bleeding");
	} else if !stats.conditions.sick() {
		// if not bleeding, nor sickened
		// also do not regenerate if last tick server hp was full

		let prev_tick_stats = proxy.modules.stats.ticks[proxy.modules.stats.ticks.len() - 2].stats;
		if prev_tick_stats.hp == prev_tick_stats.max_hp {
			trace!(?proxy.modules, "Not healing because previous tick server hp was full");
			return;
		}

		if stats.conditions.healing() {
			let heal_amount = 20.0 * time;
			autonexus!(proxy).hp += heal_amount;
			trace!(?proxy.modules, heal_amount, "Applying healing effect");
		}

		// vit regeneration
		let vit = stats.stats.vit;
		let mut regen_amount = time * (1.0 + 0.24 * vit as f64);
		if stats.conditions.in_combat() {
			regen_amount /= 2.0;
		};
		autonexus!(proxy).hp = (autonexus!(proxy).hp + regen_amount).min(stats.stats.max_hp as f64); // make sure our client hp is not more than max hp
		trace!(?proxy.modules, regen_amount, "VIT regeneration");
	}
}

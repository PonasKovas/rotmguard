use super::{take_damage, Module, ModuleInstance, PacketFlow, ProxySide, FORWARD};
use crate::{
	config::Config,
	extra_datatypes::{ObjectId, WorldPos},
	gen_this_macro,
	module::{autonexus::nexus, BLOCK},
	packets::{ClientPacket, GroundDamage, ServerPacket, ShowEffect, UpdatePacket},
	proxy::Proxy,
	util::Notification,
};
use rand::{thread_rng, Rng};
use std::{collections::HashMap, io::Result, sync::Arc};
use tracing::{error, info, instrument, trace};

gen_this_macro! {autonexus.passive}

#[derive(Debug, Clone)]
pub struct Passive {}

impl Passive {
	pub fn new() -> Self {
		Passive {}
	}
	pub fn apply_passive(proxy: &mut Proxy<'_>, time: f64) {
		let stats = proxy.modules.stats.get_newest();

		// apply bleeding/healing if there are to client hp now
		if stats.conditions.bleeding() {
			let bleed_amount = 20.0 * time;

			autonexus!(proxy).hp = (autonexus!(proxy).hp - bleed_amount).max(1.0); // bleeding stops at 1
			trace!(bleed_amount, "Applied bleeding");
		} else if !stats.conditions.sick() {
			// if not bleeding, nor sickened

			if stats.conditions.healing() {
				let heal_amount = 20.0 * time;
				autonexus!(proxy).hp += heal_amount;
				trace!(heal_amount, "Applying healing effect");
			}

			// vit regeneration
			let vit = stats.stats.vit;
			let mut regen_amount = time * (1.0 + 0.24 * vit as f64);
			if stats.conditions.in_combat() {
				regen_amount /= 2.0;
			};
			autonexus!(proxy).hp =
				(autonexus!(proxy).hp + regen_amount).min(stats.stats.max_hp as f64); // make sure our client hp is not more than max hp
			trace!(regen_amount, "VIT regeneration");
		}
	}
}

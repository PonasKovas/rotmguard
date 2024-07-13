use super::{take_damage, Module, PacketFlow, FORWARD};
use crate::{
	gen_this_macro,
	module::BLOCK,
	packets::AoePacket,
	proxy::Proxy,
};
use std::{
	collections::VecDeque,
	io::Result,
};
use tracing::trace;

gen_this_macro! {autonexus.aoes}

#[derive(Debug, Clone)]
pub struct AOEs {
	// First element is AOEs that have occured in the last tick that the client didnt acknowledge yet
	// Second is for next tick, etc
	pub aoes: VecDeque<Vec<AoePacket>>,
}

impl AOEs {
	pub fn new() -> Self {
		AOEs {
			aoes: VecDeque::from([Vec::new()]),
		}
	}
	pub fn add_aoe(proxy: &mut Proxy<'_>, aoe: &AoePacket) {
		aoes!(proxy).aoes.back_mut().unwrap().push(aoe.clone());
	}
	pub fn flush(proxy: &mut Proxy<'_>) {
		aoes!(proxy).aoes.push_back(Vec::new());
	}
	// Returns BLOCK if nexused
	pub async fn check_aoes(proxy: &mut Proxy<'_>) -> Result<PacketFlow> {
		let aoes = aoes!(proxy)
			.aoes
			.pop_front()
			.expect("client acknowledged more ticks that server sent");

		// this remapping is so that it can be logged conveniently (which aoes hit)
		let mut aoes: Vec<(AoePacket, bool)> = aoes.into_iter().map(|a| (a, false)).collect();

		// invincible is checked at take_damage because it applies to everything
		// while invulnerable doesnt apply to ground damage
		if proxy.modules.stats.get().conditions.invulnerable() {
			return FORWARD;
		}

		let player_pos = proxy.modules.stats.pos;
		for (aoe, affects_me) in &mut aoes {
			let distance = ((aoe.position.x - player_pos.x).powi(2)
				+ (aoe.position.y - player_pos.y).powi(2))
			.sqrt();

			if distance <= aoe.radius {
				*affects_me = true;

				let conditions = proxy.modules.stats.get().conditions;
				let conditions2 = proxy.modules.stats.get().conditions2;

				let mut damage = if aoe.armor_piercing || conditions.armor_broken() {
					aoe.damage as i64
				} else {
					let mut def = proxy.modules.stats.get().stats.def;
					if conditions.armored() {
						def += def / 2; // x1.5
					}
					let damage = aoe.damage as i64 - def;
					// a bullet must always deal at least 10% of its damage, doesnt matter the def
					let min_damage = aoe.damage as i64 / 10;

					damage.max(min_damage)
				};

				if conditions2.exposed() {
					damage += 20;
				}
				if conditions2.cursed() {
					damage = (damage as f64 * 1.25).floor() as i64;
				}

				// conditions :/
				// match aoe.effect {
				// 	5 => {
				// 		proxy.modules.stats.last_tick.conditions.set_sick(true);
				// 	}
				// 	16 => {
				// 		proxy.modules.stats.last_tick.conditions.set_bleeding(true);
				// 	}
				// 	_ => {}
				// }

				if take_damage(proxy, damage).await? == PacketFlow::Block {
					return BLOCK; // dont forward if nexusing
				}
			}
		}

		if !aoes.is_empty() {
			trace!(?aoes, "AOEs");
		}

		FORWARD
	}
}

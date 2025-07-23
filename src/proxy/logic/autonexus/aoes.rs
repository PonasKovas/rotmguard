use super::take_damage;
use crate::{proxy::Proxy, util::CONDITION_BITFLAG};
use anyhow::{Context, Result};
use std::collections::VecDeque;

#[derive(Default)]
pub struct Aoes {
	queue: VecDeque<Aoe>,
}

struct Aoe {
	pos: (f32, f32),
	radius: f32,
	damage: u16,
	sickens: bool,
	bleeds: bool,
	armor_piercing: bool,
}

pub async fn aoe(
	proxy: &mut Proxy,
	pos_x: f32,
	pos_y: f32,
	radius: f32,
	damage: u16,
	effect: u8,
	armor_piercing: bool,
) {
	let aoe = Aoe {
		pos: (pos_x, pos_y),
		radius,
		damage,
		sickens: effect == 5,
		bleeds: effect == 16,
		armor_piercing,
	};
	proxy.state.autonexus.aoes.queue.push_back(aoe);
}

pub async fn aoeack(proxy: &mut Proxy, pos_x: f32, pos_y: f32) -> Result<()> {
	let aoe = proxy
		.state
		.autonexus
		.aoes
		.queue
		.pop_front()
		.context("client acknowledging aoe when none were sent?")?;

	// good ol' pythagorean theorem
	let distance = ((aoe.pos.0 - pos_x).powi(2) + (aoe.pos.1 - pos_y).powi(2)).sqrt();

	if distance > aoe.radius {
		// is fine
		return Ok(());
	}

	// hole shit. WE ARE HIT
	take_damage(proxy, aoe.damage as i64, aoe.armor_piercing).await;

	// apply any status effects
	proxy.state.autonexus.ticks.for_each(|tick| {
		if aoe.sickens {
			tick.stats.conditions |= CONDITION_BITFLAG::SICK;
		}
		if aoe.bleeds {
			tick.stats.conditions |= CONDITION_BITFLAG::BLEEDING;
		}
	});

	Ok(())
}

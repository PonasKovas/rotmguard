use crate::{
	proxy::Proxy,
	util::{
		BLUE, CONDITION_BITFLAG, CONDITION2_BITFLAG, GREEN, RED, STAT_TYPE, create_escape,
		create_notification,
	},
};
use tracing::{error, info};

mod aoes;
mod ground;
mod projectiles;
mod ticks;

pub use aoes::{aoe, aoeack};
pub use ground::{ground_damage, new_tile};
pub use projectiles::{add_object, new_bullet, player_hit, remove_object};
pub use ticks::{
	client_tick_acknowledge, extra_object_status, new_tick_finish, new_tick_start,
	object_notification, self_stat,
};

#[derive(Default)]
pub struct Autonexus {
	hp: f32,
	last_damage_tick: u32, // tick id when was the last time some damage was taken
	ticks: ticks::Ticks,
	ground: ground::Ground,
	projectiles: projectiles::Projectiles,
	aoes: aoes::Aoes,
}

pub async fn command(proxy: &mut Proxy, mut args: impl Iterator<Item = &str>) {
	let value = match args.next() {
		None => {
			let current_value = *proxy.rotmguard.config.settings.autonexus_hp.lock().unwrap();
			let notification = create_notification(
				&format!("/autonexus [HP]\nCurrent value: {current_value}",),
				BLUE,
			);
			proxy.send_client(notification).await;
			return;
		}
		Some(v) => v,
	};

	let parsed = match value.parse::<i32>() {
		Ok(i) => i,
		Err(e) => {
			let notification =
				create_notification(&format!("/autonexus [HP]\nError parsing HP: {e}"), RED);
			proxy.send_client(notification).await;
			error!("Error parsing /autonexus command HP: {e:?}");
			return;
		}
	};

	*proxy.rotmguard.config.settings.autonexus_hp.lock().unwrap() = parsed;

	let notification =
		create_notification(&format!("Autonexus threshold set to {parsed} HP."), GREEN);
	proxy.send_client(notification).await;
}

// only in Update packet when self object is initially added
pub fn initial_self_stat(proxy: &mut Proxy, stat_type: u8, stat: &mut i64) {
	if stat_type == STAT_TYPE::HP {
		proxy.state.autonexus.hp = *stat as f32;
	}
}

// calculates and applies the real damage, taking into account status effects and everything
async fn take_damage(proxy: &mut Proxy, mut damage: i64, armor_piercing: bool) {
	let tick = proxy.state.autonexus.ticks.current();
	let conditions = tick.stats.conditions;
	let conditions2 = tick.stats.conditions2;

	if (conditions & CONDITION_BITFLAG::INVULNERABLE) != 0 {
		return;
	}

	// calculate damage
	if !armor_piercing && (conditions & CONDITION_BITFLAG::ARMOR_BROKEN) == 0 {
		let mut def = tick.stats.def;
		if (conditions & CONDITION_BITFLAG::ARMORED) != 0 {
			def += def / 2; // x1.5
		}

		let potential_damage = damage - def;
		// a bullet must always deal at least 10% of its damage, doesnt matter the def
		let min_damage = damage as i64 / 10;

		damage = potential_damage.max(min_damage);
	}

	if (conditions2 & CONDITION2_BITFLAG::EXPOSED) != 0 {
		damage += 20;
	}
	if (conditions2 & CONDITION2_BITFLAG::CURSED) != 0 {
		damage += damage / 4; // x 1.25
	}

	take_damage_raw(proxy, damage).await;
}

// just applies already calculated raw damage
async fn take_damage_raw(proxy: &mut Proxy, dmg: i64) {
	let condition = proxy.state.autonexus.ticks.current().stats.conditions;
	if (condition & CONDITION_BITFLAG::INVINCIBLE) != 0 {
		return; // player is invincible, no damage can be taken
	}

	proxy.state.autonexus.hp -= dmg as f32;
	proxy.state.autonexus.last_damage_tick = proxy.state.autonexus.ticks.current().id;

	let threshold = *proxy.rotmguard.config.settings.autonexus_hp.lock().unwrap();
	if proxy.state.autonexus.hp < threshold as f32 {
		// AUTONEXUS ENGAGE!!!
		proxy.send_server(create_escape()).await;
		info!("nexusing");
	}

	if devmode(proxy) {
		let notification = create_notification(&format!("-{dmg}"), 0x888888);
		proxy.send_client(notification).await;
	}
}

fn devmode(proxy: &mut Proxy) -> bool {
	*proxy.rotmguard.config.settings.dev_mode.lock().unwrap()
}

use crate::{
	proxy::Proxy,
	util::{BLUE, CONDITION_BITFLAG, GREEN, RED, STAT_TYPE, create_escape, create_notification},
};
use tracing::{error, info};

mod ground;
mod projectiles;
mod ticks;

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

async fn take_damage(proxy: &mut Proxy, dmg: i64) {
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

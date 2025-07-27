use crate::{
	proxy::{
		Proxy,
		packets::{ExtraObject, StatData},
	},
	util::{
		BLUE, CONDITION_BITFLAG, CONDITION2_BITFLAG, GREEN, RED, STAT_TYPE, create_escape,
		create_notification,
	},
};
use either::Either;
use tracing::{error, info};

mod aoes;
mod ground;
mod heals;
mod passive;
mod projectiles;

pub use aoes::{aoe, aoeack};
pub use ground::{ground_damage, new_tile};
pub use heals::object_notification;
pub use passive::new_tick;
pub use projectiles::player_hit;

#[derive(Default)]
pub struct Autonexus {
	hp: f32,
	// tick id after (client acknowledging) which it is assumed to be safe to sync HP with the server
	tick_to_sync_after: u32,
	inflicted_conditions: Vec<InflictedCondition>,
	ground: ground::Ground,
	aoes: aoes::Aoes,
}

struct InflictedCondition {
	condition: u64,
	condition2: u64,
	// in milliseconds
	expires_in: u32,
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

pub async fn client_tick_ack(proxy: &mut Proxy, tick_id: u32, tick_time: u32) {
	// clean up any inflicted conditions
	for i in (0..proxy.state.autonexus.inflicted_conditions.len()).rev() {
		let cond = &mut proxy.state.autonexus.inflicted_conditions[i];

		match cond.expires_in.checked_sub(tick_time) {
			Some(remaining) => {
				cond.expires_in = remaining;
			}
			None => {
				proxy.state.autonexus.inflicted_conditions.remove(i);
			}
		}
	}

	// sync hp if safe to do so
	let stats = proxy.state.common.objects.get_self().stats;
	let hp_delta = stats.hp - proxy.state.autonexus.hp.round() as i64;
	let safe_to_sync = tick_id > proxy.state.autonexus.tick_to_sync_after;
	if safe_to_sync && hp_delta != 0 {
		proxy.state.autonexus.hp = stats.hp as f32;
	}
}

// if devmode enabled will replace the fame bar with simulated hp
pub fn extra_object_status(
	proxy: &mut Proxy,
) -> impl Iterator<Item = ExtraObject<impl Iterator<Item = StatData<'_>> + ExactSizeIterator>> {
	if !devmode(proxy) {
		return None.into_iter();
	}

	Some(ExtraObject {
		obj_id: proxy.state.common.objects.self_id,
		pos_x: 0.0,
		pos_y: 0.0,
		stats: [
			StatData {
				stat_type: STAT_TYPE::CURRENT_FAME,
				data: Either::Right(proxy.state.autonexus.hp as i64),
				secondary: -1,
			},
			StatData {
				stat_type: STAT_TYPE::CLASS_QUEST_FAME,
				data: Either::Right(proxy.state.common.objects.get_self().stats.max_hp),
				secondary: -1,
			},
		]
		.into_iter(),
	})
	.into_iter()
}

// calculates and applies the real damage, taking into account status effects and everything
async fn take_damage(proxy: &mut Proxy, mut damage: i64, armor_piercing: bool) {
	let stats = proxy.state.common.objects.get_self().stats;
	let (conditions, conditions2) = get_conditions(proxy);

	if (conditions & CONDITION_BITFLAG::INVULNERABLE) != 0 {
		return;
	}

	// calculate damage
	if !armor_piercing && (conditions & CONDITION_BITFLAG::ARMOR_BROKEN) == 0 {
		let mut def = stats.def;
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
	if (conditions2 & CONDITION2_BITFLAG::PETRIFIED) != 0 {
		damage -= damage / 10; // x 0.9
	}

	// If any equipped items enchanted with damage resistance
	for item in &proxy.state.common.objects.get_self().equipped_items {
		if let Some(item) = item {
			for enchantment in &item.enchantments {
				match proxy.rotmguard.assets.enchantments.get(enchantment) {
					Some(enchantment) => {
						for effect in &enchantment.effects {
							match effect {
								crate::assets::EnchantmentEffect::SelfDamageMult(x) => {
									damage = (damage as f32 * x).ceil() as i64; // ceil just to be safe, idk really
								}
								_ => {}
							}
						}
					}
					None => {
						error!("Unknown enchantment id {enchantment}");
					}
				};
			}
		}
	}

	take_damage_raw(proxy, damage).await;
}

// just applies already calculated raw damage
async fn take_damage_raw(proxy: &mut Proxy, dmg: i64) {
	let (condition, _condition2) = get_conditions(proxy);
	if (condition & (CONDITION_BITFLAG::INVINCIBLE | CONDITION_BITFLAG::STASIS)) != 0 {
		return; // player is invincible/stasis, no damage can be taken
	}

	proxy.state.autonexus.hp -= dmg as f32;
	reset_safe_sync_delay(proxy);

	check_health(proxy).await;

	if devmode(proxy) {
		let notification = create_notification(&format!("-{dmg}"), 0x888888);
		proxy.send_client(notification).await;
	}
}

fn reset_safe_sync_delay(proxy: &mut Proxy) {
	// safe to sync HP only after client acknowledges 10 ticks after current server tick
	proxy.state.autonexus.tick_to_sync_after = proxy.state.common.server_tick_id + 10;
}

async fn check_health(proxy: &mut Proxy) {
	let threshold = *proxy.rotmguard.config.settings.autonexus_hp.lock().unwrap();
	if proxy.state.autonexus.hp < threshold as f32 {
		// AUTONEXUS ENGAGE!!!
		proxy.send_server(create_escape()).await;
		info!("nexusing");
	}
}

fn devmode(proxy: &mut Proxy) -> bool {
	*proxy.rotmguard.config.settings.dev_mode.lock().unwrap()
}

// gets the self player conditions, taking into account also conditions that were inflicted
// by bullets and maybe not necessarily reflected in the server-provided conditions YET
fn get_conditions(proxy: &mut Proxy) -> (u64, u64) {
	let mut cond = proxy.state.common.objects.get_self().stats.conditions;
	let mut cond2 = proxy.state.common.objects.get_self().stats.conditions2;

	for c in &proxy.state.autonexus.inflicted_conditions {
		cond |= c.condition;
		cond2 |= c.condition2;
	}

	(cond, cond2)
}

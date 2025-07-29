use super::{DamageMonitor, TAKEN_DAMAGE_CRITERIA};
use crate::damage_monitor_http_server::{EnemyTab, PlayerItem, PlayerRow, Report};
use base64::{Engine, prelude::BASE64_STANDARD};
use std::collections::BTreeMap;

/// generates a report if map name set
pub fn generate_report(this: &DamageMonitor) -> Option<Report> {
	let map_name = match &this.map_name {
		Some(x) => x.clone(),
		None => return None,
	};

	// sort enemies by total damage done to them
	let mut enemy_tabs: Vec<EnemyTab> = this
		.enemies
		.iter()
		.map(|(_enemy_id, enemy)| {
			let total_damage = enemy.player_damage.values().map(|(_status, dmg)| dmg).sum();

			EnemyTab {
				name: enemy.name.clone(),
				object_id: enemy.object_type,
				total_damage,
				players: {
					let mut player_rows: Vec<PlayerRow> = enemy
						.player_damage
						.iter()
						.filter(|(_p, (_status, dmg))| *dmg > 0)
						.map(|(&player_id, &(status, damage))| {
							let player = &this.players[&player_id];

							PlayerRow {
								name: player.name.clone(),
								is_self: player.is_self,
								status: match status {
									super::PlayerStatus::Present => ' ',
									super::PlayerStatus::Death => '🪦',
									super::PlayerStatus::Nexus => 'N',
								},
								damage,
								damage_percent: format!(
									"{:.2}",
									100.0 * damage as f64 / total_damage as f64
								),
								items: player.items.clone().map(|item| {
									item.map(|item| PlayerItem {
										id: item.id,
										name: this
											.rotmguard
											.assets
											.objects
											.get(&item.id)
											.map(|obj| obj.name.as_str())
											.unwrap_or("undefined item")
											.to_owned(),
										enchantments: item
											.enchantments
											.iter()
											.map(|&ench_id| {
												this.rotmguard
													.assets
													.enchantments
													.get(&(ench_id as u32))
													.map(|ench| ench.name.as_str())
													.unwrap_or("undefined enchantment")
													.to_owned()
											})
											.collect(),
									})
								}),
							}
						})
						.collect();

					player_rows.sort_by_key(|p| -p.damage); // negative to make it descending

					player_rows
				},
			}
		})
		.filter(|e| e.total_damage > TAKEN_DAMAGE_CRITERIA)
		.collect();

	enemy_tabs.sort_by_key(|e| -e.total_damage); // negative to make it descending

	let mut all_items: BTreeMap<u32, Option<String>> = this
		.players
		.iter()
		.map(|(_, p)| {
			p.items
				.iter()
				.filter_map(|item| item.as_ref().map(|item| item.id))
		})
		.flatten()
		.map(|item_id| (item_id, None))
		.collect();
	all_items.iter_mut().for_each(|(item_id, sprite)| {
		*sprite = this
			.rotmguard
			.assets
			.try_get_obj_sprite(*item_id)
			.map(|raw_sprite| BASE64_STANDARD.encode(raw_sprite))
	});

	let mut all_enemies: BTreeMap<u32, Option<String>> = enemy_tabs
		.iter()
		.map(|enemy| (enemy.object_id, None))
		.collect();
	all_enemies.iter_mut().for_each(|(enemy_id, sprite)| {
		*sprite = this
			.rotmguard
			.assets
			.try_get_obj_sprite(*enemy_id)
			.map(|raw_sprite| BASE64_STANDARD.encode(raw_sprite))
	});

	Some(Report {
		map_name,
		time: format!("{}", chrono::Local::now().format("%F %T")),
		enemy_tabs,
		all_items,
		all_enemies,
	})
}

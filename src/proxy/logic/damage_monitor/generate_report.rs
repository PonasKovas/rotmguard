use super::TAKEN_DAMAGE_CRITERIA;
use crate::{
	proxy::Proxy,
	util::{GREEN, RED, create_notification},
};
use anyhow::Result;
use askama::Template;
use base64::{Engine, prelude::BASE64_STANDARD};
use std::{collections::BTreeMap, sync::OnceLock};
use tracing::error;

#[derive(Template)]
#[template(path = "damage_report.html")]
struct Report<'a> {
	map_name: &'a str,
	enemy_tabs: Vec<EnemyTab<'a>>,
	all_items: BTreeMap<u32, Option<String>>, // item id -> item sprite base64
	all_enemies: BTreeMap<u32, Option<String>>, // enemy object id -> item sprite base64
}

struct EnemyTab<'a> {
	name: &'a str,
	object_id: u32,
	total_damage: i64,
	players: Vec<PlayerRow<'a>>,
}

struct PlayerRow<'a> {
	name: &'a str,
	is_self: bool,
	status: char,
	damage: i64,
	damage_percent: String,
	items: [Option<PlayerItem<'a>>; 4],
}

struct PlayerItem<'a> {
	id: u32,
	name: &'a str,
	enchantments: Vec<&'a str>,
}

pub async fn generate_report(proxy: &mut Proxy) {
	if let Err(e) = inner(proxy).await {
		error!("{e}");
		proxy
			.send_client(create_notification(&format!("{e}"), RED))
			.await;
	}
}

async fn inner(proxy: &mut Proxy) -> Result<()> {
	let report = Report::new(proxy);
	let page = report.render()?;

	let id = proxy.rotmguard.damage_monitor_http.add_page(page);

	let port = proxy.rotmguard.damage_monitor_http.port;
	let url = format!("http://127.0.0.1:{port}/{id}");
	if proxy.rotmguard.config.settings.open_browser {
		webbrowser::open(&url)?;
	}
	proxy
		.send_client(create_notification(
			&format!("Report generated. Open at\n{url}"),
			GREEN,
		))
		.await;

	Ok(())
}

impl<'a> Report<'a> {
	fn new(proxy: &'a mut Proxy) -> Self {
		let this = &proxy.state.damage_monitor;

		// sort enemies by total damage done to them
		let mut enemy_tabs: Vec<EnemyTab> = this
			.enemies
			.iter()
			.map(|(_enemy_id, enemy)| {
				let total_damage = enemy.player_damage.values().sum();

				EnemyTab {
					name: &enemy.name,
					object_id: enemy.object_type,
					total_damage,
					players: {
						let mut player_rows: Vec<PlayerRow> = enemy
							.player_damage
							.iter()
							.filter(|(_p, dmg)| **dmg > 0)
							.map(|(&player_id, &damage)| {
								let player = &this.players[player_id];

								PlayerRow {
									name: &player.name,
									is_self: player.is_self,
									status: match player.status {
										super::PlayerStatus::Present => ' ',
										super::PlayerStatus::Death => '🪦',
										super::PlayerStatus::Nexus => 'N',
									},
									damage,
									damage_percent: format!(
										"{:.2}",
										100.0 * damage as f64 / total_damage as f64
									),
									items: player.items.map(|item| {
										item.item_id.map(|item_id| PlayerItem {
											id: item_id,
											name: proxy
												.rotmguard
												.assets
												.objects
												.get(&item_id)
												.map(|obj| obj.name.as_str())
												.unwrap_or("undefined item"),
											enchantments: item
												.enchantments
												.iter()
												.filter_map(|x| *x)
												.map(|ench_id| {
													proxy
														.rotmguard
														.assets
														.enchantments
														.get(&(ench_id as u32))
														.map(|ench| ench.name.as_str())
														.unwrap_or("undefined enchantment")
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

		enemy_tabs.sort_by_key(|e| -e.total_damage); // again, negative to make it descending

		let mut all_items: BTreeMap<u32, Option<String>> = this
			.players
			.iter()
			.map(|(_, p)| p.items.iter().filter_map(|item| item.item_id))
			.flatten()
			.map(|item_id| (item_id, None))
			.collect();
		all_items.iter_mut().for_each(|(item_id, sprite)| {
			*sprite = proxy
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
			*sprite = proxy
				.rotmguard
				.assets
				.try_get_obj_sprite(*enemy_id)
				.map(|raw_sprite| BASE64_STANDARD.encode(raw_sprite))
		});

		Self {
			map_name: &this.map_name,
			enemy_tabs,
			all_items,
			all_enemies,
		}
	}
}

fn icon() -> &'static str {
	static ICON: OnceLock<String> = OnceLock::new();

	ICON.get_or_init(|| {
		BASE64_STANDARD.encode(include_bytes!(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/assets/icon.png"
		)))
	})
}

fn undefined_sprite() -> &'static str {
	static UNDEFINED_SPRITE: OnceLock<String> = OnceLock::new();

	UNDEFINED_SPRITE.get_or_init(|| {
		BASE64_STANDARD.encode(include_bytes!(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/assets/undefined_sprite.png"
		)))
	})
}

fn time() -> String {
	format!("{}", chrono::Local::now().format("%F %T"))
}

fn format_number(n: i64) -> String {
	match n.abs() {
		..1_000 => n.to_string(),
		1_000..100_000 => format!("{:.1}K", n as f64 / 1_000.0),
		100_000..1_000_000 => format!("{:.0}K", n as f64 / 1_000.0),
		1_000_000..100_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
		100_000_000..1_000_000_000 => format!("{:.0}M", n as f64 / 1_000_000.0),
		1_000_000_000..100_000_000_000 => format!("{:.1}G", n as f64 / 1_000_000_000.0),
		_ => format!("{:.0}G", n as f64 / 1_000_000_000.0),
	}
}

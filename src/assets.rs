use crate::config::Config;
use anyhow::{Context, bail};
use either::Either;
use nix::NixPath;
use process::ReverseChangesGuard;
use raw_parse::RawAssets;
use std::collections::{BTreeMap, HashMap};
use tracing::info;

mod process;
mod raw_parse;

pub struct Assets {
	pub sprites: Sprites,
	pub objects: HashMap<u32, Object>,
	pub enchantments: HashMap<u32, Enchantment>,
	pub tiles: HashMap<u32, Tile>,
	/// Reverses the changes to assets file on drop
	reverse_changes_guard: Option<ReverseChangesGuard>,
}

pub struct Sprites {
	pub animated_spritesheets: HashMap<String, Spritesheet>,
	pub spritesheets: HashMap<String, Spritesheet>,
}

// mapping sprite id to actual sprite PNG
pub type Spritesheet = BTreeMap<u32, Vec<u8>>;

pub struct Object {
	pub name: String,
	pub sprite: Option<SpriteId>,
	/// projectile type -> projectile data
	pub projectiles: BTreeMap<u8, ProjectileInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ProjectileInfo {
	// either precise damage or a range
	pub damage: Either<i32, (i32, i32)>,
	pub armor_piercing: bool,
	// bitflags
	pub inflicts_condition: u64,
	pub inflicts_condition2: u64,
}

pub struct SpriteId {
	pub is_animated: bool,
	pub spritesheet: String,
	pub index: u32,
}

pub struct Enchantment {
	pub name: String,
	pub effects: Vec<EnchantmentEffect>,
}

pub enum EnchantmentEffect {
	FlatLifeRegen(f32),
	PercentageLifeRegen(f32),
	Other, // not particularly interested in the gazillion other enchantments
}

pub struct Tile {
	pub name: String,
	pub damage: Option<i16>,
	pub is_conveyor: bool,
}

impl Assets {
	pub fn try_get_obj_sprite(&self, object_id: u32) -> Option<&Vec<u8>> {
		let obj_sprite_id = &self.objects.get(&object_id)?.sprite.as_ref()?;
		let spritesheets = if obj_sprite_id.is_animated {
			&self.sprites.animated_spritesheets
		} else {
			&self.sprites.spritesheets
		};
		let spritesheet = spritesheets.get(&obj_sprite_id.spritesheet)?;
		let sprite = spritesheet.get(&obj_sprite_id.index)?;

		Some(sprite)
	}
}

pub fn handle_assets(config: &Config) -> anyhow::Result<Assets> {
	if config.assets_res.is_empty() {
		bail!("assets_res not set. Please edit your rotmguard.toml!",);
	}

	let raw_assets = RawAssets::parse(&config.assets_res).context("parsing resources.assets")?;
	info!("Assets file parsed. Processing...");
	let assets = Assets::process(config, raw_assets).context("processing resources.assets")?;

	Ok(assets)
}

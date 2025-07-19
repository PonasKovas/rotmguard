use super::{XMLUtility, parse_id};
use crate::{assets::Tile, config::Config};
use anyhow::{Context, Result};
use std::collections::HashMap;
use xmltree::Element;

// returns true if any modifications were made
pub fn parse(config: &Config, tiles: &mut HashMap<u32, Tile>, xml: &mut Element) -> Result<bool> {
	let mut modified = false;

	for (i, ground) in xml.child_elements().enumerate() {
		if ground.name != "Ground" {
			// ONLY INTERESTED IN GROUND TYPES!
			continue;
		}

		modified |= parse_ground(config, tiles, ground).with_context(|| format!("ground {i}"))?;
	}

	Ok(modified)
}

fn parse_ground(
	_config: &Config,
	tiles: &mut HashMap<u32, Tile>,
	ground: &mut Element,
) -> Result<bool> {
	// parse the goofy ass ground type
	let ground_type_str = ground.attributes.get("type").context("type attr")?;
	let ground_type = parse_id(ground_type_str)?;

	let ground_id = ground.attributes.get("id").context("id attr")?;

	let is_conveyor = ground.get_child("Push").is_some();
	let damage = ground
		.get_child_text("MaxDamage")
		.or_else(|| ground.get_child_text("Damage"))
		.map(|s| s.parse().context("damage"))
		.transpose()?;

	tiles.insert(
		ground_type,
		Tile {
			name: ground_id.to_owned(),
			damage,
			is_conveyor,
		},
	);

	Ok(false)
}

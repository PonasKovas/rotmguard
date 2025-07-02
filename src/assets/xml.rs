use super::Assets;
use crate::config::Config;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use xmltree::{Element, XMLNode};

mod enemy_projectiles;

pub fn process_xml(
	config: &Config,
	assets: &mut Assets,
	mut xml: Element,
	raw_slice: &mut [u8],
) -> Result<()> {
	match xml.name.as_str() {
		"Objects" => objects(config, assets, &mut xml.children)?,
		"GroundTypes" => grounds(config, assets, &mut xml.children)?,
		_ => return Ok(()), // Not Interested 👍
	}

	if config.settings.edit_assets.enabled {
		let mut edited_xml = Vec::with_capacity(raw_slice.len());

		xml.write(&mut edited_xml)?;

		if edited_xml.len() > raw_slice.len() {
			bail!("Modified XML can not be longer than original.");
		}
		// add spaces to the end to make sure old and edited XMLs have the same
		// length to not fuck up the rest of the file
		edited_xml.resize(raw_slice.len(), b' ');

		raw_slice.copy_from_slice(&edited_xml);
	}

	Ok(())
}

fn objects(config: &Config, assets: &mut Assets, objects: &mut [XMLNode]) -> Result<()> {
	for (i, object) in objects.iter_mut().enumerate() {
		let object = match object {
			XMLNode::Element(object) => object,
			_ => continue,
		};

		if object.name != "Object" {
			// Again, ONLY INTERESTED IN OBJECTS!
			continue;
		}

		// parse the goofy ass object type
		let object_type_str = object
			.attributes
			.get("type")
			.context(format!("object {i} has no 'type' attr"))?;
		let object_type_str = object_type_str
			.strip_prefix("0x")
			.context(format!("object {i} 'type' attr doesnt start with 0x"))?;
		let object_type = u32::from_str_radix(object_type_str, 16)
			.context(format!("unexpected object {i} type format"))?;

		let mut projectiles = BTreeMap::new();
		enemy_projectiles::handle_projectiles(config, object, &mut projectiles)
			.context("enemy projectiles")?;

		if !projectiles.is_empty() {
			// save
			assets.projectiles.insert(object_type, projectiles);
		}
	}

	Ok(())
}

fn grounds(_config: &Config, assets: &mut Assets, grounds: &mut [XMLNode]) -> Result<()> {
	for (i, ground) in grounds.iter_mut().enumerate() {
		let ground = match ground {
			XMLNode::Element(ground) => ground,
			_ => continue,
		};

		if ground.name != "Ground" {
			// ONLY INTERESTED IN GROUND TYPES!
			continue;
		}

		// parse the goofy ass ground type
		let ground_type_str = ground
			.attributes
			.get("type")
			.context(format!("ground {i} has no 'type' attr"))?;
		let ground_type_str = ground_type_str
			.strip_prefix("0x")
			.context(format!("ground {i} 'type' attr doesnt start with 0x"))?;
		let ground_type = u16::from_str_radix(ground_type_str, 16)
			.context(format!("unexpected ground {i} type format"))?;

		for param in &ground.children {
			let param = match param {
				XMLNode::Element(param) => param,
				_ => continue,
			};

			if param.name == "MaxDamage" {
				if param.children.is_empty() || param.children.len() > 1 {
					bail!("Invalid Ground MaxDamage. Must have only text");
				}

				let damage = match &param.children[0] {
					XMLNode::Text(dmg) => dmg,
					_ => bail!("Invalid Ground MaxDamage. Value be text"),
				};

				let damage = damage
					.parse::<i64>()
					.context("Invalid Ground MaxDamage, must be integer")?;

				assets.hazardous_tiles.insert(ground_type, damage);
			}

			if param.name == "Push" {
				assets.conveyor_tiles.insert(ground_type);
			}
		}
	}

	Ok(())
}

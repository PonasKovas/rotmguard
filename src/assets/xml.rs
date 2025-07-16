use super::{Assets, Enchantment, EnchantmentEffect, Object};
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
		"Enchantments" => enchantments(config, assets, &mut xml.children)?,
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
			// Again, ONLY INTERESTED IN OBJECTS!!!!
			continue;
		}

		// parse the goofy ass object type format... 0xRETARDED
		let object_type_str = object
			.attributes
			.get("type")
			.context(format!("object {i} has no 'type' attr"))?;
		let object_type = parse_id(object_type_str)?;

		let mut object_data = Object {
			name: object.attributes.get("id").context("object id")?.to_owned(),
			is_animated: false,
			sprite: (String::new(), 0),
			projectiles: BTreeMap::new(),
		};
		let mut projectile_i = 0; // projectiles dont always have ids, so we keep a counter ourselves
		for parameter in &mut object.children {
			let parameter = match parameter {
				XMLNode::Element(p) => p,
				_ => continue,
			};
			match parameter.name.as_str() {
				"Projectile" => {
					let (id, projectile) =
						enemy_projectiles::parse(config, parameter).context("enemy projectile")?;

					let id = id.unwrap_or(projectile_i);
					projectile_i += 1;

					if object_data.projectiles.insert(id, projectile).is_some() {
						bail!("duplicate projectile id: {object:?}");
					}
				}
				"DisplayId" => {
					object_data.name = get_inner_text(parameter)?.to_owned();
				}
				"AnimatedTexture" | "Texture" => {
					object_data.is_animated = "AnimatedTexture" == parameter.name;
					let spritesheet = get_inner_text(
						parameter
							.get_child("File")
							.context("AnimatedTexture->File")?,
					)?;
					let index = get_inner_text(
						parameter
							.get_child("Index")
							.context("AnimatedTexture->Index")?,
					)?;
					let index = parse_id(index)?;

					object_data.sprite = (spritesheet.to_owned(), index);
				}
				_ => {}
			}
		}

		assets.objects.insert(object_type, object_data);
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
		let ground_type = parse_id(ground_type_str)?;

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

fn enchantments(_config: &Config, assets: &mut Assets, enchantments: &mut [XMLNode]) -> Result<()> {
	for (i, enchantment) in enchantments.iter_mut().enumerate() {
		let enchantment = match enchantment {
			XMLNode::Element(enchantment) => enchantment,
			_ => continue,
		};

		if enchantment.name != "Enchantment" {
			continue;
		}

		// parse the goofy ass ground type
		let enchantment_type_str = enchantment
			.attributes
			.get("type")
			.context(format!("enchantment {i} has no 'type' attr"))?;
		let enchantment_type = parse_id(enchantment_type_str)?;

		let name = get_inner_text(
			enchantment
				.get_child("DisplayId")
				.context("Enchantment->DisplayId")?,
		)?;

		let mutators = enchantment
			.get_child("Mutators")
			.context("Enchantment->Mutators")?;

		let mut effect = None;

		for mutator in &mutators.children {
			let mutator = match mutator {
				XMLNode::Element(mutator) => mutator,
				_ => continue,
			};

			let mutator_type = get_inner_text(mutator)?;
			effect = Some(match mutator_type {
				"FlatRegen" => {
					let stat = mutator
						.attributes
						.get("stat")
						.context("FlatRegen stat attr")?;
					let amount = mutator
						.attributes
						.get("amount")
						.context("FlatRegen amount attr")?
						.parse::<f32>()?;

					if stat == "HP" {
						EnchantmentEffect::FlatLifeRegen(amount)
					} else {
						EnchantmentEffect::Other
					}
				}
				"PercentageRegen" => {
					let stat = mutator
						.attributes
						.get("stat")
						.context("PercentageRegen stat attr")?;
					let amount = mutator
						.attributes
						.get("amount")
						.context("PercentageRegen amount attr")?
						.parse::<f32>()?;

					if stat == "HP" {
						EnchantmentEffect::PercentageLifeRegen(amount)
					} else {
						EnchantmentEffect::Other
					}
				}
				_ => EnchantmentEffect::Other, // not interested in others at this point
			});
		}

		let enchantment_data = Enchantment {
			name: name.to_owned(),
			effect: effect.context("enchantment with no mutators?")?,
		};

		assets
			.enchantments
			.insert(enchantment_type, enchantment_data);
	}

	Ok(())
}

fn get_inner_text(element: &Element) -> Result<&str> {
	if element.children.is_empty() {
		return Ok("");
	}

	if element.children.len() > 1 {
		bail!("Invalid element. Must have only text inside. {element:?}");
	}

	let text = match &element.children[0] {
		XMLNode::Text(text) => text,
		_ => bail!("Invalid element. Value be text. {element:?}"),
	};

	Ok(text)
}

fn parse_id(id: &str) -> Result<u32> {
	if id.starts_with("0x") {
		let id_stripped = id.strip_prefix("0x").unwrap();
		let id =
			u32::from_str_radix(id_stripped, 16).context(format!("unexpected {id:?} format"))?;

		Ok(id)
	} else {
		Ok(id.parse().context(format!("unexpected {id:?} format"))?)
	}
}

use super::{XMLUtility, parse_id};
use crate::{
	assets::{Object, ProjectileInfo, SpriteId},
	config::Config,
	util::{CONDITION_BITFLAG, CONDITION2_BITFLAG},
};
use anyhow::{Context, Result, bail};
use either::Either;
use std::collections::{BTreeMap, HashMap};
use xmltree::{Element, XMLNode};

// returns true if any modifications were made
pub fn parse(
	config: &Config,
	objects: &mut HashMap<u32, Object>,
	xml: &mut Element,
) -> Result<bool> {
	let mut modified = false;

	for (i, object) in xml.child_elements().enumerate() {
		if object.name != "Object" {
			// ONLY INTERESTED IN OBJECTS!!!!
			continue;
		}

		modified |= parse_object(config, objects, object).with_context(|| format!("object {i}"))?;
	}

	Ok(modified)
}

fn parse_object(
	config: &Config,
	objects: &mut HashMap<u32, Object>,
	object: &mut Element,
) -> Result<bool> {
	let mut modified = false;

	// parse the goofy ass object type format... 0xRETARDED
	let object_type_str = object.attributes.get("type").context("type attr")?;
	let object_type = parse_id(object_type_str)?;

	// for name, prefer to use DisplayId, but if not present just use ID
	let name = match object.get_child_text("DisplayId") {
		Some(text) => text.into_owned(),
		None => object.attributes.get("id").context("object id")?.to_owned(),
	};

	let sprite = match (
		object.get_child("AnimatedTexture"),
		object.get_child("Texture"),
	) {
		(Some(texture), None) | (None, Some(texture)) => {
			let spritesheet = texture
				.get_child_text("File")
				.context("Texture File")?
				.into_owned();

			let index = texture.get_child_text("Index").context("Texture Index")?;
			let index = parse_id(&*index)?;

			Some(SpriteId {
				is_animated: object.get_child("AnimatedTexture").is_some(),
				spritesheet,
				index,
			})
		}
		_ => None,
	};

	let mut projectiles = BTreeMap::new();

	let mut projectile_i = 0; // projectiles dont always have ids, so we keep a counter ourselves
	for parameter in object.child_elements() {
		if parameter.name == "Projectile" {
			let (modded, id, projectile) = parse_projectile(config, parameter)
				.with_context(|| format!("projectile {projectile_i}"))?;

			modified |= modded;

			let id = id.unwrap_or(projectile_i);
			projectile_i += 1;

			if projectiles.insert(id, projectile).is_some() {
				bail!("duplicate projectile id {id}");
			}
		}
	}

	let object_data = Object {
		name,
		sprite,
		projectiles,
	};

	objects.insert(object_type, object_data);

	Ok(modified)
}

fn parse_projectile(
	config: &Config,
	projectile: &mut Element,
) -> Result<(bool, Option<u8>, ProjectileInfo)> {
	let mut modified = false;

	let projectile_id = match projectile.attributes.get("id") {
		Some(s) => Some(s.parse::<u8>().context("Projectile id non-integer")?),
		None => None,
	};

	let damage = match (
		projectile.get_child_text("Damage"),
		projectile.get_child_text("MinDamage"),
		projectile.get_child_text("MaxDamage"),
	) {
		(Some(exact_dmg), None, None) => Either::Left(exact_dmg.parse().context("Damage")?),
		(None, Some(min), Some(max)) => Either::Right((
			min.parse().context("MinDamage")?,
			max.parse().context("MinDamage")?,
		)),
		_ => bail!("invalid damage"),
	};

	let mut armor_piercing = false;
	let mut inflicts_condition = 0;
	let mut inflicts_condition2 = 0;

	// iterating using indexes instead of directly, because we will modify as we go (removing debuffs)
	// so its also important to start from the end
	for projectile_parameter_i in (0..projectile.children.len()).rev() {
		let projectile_parameter = match &projectile.children[projectile_parameter_i] {
			XMLNode::Element(p) => p,
			_ => continue,
		};

		match projectile_parameter.name.as_str() {
			"ArmorPiercing" => {
				armor_piercing = true;
			}
			"ConditionEffect" => {
				let condition = &*projectile_parameter.get_text().with_context(|| {
					format!("parameter {projectile_parameter_i} ConditionEffect text")
				})?;

				match condition {
					"Curse" => {
						inflicts_condition2 |= CONDITION2_BITFLAG::CURSED;
					}
					"Exposed" => {
						inflicts_condition2 |= CONDITION2_BITFLAG::EXPOSED;
					}
					"Sick" => {
						inflicts_condition |= CONDITION_BITFLAG::SICK;
					}
					"Bleeding" => {
						inflicts_condition |= CONDITION_BITFLAG::BLEEDING;
					}
					"Armor Broken" => {
						inflicts_condition |= CONDITION_BITFLAG::ARMOR_BROKEN;
					}
					"Weak" => {
						inflicts_condition |= CONDITION_BITFLAG::WEAK;
					}
					_ => {} // may want to handle more later
				}

				// Client-side debuffs for force antidebuff
				if config.settings.edit_assets.force_debuffs {
					let debuffs = &config.settings.debuffs;
					let c = condition;
					if (c == "Blind" && debuffs.blind)
						|| (c == "Hallucinating" && debuffs.hallucinating)
						|| (c == "Drunk" && debuffs.drunk)
						|| (c == "Confused" && debuffs.confused)
						|| (c == "Unstable" && debuffs.unstable)
						|| (c == "Darkness" && debuffs.darkness)
					{
						projectile.children.remove(projectile_parameter_i);
						modified = true;
					}
				}
			}
			_ => {}
		}
	}

	Ok((
		modified,
		projectile_id,
		ProjectileInfo {
			damage,
			armor_piercing,
			inflicts_condition,
			inflicts_condition2,
		},
	))
}

use super::{XMLUtility, parse_id};
use crate::{
	assets::{Object, ProjectileCondition, ProjectileInfo, SpriteId},
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
	let mut subattacks = Vec::new();

	let mut projectile_i = 0; // projectiles dont always have ids, so we keep a counter ourselves
	for parameter in object.child_elements() {
		match parameter.name.as_str() {
			"Projectile" => {
				let (modded, id, projectile) = parse_projectile(config, parameter)
					.with_context(|| format!("projectile {projectile_i}"))?;

				modified |= modded;

				let id = id.unwrap_or(projectile_i);
				projectile_i += 1;

				if projectiles.insert(id, projectile).is_some() {
					bail!("duplicate projectile id {id}");
				}
			}
			"Subattack" => {
				let projectile_id = parse_subattack(parameter)
					.with_context(|| format!("subattack {}", subattacks.len()))?;

				subattacks.push(projectile_id);
			}
			_ => {}
		}
	}

	let projectiles = if subattacks.is_empty() {
		projectiles
	} else {
		subattacks
			.into_iter()
			.enumerate()
			.map(|(i, projectile_id)| {
				projectiles
					.get(&projectile_id)
					.map(|proj| (i as u8, proj.clone()))
			})
			.collect::<Option<BTreeMap<_, _>>>()
			.context("subattack invalid ProjectileId")?
	};

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
	let mut inflicts = Vec::new();

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
				let condition_name = &*projectile_parameter.get_text().with_context(|| {
					format!("parameter {projectile_parameter_i} ConditionEffect text")
				})?;
				let duration = match projectile_parameter.attributes.get("duration") {
					Some(d) => d,
					// bruh
					None => match projectile_parameter.attributes.get("effect") {
						Some(d) => d,
						None => bail!(
							"parameter {projectile_parameter_i} neither duration nor effect param found"
						),
					},
				};

				// Client-side debuffs for force antidebuff
				if config.settings.edit_assets.force_debuffs {
					let debuffs = &config.settings.debuffs;
					let c = condition_name;
					if (c == "Blind" && debuffs.blind)
						|| (c == "Hallucinating" && debuffs.hallucinating)
						|| (c == "Drunk" && debuffs.drunk)
						|| (c == "Confused" && debuffs.confused)
						|| (c == "Unstable" && debuffs.unstable)
						|| (c == "Darkness" && debuffs.darkness)
					{
						projectile.children.remove(projectile_parameter_i);
						modified = true;
						continue;
					}
				}

				let mut condition = ProjectileCondition::default();
				condition.duration = duration
					.parse()
					.with_context(|| format!("parameter {projectile_parameter_i} duration"))?;

				match condition_name {
					"Curse" => {
						condition.condition2 |= CONDITION2_BITFLAG::CURSED;
					}
					"Exposed" => {
						condition.condition2 |= CONDITION2_BITFLAG::EXPOSED;
					}
					"Sick" => {
						condition.condition |= CONDITION_BITFLAG::SICK;
					}
					"Bleeding" => {
						condition.condition |= CONDITION_BITFLAG::BLEEDING;
					}
					"Armor Broken" => {
						condition.condition |= CONDITION_BITFLAG::ARMOR_BROKEN;
					}
					"Weak" => {
						condition.condition |= CONDITION_BITFLAG::WEAK;
					}
					_ => {} // may want to handle more later
				}

				inflicts.push(condition);
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
			inflicts,
		},
	))
}

fn parse_subattack(subattack: &mut Element) -> Result<u8> {
	let projectile_id = subattack
		.attributes
		.get("projectileId")
		.context("subattack has no projectileId")?
		.parse::<u8>()
		.context("subattack projectile id non-integer")?;

	Ok(projectile_id)
}

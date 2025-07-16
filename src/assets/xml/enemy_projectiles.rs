use super::get_inner_text;
use crate::{assets::ProjectileInfo, config::Config};
use anyhow::{Context, Result, bail};
use either::Either;
use xmltree::{Element, XMLNode};

// parses a projectile
// And also removes debuffs if force_debuffs enabled
pub fn parse(config: &Config, projectile: &mut Element) -> Result<(Option<u8>, ProjectileInfo)> {
	let projectile_id = match projectile.attributes.get("id") {
		Some(s) => Some(s.parse::<u8>().context("Projectile id non-integer")?),
		None => None,
	};

	let mut exact_damage = None;
	let mut min_damage = None;
	let mut max_damage = None;
	let mut armor_piercing = false;
	let mut inflicts_cursed = false;
	let mut inflicts_exposed = false;
	let mut inflicts_sick = false;
	let mut inflicts_bleeding = false;
	let mut inflicts_armor_broken = false;

	// iterating using indexes instead of directly, because we will modify as we go (removing debuffs)
	// so its also important to start from the end
	for projectile_parameter_i in (0..projectile.children.len()).rev() {
		let projectile_parameter = match &projectile.children[projectile_parameter_i] {
			XMLNode::Element(p) => p,
			_ => continue,
		};

		match projectile_parameter.name.as_str() {
			"Damage" => {
				let dmg = get_inner_text(projectile_parameter)?;
				let dmg = dmg.parse::<i32>()?;
				if exact_damage.replace(dmg).is_some() {
					bail!("twice <Damage> in object projectile. {projectile:?}");
				}
			}
			"MinDamage" => {
				let dmg = get_inner_text(projectile_parameter)?;
				let dmg = dmg.parse::<i32>()?;
				if min_damage.replace(dmg).is_some() {
					bail!("twice <MinDamage> in object projectile. {projectile:?}");
				}
			}
			"MaxDamage" => {
				let dmg = get_inner_text(projectile_parameter)?;
				let dmg = dmg.parse::<i32>()?;
				if max_damage.replace(dmg).is_some() {
					bail!("twice <MaxDamage> in object projectile. {projectile:?}");
				}
			}
			"ArmorPiercing" => {
				armor_piercing = true;
			}
			"ConditionEffect" => {
				let condition = get_inner_text(projectile_parameter)?;
				match condition {
					"Curse" => {
						inflicts_cursed = true;
					}
					"Exposed" => {
						inflicts_exposed = true;
					}
					"Sick" => {
						inflicts_sick = true;
					}
					"Bleeding" => {
						inflicts_bleeding = true;
					}
					"Armor Broken" => {
						inflicts_armor_broken = true;
					}
					_ => {}
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
					}
				}
			}
			_ => {}
		}
	}

	let damage = match (exact_damage, min_damage, max_damage) {
		(Some(dmg), None, None) => Either::Left(dmg),
		(None, Some(min), Some(max)) => Either::Right((min, max)),
		_ => bail!("invalid combination of damage for projectile: {projectile:?}"),
	};

	Ok((
		projectile_id,
		ProjectileInfo {
			damage,
			armor_piercing,
			inflicts_cursed,
			inflicts_exposed,
			inflicts_sick,
			inflicts_bleeding,
			inflicts_armor_broken,
		},
	))
}

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use xmltree::{Element, XMLNode};

use crate::{assets::ProjectileInfo, config::Config};

// Adds all projectiles of this object to the given BTreeMap
// And also removes debuffs if force_debuffs enabled
pub fn handle_projectiles(
	config: &Config,
	object: &mut Element,
	projectiles: &mut BTreeMap<u32, ProjectileInfo>,
) -> Result<()> {
	let mut i = 0;
	for parameter in &mut object.children {
		let parameter = match parameter {
			XMLNode::Element(p) => p,
			_ => continue,
		};
		if parameter.name != "Projectile" {
			continue;
		}
		let projectile_id = match parameter.attributes.get("id") {
			Some(s) => s.parse::<u32>().context("Projectile id non-integer")?,
			None => i,
		};

		let mut armor_piercing = false;
		let mut inflicts_cursed = false;
		let mut inflicts_exposed = false;
		let mut inflicts_sick = false;
		let mut inflicts_bleeding = false;
		let mut inflicts_armor_broken = false;
		// Iterate over all parameters starting from the end so we can remove the debuffs
		for projectile_parameter_i in (0..parameter.children.len()).rev() {
			let projectile_parameter = match &parameter.children[projectile_parameter_i] {
				XMLNode::Element(p) => p,
				_ => continue,
			};

			if projectile_parameter.name == "ArmorPiercing" {
				armor_piercing = true;
			}

			if projectile_parameter.name == "ConditionEffect" {
				if projectile_parameter.children.is_empty()
					|| projectile_parameter.children.len() > 1
				{
					bail!("Invalid Object Projectile ConditionEffect. Must have only text inside");
				}

				let condition = match &projectile_parameter.children[0] {
					XMLNode::Text(condition) => condition,
					_ => bail!("Invalid Object Projectile ConditionEffect. Value be text"),
				};

				match condition.as_str() {
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
					let c = condition.as_str();
					if (c == "Blind" && debuffs.blind)
						|| (c == "Hallucinating" && debuffs.hallucinating)
						|| (c == "Drunk" && debuffs.drunk)
						|| (c == "Confused" && debuffs.confused)
						|| (c == "Unstable" && debuffs.unstable)
						|| (c == "Darkness" && debuffs.darkness)
					{
						parameter.children.remove(projectile_parameter_i);
					}
				}
			}
		}

		projectiles.insert(
			projectile_id,
			ProjectileInfo {
				armor_piercing,
				inflicts_cursed,
				inflicts_exposed,
				inflicts_sick,
				inflicts_bleeding,
				inflicts_armor_broken,
			},
		);
		i += 1;
	}

	Ok(())
}

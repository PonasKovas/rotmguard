use super::{XMLUtility, parse_id};
use crate::{
	assets::{Enchantment, EnchantmentEffect},
	config::Config,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use xmltree::Element;

// returns true if any modifications were made
pub fn parse(
	config: &Config,
	enchantments: &mut HashMap<u32, Enchantment>,
	xml: &mut Element,
) -> Result<bool> {
	let mut modified = false;

	for (i, element) in xml.child_elements().enumerate() {
		if element.name != "Enchantment" {
			// ONLY INTERESTED IN ENCHANTMENTS!
			continue;
		}

		modified |= parse_enchantment(config, enchantments, element)
			.with_context(|| format!("enchantment {i}"))?;
	}

	Ok(modified)
}

fn parse_enchantment(
	config: &Config,
	enchantments: &mut HashMap<u32, Enchantment>,
	element: &mut Element,
) -> Result<bool> {
	// parse the goofy ass ground type
	let enchantment_type_str = element.attributes.get("type").context("type attr")?;
	let enchantment_type = parse_id(enchantment_type_str)?;

	let name = element
		.get_child_text("DisplayId")
		.context("DisplayId")?
		.into_owned();

	let mutators = element.get_mut_child("Mutators").context("Mutators")?;

	let mut effects = Vec::new();
	for (i, mutator) in mutators.child_elements().enumerate() {
		effects.push(parse_mutator(config, mutator).with_context(|| format!("Mutator {i}"))?);
	}

	let enchantment_data = Enchantment { name, effects };

	enchantments.insert(enchantment_type, enchantment_data);

	Ok(false)
}

fn parse_mutator(_config: &Config, mutator: &mut Element) -> Result<EnchantmentEffect> {
	let mutator_type = mutator.get_text().context("Mutator text")?;

	let stat = mutator.attributes.get("stat").context("stat attr");
	let amount = mutator
		.attributes
		.get("amount")
		.map(|x| x.parse::<f32>())
		.context("amount attr");

	let effect = match &*mutator_type {
		"FlatRegen" => match &**stat? {
			"HP" => EnchantmentEffect::FlatLifeRegen(amount??),
			_ => EnchantmentEffect::Other,
		},
		"PercentageRegen" => match &**stat? {
			"HP" => EnchantmentEffect::PercentageLifeRegen(amount??),
			_ => EnchantmentEffect::Other,
		},
		_ => EnchantmentEffect::Other,
	};

	Ok(effect)
}

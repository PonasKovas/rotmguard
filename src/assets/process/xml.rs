use super::{Assets, Enchantment, Object, Tile, modify::OverwriteRegion};
use crate::{assets::raw_parse::XmlAsset, config::Config};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::{borrow::Cow, collections::HashMap};
use tracing::debug;
use xmltree::Element;

mod enchantments;
mod objects;
mod tiles;

#[derive(Default)]
struct Processed {
	objects: HashMap<u32, Object>,
	enchantments: HashMap<u32, Enchantment>,
	tiles: HashMap<u32, Tile>,

	to_overwrite: Vec<OverwriteRegion>,
}

pub fn process_xml(
	config: &Config,
	assets: &mut Assets,
	xml_assets: Vec<XmlAsset>,
) -> Result<Vec<OverwriteRegion>> {
	let res: Result<Processed> = xml_assets
		.into_par_iter()
		.try_fold(
			|| Processed::default(),
			|mut processed, xml| {
				// try parsing the file as XML
				let mut parsed_xml = match Element::parse(xml.data.as_slice()) {
					Ok(x) => x,
					Err(e) => {
						// not all text assets are XML and thats fine. Its okay.
						debug!(
							"Skipping text object {:?} which is a TextAsset but not valid XML: {e:?}",
							xml.name
						);
						return Ok(processed);
					}
				};

				let modified = inner(config, &mut processed, &mut parsed_xml)
					.with_context(|| format!("{:?}", xml.name))?;

				if modified {
					// initialize a new buffer of the original size filled with spaces (which dont matter in xml)
					let mut edited_xml = vec![b' '; xml.original_size];
					let mut slice = edited_xml.as_mut_slice();

					// if the new xml is larger than the old one this will fail
					// because we are writing to a slice with a limited size
					parsed_xml
						.write(&mut slice)
						.context("writing modified xml")?;

					processed.to_overwrite.push(OverwriteRegion {
						position: xml.position,
						data: edited_xml,
					});
				}

				Ok(processed)
			},
		)
		.try_reduce(
			|| Processed::default(),
			|acc, processed| Ok(acc.sum(processed)),
		);

	let processed = res?;

	assets.objects = processed.objects;
	assets.enchantments = processed.enchantments;
	assets.tiles = processed.tiles;

	Ok(processed.to_overwrite)
}

impl Processed {
	fn sum(mut self, other: Self) -> Self {
		self.objects.extend(other.objects);
		self.enchantments.extend(other.enchantments);
		self.tiles.extend(other.tiles);
		self.to_overwrite.extend(other.to_overwrite);

		self
	}
}

// returns true if any modifications were made
pub fn inner(config: &Config, processed: &mut Processed, xml: &mut Element) -> Result<bool> {
	match xml.name.as_str() {
		"Objects" => objects::parse(config, &mut processed.objects, xml),
		"GroundTypes" => tiles::parse(config, &mut processed.tiles, xml),
		"Enchantments" => enchantments::parse(config, &mut processed.enchantments, xml),
		_ => Ok(false), // Not Interested 👍
	}
}

// utility functions

fn parse_id(id: &str) -> Result<u32> {
	(|| {
		if id.starts_with("0x") {
			let id_stripped = id.strip_prefix("0x").unwrap();
			let id = u32::from_str_radix(id_stripped, 16)?;

			Ok(id)
		} else {
			id.parse()
		}
	})()
	.with_context(|| format!("unexpected format: {id:?}"))
}
trait XMLUtility {
	fn get_child_text(&self, child_name: impl AsRef<str>) -> Option<Cow<str>>;
	fn child_elements(&mut self) -> impl Iterator<Item = &mut Self>;
}
impl XMLUtility for Element {
	fn get_child_text(&self, child_name: impl AsRef<str>) -> Option<Cow<str>> {
		self.get_child(child_name.as_ref())
			.map(|e| e.get_text())
			.flatten()
			.map(|txt| txt)
	}
	fn child_elements(&mut self) -> impl Iterator<Item = &mut Self> {
		self.children.iter_mut().filter_map(|child| match child {
			xmltree::XMLNode::Element(element) => Some(element),
			_ => None,
		})
	}
}

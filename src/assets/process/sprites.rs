//! process the spritesheetf data and prepare convenient PNG data for each sprite
//! parallelised with rayon

use super::{Sprites, Spritesheet, spritesheetf};
use crate::assets::raw_parse::{RawAssets, Texture2D};
use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};

impl Sprites {
	pub(super) fn process(raw_assets: &RawAssets) -> Result<Self> {
		let spritesheetf = spritesheetf::root_as_sprite_sheet_root(&raw_assets.spritesheetf)?;

		let spritesheets = spritesheetf
			.sprites()
			.context("spritesheetf->sprites")?
			.into_iter()
			.par_bridge()
			.map(|sheet| {
				let sheet_name = sheet.name().context("spritesheetf->sprites->name")?;
				let sprites = process_spritesheet(&raw_assets, &sheet)
					.with_context(|| format!("sheet {sheet_name}"))?;

				Ok((sheet_name.to_owned(), sprites))
			})
			.collect::<Result<_>>()?;

		let animated_spritesheets_tuples: Vec<_> = spritesheetf
			.animated_sprites()
			.context("spritesheetf->animated_sprites")?
			.into_iter()
			.par_bridge()
			.map(|animated_sprite| {
				let index = animated_sprite.index();

				let (name, image_data) = process_animated_sprite(&raw_assets, &animated_sprite)
					.with_context(|| format!("animated sprite {index}"))?;

				Ok(image_data.map(|data| (name.to_owned(), index, data)))
			})
			.filter_map(|res| res.transpose())
			.collect::<Result<_>>()?;

		let mut animated_spritesheets: HashMap<String, BTreeMap<u32, Vec<u8>>> = HashMap::new();
		for (name, index, data) in animated_spritesheets_tuples {
			animated_spritesheets
				.entry(name)
				.or_default()
				.insert(index, data);
		}

		Ok(Self {
			animated_spritesheets,
			spritesheets,
		})
	}
}

fn process_spritesheet(
	raw_assets: &RawAssets,
	spritesheet: &spritesheetf::SpriteSheet,
) -> Result<Spritesheet> {
	spritesheet
		.sprites()
		.context("spritesheetf->sprites->sprites")?
		.into_iter()
		.par_bridge()
		.map(|sprite| {
			let index = sprite.index() as u32;

			let texture_data =
				process_sprite(raw_assets, &sprite).with_context(|| format!("sprite {index}"))?;

			Ok(texture_data.map(|data| (index, data)))
		})
		.filter_map(|res| res.transpose())
		.collect()
}

fn process_animated_sprite<'a>(
	raw_assets: &RawAssets,
	animated_sprite: &'a spritesheetf::AnimatedSprite,
) -> Result<(&'a str, Option<Vec<u8>>)> {
	let name = animated_sprite.name().context("animated_sprite->name")?;

	let sprite = animated_sprite
		.sprite()
		.context("animated_sprite->sprite")?;

	let texture_data = process_sprite(raw_assets, &sprite).with_context(|| format!("{name}"))?;

	Ok((name, texture_data))
}

// returns None if atlas id some other than 2 (characters) or 4 (mapObjects)
fn process_sprite(
	raw_assets: &RawAssets,
	sprite: &spritesheetf::Sprite,
) -> Result<Option<Vec<u8>>> {
	let atlas_id = sprite.atlas_id();
	let position = sprite.position().context("position")?;

	let sheet = match atlas_id {
		2 => &raw_assets.characters,
		4 => &raw_assets.map_objects,
		_ => return Ok(None),
	};

	let w = position.w() as u32;
	let h = position.h() as u32;

	let x = position.x() as u32;
	// y is inverted bcs we need to flip the image upside down
	let y = sheet.height - position.y() as u32 - h;

	let subimage = extract_subimage(sheet, x, y, w, h)?;

	let mut png_data = Vec::new();
	let mut encoder = png::Encoder::new(&mut png_data, w, h);
	encoder.set_color(png::ColorType::Rgba);
	encoder.set_depth(png::BitDepth::Eight);
	let mut writer = encoder.write_header().unwrap();
	writer.write_image_data(&subimage).unwrap();
	writer.finish().unwrap();

	Ok(Some(png_data))
}

fn extract_subimage(image: &Texture2D, x: u32, y: u32, width: u32, height: u32) -> Result<Vec<u8>> {
	if x + width > image.width || y + height > image.height {
		bail!("Sub-image bounds are outside the original image dimensions.");
	}

	let mut sub_data = Vec::with_capacity((width * height) as usize * 4);

	let original_stride = image.width as usize * 4;
	let sub_image_stride = width as usize * 4;

	// reverse rows - swap the image upside down
	for row in (0..height).rev() {
		let start_index = original_stride * (y + row) as usize + x as usize * 4;

		sub_data.extend_from_slice(&image.data[start_index..(start_index + sub_image_stride)]);
	}

	Ok(sub_data)
}

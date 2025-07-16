use super::{Image, RawAssets, spritesheetf::Position};

impl RawAssets {
	// returns PNG encoded image
	pub fn get_sprite(&self, atlas_id: i64, pos: Position) -> Vec<u8> {
		let image = match atlas_id {
			2 => &self.characters,
			4 => &self.map_objects,
			other => {
				panic!("unknown atlas id {other}");
			}
		};

		let w = pos.w() as u32;
		let h = pos.h() as u32;
		let x = pos.x() as u32;
		let y = image.h - pos.y() as u32 - h;

		let subimage = extract_subimage(image, x, y, w, h).unwrap();

		let mut png_data = Vec::new();

		let mut encoder = png::Encoder::new(&mut png_data, w, h);
		encoder.set_color(png::ColorType::Rgba);
		encoder.set_depth(png::BitDepth::Eight);
		let mut writer = encoder.write_header().unwrap();
		writer.write_image_data(&subimage).unwrap();
		writer.finish().unwrap();

		png_data
	}
}

fn extract_subimage(
	image: &Image,
	x: u32,
	y: u32,
	w: u32,
	h: u32,
) -> Result<Vec<u8>, &'static str> {
	if x + w > image.w || y + h > image.h {
		return Err("Sub-image bounds are outside the original image dimensions.");
	}

	let mut sub_data = Vec::with_capacity((w * h) as usize * 4);

	let original_stride = image.w as usize * 4;
	let sub_image_stride = w as usize * 4;

	for row in (0..h).rev() {
		let start_index = original_stride * (y + row) as usize + x as usize * 4;

		sub_data.extend_from_slice(&image.data[start_index..(start_index + sub_image_stride)]);
	}

	Ok(sub_data)
}

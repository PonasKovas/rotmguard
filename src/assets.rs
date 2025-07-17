use crate::config::Config;
use anyhow::{Context, bail};
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use bytes::Buf;
use either::Either;
use nix::NixPath;
use reverse_changes::ReverseChangesGuard;
use std::{
	collections::{BTreeMap, BTreeSet, HashMap},
	io::{self, Cursor, Error, Read},
};
use tracing::{debug, info};
use xml::process_xml;

mod get_sprite;
mod reverse_changes;
#[allow(
	unused_imports,
	unsafe_op_in_unsafe_fn,
	dead_code,
	mismatched_lifetime_syntaxes
)]
mod spritesheetf;
mod xml;

const TEXT_ASSET: i32 = 49;
const TEXTURE2D_ASSET: i32 = 28;

pub struct Assets {
	pub spritesheets: HashMap<String, Spritesheet>,
	pub animated_spritesheets: HashMap<String, Spritesheet>,
	/// object type -> object data
	pub objects: HashMap<u32, Object>,
	/// enchantment type -> enchantment data
	pub enchantments: HashMap<u32, Enchantment>,
	/// ground type -> damage
	pub hazardous_tiles: BTreeMap<u32, i64>,
	/// grounds that push the player like conveyors
	pub conveyor_tiles: BTreeSet<u32>,
	/// Reverses the changes to assets file on drop
	reverse_changes_guard: Option<ReverseChangesGuard>,
}

// mapping sprite id to actual sprite PNG
pub type Spritesheet = BTreeMap<u32, Vec<u8>>;

pub struct Object {
	pub name: String,
	/// whether the sprite is from normal spritesheets or animated spritesheets
	pub is_animated: bool,
	/// spritesheet and index
	pub sprite: (String, u32),
	/// projectile type -> projectile data
	pub projectiles: BTreeMap<u8, ProjectileInfo>,
}

pub struct Enchantment {
	pub name: String,
	pub effect: EnchantmentEffect,
}

pub enum EnchantmentEffect {
	FlatLifeRegen(f32),
	PercentageLifeRegen(f32),
	Other, // not particularly interested in the gazillion other enchantments
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ProjectileInfo {
	// either precise damage or a range
	pub damage: Either<i32, (i32, i32)>,
	pub armor_piercing: bool,
	pub inflicts_cursed: bool,
	pub inflicts_exposed: bool,
	pub inflicts_sick: bool,
	pub inflicts_bleeding: bool,
	pub inflicts_armor_broken: bool,
}

#[derive(Default)]
struct RawAssets {
	spritesheetf: View,
	characters: Image,
	map_objects: Image,
	// name, view
	text_assets: Vec<(String, View)>,
}

#[derive(Default)]
struct Image {
	w: u32,
	h: u32,
	data: Vec<u8>,
}

#[derive(Default, Clone, Copy)]
struct View {
	pos: usize,
	len: usize,
}
impl View {
	fn get(self, file: &mut [u8]) -> &mut [u8] {
		&mut file[self.pos..(self.pos + self.len)]
	}
}

pub fn handle_assets(config: &Config) -> anyhow::Result<Assets> {
	if config.assets_res.is_empty() {
		bail!("assets_res not set. Please edit your rotmguard.toml!",);
	}

	// read the whole assets file into memory
	let mut file = Cursor::new(std::fs::read(&config.assets_res)?);

	let real_size = file.get_ref().len() as u64;

	file.read_exact(&mut [0; 4 * 2])?; // 2 ints
	let version = file.read_i32::<BigEndian>()?;
	file.read_exact(&mut [0; 4])?; // int
	let big_endian = file.read_u8()? != 0;
	file.read_exact(&mut [0; 3])?;
	let metadata_size = file.read_u32::<BigEndian>()? as u64;
	let file_size = file.read_u64::<BigEndian>()?;
	let data_offset = file.read_u64::<BigEndian>()?;
	file.read_i64::<BigEndian>()?; // unknown

	// Some wack ass sanity tests stole from somewhere
	if version > 100
		|| file_size > real_size
		|| metadata_size > real_size
		|| (version as u64) > real_size
		|| data_offset > real_size
		|| file_size < metadata_size
		|| file_size < data_offset
	{
		bail!("invalid assets file");
	}

	// NUL-terminated string LOL 😂
	read_nul_terminated_string(&mut file)?; // unity version

	if big_endian {
		in_endian::<BigEndian>(config, &mut file, data_offset)
	} else {
		in_endian::<LittleEndian>(config, &mut file, data_offset)
	}
}

fn in_endian<ORDER: ByteOrder>(
	config: &Config,
	file: &mut Cursor<Vec<u8>>,
	data_offset: u64,
) -> anyhow::Result<Assets> {
	file.read_u32::<ORDER>()?; // target_platform
	let enable_type_tree = file.read_u8()? != 0;

	if enable_type_tree {
		bail!("enable_type_tree not supported.");
	}

	// Types
	let types_count = file.read_u32::<ORDER>()? as usize;
	let mut types = vec![0; types_count];

	for t in types.iter_mut() {
		let class_id = file.read_i32::<ORDER>()?;
		file.read_exact(&mut [0; 1 + 2])?; // is_stripped_type + script_type_index
		if class_id == 114 {
			file.read_exact(&mut [0; 16])?; // script_id
		}
		file.read_exact(&mut [0; 16])?; // old_type_hash

		*t = class_id;
	}

	// Objects
	let object_count = file.read_u32::<ORDER>()? as u64;
	info!("Reading {object_count} objects from assets file.");

	align_stream(file);

	// save the position because we will be jumping around
	let position = file.position();

	let mut raw_assets = RawAssets::default();

	// Iterate over the objects
	for (processed, i) in (0..object_count).enumerate() {
		if processed != 0 && processed as u64 % (object_count / 5) == 0 {
			info!("{processed} / {object_count} objects read...");
		}

		file.set_position(position + i * 24); // each entry is 24 bytes

		file.read_i64::<ORDER>()?; // path_id

		let byte_start = file.read_u64::<ORDER>()? + data_offset;

		file.read_u32::<ORDER>()?; // byte_size

		let type_id = file.read_u32::<ORDER>()?;
		let class_id = types[type_id as usize];

		if ![TEXT_ASSET, TEXTURE2D_ASSET].contains(&class_id) {
			continue;
		}

		// now we gotta jump to the actual object data to read it
		file.set_position(byte_start);

		match class_id {
			TEXT_ASSET => {
				let name = read_string::<ORDER>(file).context("object name")?;
				align_stream(file);

				let bytes_n = file.read_u32::<ORDER>()? as usize;

				let view = View {
					pos: file.position() as usize,
					len: bytes_n,
				};

				if name == "spritesheetf" {
					raw_assets.spritesheetf = view;
				} else {
					raw_assets.text_assets.push((name, view));
				}
			}
			TEXTURE2D_ASSET => {
				let name = read_string::<ORDER>(file).context("object name")?;
				align_stream(file);

				let _forced_fallback_format = file.read_u32::<ORDER>()?;
				let _downscale_fallback = file.read_u8()?;
				let _alpha_channel_optional = file.read_u8()?;
				align_stream(file);
				let width = file.read_u32::<ORDER>()?;
				let height = file.read_u32::<ORDER>()?;
				let _complete_image_size = file.read_u32::<ORDER>()?;
				let _mips_stripped = file.read_u32::<ORDER>()?;
				let texture_format = file.read_u32::<ORDER>()?;
				let _mip_count = file.read_u32::<ORDER>()?;
				let _is_readable = file.read_u8()?;
				let _is_preprocessed = file.read_u8()?;
				let _ignore_master_texture_limit = file.read_u8()?;
				let _streaming_mipmaps = file.read_u8()?;
				align_stream(file);
				let _streaming_mipmaps_priority = file.read_u32::<ORDER>()?;
				let _image_count = file.read_u32::<ORDER>()?;
				let _texture_dimension = file.read_u32::<ORDER>()?;
				let _filter_mode = file.read_u32::<ORDER>()?;
				let _aniso = file.read_u32::<ORDER>()?;
				let _mip_bias = file.read_f32::<ORDER>()?;
				let _wrap_mode = file.read_u32::<ORDER>()?;
				let _wrap_v = file.read_u32::<ORDER>()?;
				let _wrap_w = file.read_u32::<ORDER>()?;
				let _lightmap_format = file.read_u32::<ORDER>()?;
				let _color_space = file.read_u32::<ORDER>()?;
				let platform_blob_n = file.read_u32::<ORDER>()?;
				file.advance(platform_blob_n as usize);
				align_stream(file);
				let image_data_size = file.read_u32::<ORDER>()? as usize;
				let mut image_data = vec![0u8; image_data_size];
				file.read_exact(&mut image_data)?;

				let image = Image {
					w: width,
					h: height,
					data: image_data,
				};

				if name == "characters" {
					if texture_format != 4 {
						bail!("expected RGBA32 Texture2D image format");
					}
					raw_assets.characters = image;
				} else if name == "mapObjects" {
					if texture_format != 4 {
						bail!("expected RGBA32 Texture2D image format");
					}
					raw_assets.map_objects = image;
				}
			}
			_ => {}
		}
	}

	let assets = raw_assets.parse(file.get_mut(), config)?;

	info!("All assets extracted and read.");
	info!("Objects loaded: {}", assets.objects.len());

	Ok(assets)
}

// these clowns use both NUL terminated and length-prefixed strings 🤡🤡
fn read_nul_terminated_string<R: Read>(reader: &mut R) -> io::Result<String> {
	let mut res = Vec::new();
	loop {
		let byte = reader.read_u8()?;
		if byte == 0 {
			break;
		}

		res.push(byte);
	}

	let s = match String::from_utf8(res) {
		Ok(s) => s,
		Err(e) => return Err(Error::new(io::ErrorKind::InvalidData, e)),
	};
	Ok(s)
}
fn read_string<ORDER: ByteOrder>(file: &mut Cursor<Vec<u8>>) -> io::Result<String> {
	let length = file.read_u32::<ORDER>()?;
	let mut name = vec![0; length as usize];
	file.read_exact(&mut name)?;

	match String::from_utf8(name) {
		Ok(s) => Ok(s),
		Err(e) => Err(Error::new(io::ErrorKind::InvalidData, e)),
	}
}

// moves the stream forward to align to 4 bytes
fn align_stream(file: &mut Cursor<Vec<u8>>) {
	let position = file.position();
	let bytes_to_skip = (4 - (position % 4)) % 4;
	file.set_position(position + bytes_to_skip);
}

impl RawAssets {
	fn parse(self, file: &mut Vec<u8>, config: &Config) -> anyhow::Result<Assets> {
		// first make sure we actually collected all the views we need
		if self.spritesheetf.pos == 0 {
			bail!("spritesheetf not found");
		}
		if self.characters.data.is_empty() {
			bail!("characters not found");
		}
		if self.map_objects.data.is_empty() {
			bail!("mapObjects not found");
		}
		if self.text_assets.is_empty() {
			bail!("no text assets found");
		}

		let mut assets = Assets {
			spritesheets: HashMap::new(),
			animated_spritesheets: HashMap::new(),
			objects: HashMap::new(),
			enchantments: HashMap::new(),
			hazardous_tiles: BTreeMap::new(),
			conveyor_tiles: BTreeSet::new(),
			reverse_changes_guard: None,
		};

		// parse and extract the sprites
		let spritesheetf = spritesheetf::root_as_sprite_sheet_root(self.spritesheetf.get(file))?;

		for s in spritesheetf.sprites().unwrap() {
			if ![2, 4].contains(&s.atlas_id()) {
				continue;
			}
			let mut sprites = BTreeMap::new();

			for s in s.sprites().unwrap() {
				sprites.insert(
					s.index() as u32,
					self.get_sprite(s.atlas_id(), *s.position().unwrap()),
				);
			}

			assets
				.spritesheets
				.insert(s.name().unwrap().to_owned(), sprites);
		}

		for s in spritesheetf.animated_sprites().unwrap() {
			let sprite = s.sprite().unwrap();
			if ![2, 4].contains(&sprite.atlas_id()) {
				continue;
			}

			assets
				.animated_spritesheets
				.entry(s.name().unwrap().to_owned())
				.or_insert(BTreeMap::new())
				.insert(
					s.index() as u32,
					self.get_sprite(sprite.atlas_id(), *sprite.position().unwrap()),
				);
		}

		//  XML data
		for (name, view) in self.text_assets {
			let slice = view.get(file);

			match xmltree::Element::parse(&*slice) {
				Ok(xml) => process_xml(config, &mut assets, xml, slice)
					.context(format!("Error processing XML {:?}", name))?,
				Err(e) => {
					// not all text assets are XML and thats fine.
					debug!(
						"Skipping object {name:?} which is a TextAsset but not valid XML: {e:?}"
					);
				}
			}
		}

		if config.settings.edit_assets.enabled {
			assets.reverse_changes_guard =
				Some(ReverseChangesGuard::new(&config.assets_res, file)?);

			info!("Assets in filesystem modified.");
		}

		Ok(assets)
	}
}

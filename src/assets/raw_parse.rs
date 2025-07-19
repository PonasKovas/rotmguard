use anyhow::{Context, Result, bail};
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use std::{
	fs::File,
	io::{self, BufReader, Error, Read, Seek},
	path::Path,
};
use tracing::{debug, info};

pub struct RawAssets {
	pub spritesheetf: Vec<u8>,
	pub characters: Texture2D,
	pub map_objects: Texture2D,
	pub xml_assets: Vec<XmlAsset>,
}

pub struct Texture2D {
	pub width: u32,
	pub height: u32,
	pub data: Vec<u8>,
}

pub struct XmlAsset {
	pub name: String,
	pub data: Vec<u8>,
	// for when we want to overwrite some XML in the assets file
	pub position: u64,
	pub original_size: usize,
}

// class IDs of types of assets in the unity binary format
const TEXT_ASSET: i32 = 49;
const TEXTURE2D_ASSET: i32 = 28;

impl RawAssets {
	pub fn parse(file: impl AsRef<Path>) -> Result<Self> {
		let mut file = BufReader::new(File::open(file)?);

		let real_size = file.get_ref().metadata()?.len();

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
			in_endian::<BigEndian>(file, data_offset)
		} else {
			in_endian::<LittleEndian>(file, data_offset)
		}
	}
}

fn in_endian<ORDER: ByteOrder>(mut file: BufReader<File>, data_offset: u64) -> Result<RawAssets> {
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
	let object_count = file.read_u32::<ORDER>()? as usize;
	align_stream(&mut file)?;
	let mut objects = Vec::new();
	struct Object {
		pos: u64,
		class: ObjectClass,
	}
	enum ObjectClass {
		Text,
		Texture2D,
	}
	for _ in 0..object_count {
		file.read_i64::<ORDER>()?; // path_id
		let data_position = file.read_u64::<ORDER>()? + data_offset;
		file.read_u32::<ORDER>()?; // byte_size
		let type_id = file.read_u32::<ORDER>()?;

		let class_id = types[type_id as usize];
		if [TEXT_ASSET, TEXTURE2D_ASSET].contains(&class_id) {
			objects.push(Object {
				pos: data_position,
				class: match class_id {
					TEXT_ASSET => ObjectClass::Text,
					TEXTURE2D_ASSET => ObjectClass::Texture2D,
					_ => unreachable!(),
				},
			});
		}
	}

	// sort the object list by position so we're being cache friendly
	objects.sort_unstable_by_key(|obj| obj.pos);

	info!("Reading {} objects from assets file.", objects.len());

	let mut spritesheetf = None;
	let mut characters = None;
	let mut map_objects = None;
	let mut xml_assets = Vec::new();

	for (i, object) in objects.iter().enumerate() {
		// print 5 lines for progress status :)
		if i != 0 && i % (objects.len() / 5) == 0 {
			info!("{i} / {} objects read...", objects.len());
		}

		file.seek(io::SeekFrom::Start(object.pos))?;

		match object.class {
			ObjectClass::Text => {
				let name = read_string::<ORDER>(&mut file).context("text object name")?;
				align_stream(&mut file)?;

				let len = file.read_u32::<ORDER>()? as usize;

				let position = file.stream_position()?;

				// we are only interested in a binary file called spritesheetf and in xml files
				if name == "spritesheetf" {
					let mut data = vec![0; len];
					file.read_exact(&mut data)?;

					if spritesheetf.replace(data).is_some() {
						bail!("duplicate spritesheetf");
					}
				} else {
					// just to avoid adding objects that are definitely not xml
					// check if the data starts with a `<`
					let mut hint_xml = true;
					// only check as far as the BufReader's buffer goes
					for &byte in file.buffer() {
						// skip all whitespace
						if byte.is_ascii_whitespace() {
							continue;
						}
						// first non whitespace character must be '<'
						hint_xml = byte == b'<';
						break;
					}

					if !hint_xml {
						debug!("skipping {name} as it was found to not be XML");
						continue;
					}

					let mut data = vec![0; len];
					file.read_exact(&mut data)?;

					xml_assets.push(XmlAsset {
						name,
						data,
						position,
						original_size: len,
					});
				}
			}
			ObjectClass::Texture2D => {
				let name = read_string::<ORDER>(&mut file).context("texture2d object name")?;

				// immediatelly shortcircuit if name is not something we are interested in
				if !["mapObjects", "characters"].contains(&name.as_str()) {
					continue;
				}
				align_stream(&mut file)?;

				// a bunch of slop...
				let _forced_fallback_format = file.read_u32::<ORDER>()?;
				let _downscale_fallback = file.read_u8()?;
				let _alpha_channel_optional = file.read_u8()?;
				align_stream(&mut file)?;
				let width = file.read_u32::<ORDER>()?;
				let height = file.read_u32::<ORDER>()?;
				let _complete_image_size = file.read_u32::<ORDER>()?;
				let _mips_stripped = file.read_u32::<ORDER>()?;
				let texture_format = file.read_u32::<ORDER>()?;
				if texture_format != 4 {
					bail!("expected RGBA32 Texture2D image format");
				}
				let _mip_count = file.read_u32::<ORDER>()?;
				let _is_readable = file.read_u8()?;
				let _is_preprocessed = file.read_u8()?;
				let _ignore_master_texture_limit = file.read_u8()?;
				let _streaming_mipmaps = file.read_u8()?;
				align_stream(&mut file)?;
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
				file.seek_relative(platform_blob_n as i64)?;
				align_stream(&mut file)?;
				let texture_data_size = file.read_u32::<ORDER>()? as usize;
				let mut texture_data = vec![0u8; texture_data_size];
				file.read_exact(&mut texture_data)?;

				let texture = Texture2D {
					width,
					height,
					data: texture_data,
				};

				match name.as_str() {
					"characters" => {
						if characters.replace(texture).is_some() {
							bail!("duplicate 'characters' Texture2D");
						}
					}
					"mapObjects" => {
						if map_objects.replace(texture).is_some() {
							bail!("duplicate 'mapObjects' Texture2D");
						}
					}
					_ => unreachable!(),
				}
			}
		}
	}

	let raw_assets = RawAssets {
		spritesheetf: spritesheetf.context("spritesheetf not found")?,
		characters: characters.context("characters not found")?,
		map_objects: map_objects.context("map_objects not found")?,
		xml_assets,
	};

	Ok(raw_assets)
}

// these clowns use both NUL terminated and length-prefixed strings 🤡🤡
fn read_nul_terminated_string(reader: &mut impl Read) -> io::Result<String> {
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
fn read_string<ORDER: ByteOrder>(file: &mut impl Read) -> io::Result<String> {
	let length = file.read_u32::<ORDER>()?;
	let mut name = vec![0; length as usize];
	file.read_exact(&mut name)?;

	match String::from_utf8(name) {
		Ok(s) => Ok(s),
		Err(e) => Err(Error::new(io::ErrorKind::InvalidData, e)),
	}
}

// moves the stream forward to align to 4 bytes
fn align_stream(file: &mut BufReader<File>) -> io::Result<()> {
	let position = file.stream_position()? as i64;
	let aligned_position = (position + 3) & !3;

	file.seek_relative(aligned_position - position)?;

	Ok(())
}

use crate::config::Config;
use anyhow::{Context, bail};
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use nix::NixPath;
use rand::prelude::*;
use reverse_changes::ReverseChangesGuard;
use std::{
	collections::{BTreeMap, BTreeSet},
	io::{self, Cursor, Error, Read},
};
use tracing::{debug, info};
use xml::process_xml;

mod reverse_changes;
mod xml;

pub struct Assets {
	/// Map<object type -> Map<projectile_type -> projectile_info>>
	pub projectiles: BTreeMap<u32, BTreeMap<u32, ProjectileInfo>>,
	/// ground type -> damage
	pub hazardous_tiles: BTreeMap<u16, i64>,
	/// grounds that push the player like conveyors
	pub conveyor_tiles: BTreeSet<u16>,

	/// Reverses the changes to assets file on drop
	reverse_changes_guard: Option<ReverseChangesGuard>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ProjectileInfo {
	pub armor_piercing: bool,
	pub inflicts_cursed: bool,
	pub inflicts_exposed: bool,
	pub inflicts_sick: bool,
	pub inflicts_bleeding: bool,
	pub inflicts_armor_broken: bool,
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
	let mut assets = Assets {
		projectiles: BTreeMap::new(),
		hazardous_tiles: BTreeMap::new(),
		conveyor_tiles: BTreeSet::new(),
		reverse_changes_guard: None,
	};

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

	// Iterate over the objects

	// now the below is completely unnecessary but it makes the progress bar not useless
	// basically we skip like 90% of these objects, and all those that we don't skip
	// are clustered together like a bunch of clowns in a car, so without this
	// the progress bar is very jumpy.
	let mut indices = (0..object_count).collect::<Vec<u64>>();
	indices.shuffle(&mut rand::thread_rng());

	for (processed, i) in indices.into_iter().enumerate() {
		if processed != 0 && processed as u64 % (object_count / 5) == 0 {
			info!("{processed} / {object_count} objects read...");
		}

		file.set_position(position + i * 24); // each entry is 24 bytes

		file.read_i64::<ORDER>()?; // path_id

		let byte_start = file.read_u64::<ORDER>()? + data_offset;

		file.read_u32::<ORDER>()?; // byte_size

		let type_id = file.read_u32::<ORDER>()?;
		let class_id = types[type_id as usize];

		if class_id != 49 {
			// 49 is TextAsset - the only one we need
			continue;
		}

		// now we gotta jump to the actual object data to read it
		file.set_position(byte_start);

		let name = read_string::<ORDER>(file)?;
		align_stream(file);

		let bytes_n = file.read_u32::<ORDER>()? as usize;
		let p = file.position() as usize;
		let slice = &mut file.get_mut()[p..(p + bytes_n)];

		match xmltree::Element::parse(&*slice) {
			Ok(xml) => process_xml(config, &mut assets, xml, slice)
				.context(format!("Error processing XML {:?}", name))?,
			Err(e) => {
				debug!("Skipping object {name:?} which is a TextAsset but not valid XML: {e:?}");
			}
		}
	}

	info!("All assets extracted and read.");

	if config.settings.edit_assets.enabled {
		assets.reverse_changes_guard = Some(ReverseChangesGuard::new(
			&config.assets_res,
			file.get_ref(),
		)?);

		info!("Assets in filesystem modified.");
	}

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

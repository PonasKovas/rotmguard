use anyhow::{bail, Context};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	fs::File,
	io::{self, Error, Read, Seek},
	path::{Path, PathBuf},
	sync::Mutex,
};
use tracing::{error, info};
use xmltree::XMLNode;

use crate::config;

const NON_XML_FILES: &[&str] = &[
	"manifest_xml",
	"COPYING",
	"Errors",
	"ExplainUnzip",
	"cloth_bazaar",
	"Cursors",
	"Dialogs",
	"Keyboard",
	"LICENSE",
	"LineBreaking Following Characters",
	"LineBreaking Leading Characters",
	"manifest_json",
	"spritesheetf",
	"iso_4217",
	"data",
	"manifest",
	"BillingMode",
];

/// object type -> Map<projectile_type -> projectile_info>
pub static PROJECTILES: Mutex<BTreeMap<u32, BTreeMap<u32, ProjectileInfo>>> =
	Mutex::new(BTreeMap::new());
/// ground type -> damage
pub static HAZARDOUS_GROUNDS: Mutex<BTreeMap<u32, i64>> = Mutex::new(BTreeMap::new());
/// grounds that push the player like conveyors
pub static PUSH_GROUNDS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ProjectileInfo {
	pub armor_piercing: bool,
	pub inflicts_cursed: bool,
	pub inflicts_exposed: bool,
	pub inflicts_sick: bool,
	pub inflicts_bleeding: bool,
	pub inflicts_armor_broken: bool,
}

/// This cleans up and reverses the changes to resources.assets file on drop
pub struct EditedAssetsGuard {
	real_assets_path: PathBuf,
	edited_assets_path: Option<PathBuf>,
}

impl Drop for EditedAssetsGuard {
	fn drop(&mut self) {
		if let Some(edited_assets_path) = self.edited_assets_path.take() {
			// delete the edited assets and rename original back to its place
			if let Err(e) = std::fs::remove_file(&edited_assets_path)
				.and_then(|_| std::fs::rename(&self.real_assets_path, &edited_assets_path))
			{
				error!("Error reversing changes to game files: {e:?}");
				error!("To do it manually: delete the `resources.assets` file, and rename `resources.assets.rotgmuard` to `resources.assets`.")
			} else {
				info!("Successfully reversed changed to game files.");
			}
		}
	}
}

pub fn extract_assets(path: &Path) -> io::Result<EditedAssetsGuard> {
	let mut file = File::open(path)?;

	// If forcing debuffs, read the whole file into memory so it can be edited and written to replace the original file
	let mut force_debuffs = if config().settings.lock().unwrap().force_debuffs {
		Some(std::fs::read(path)?)
	} else {
		None
	};

	let real_size = file.metadata().unwrap().len();

	// this is all written for version 22+ by the way
	// if you have older version then idk..

	file.read_exact(&mut [0; 4 * 2])?; // 2 ints
	let version = file.read_i32::<BigEndian>()?;
	file.read_exact(&mut [0; 4])?; // int
	let big_endian = file.read_u8()? != 0;
	file.read_exact(&mut [0; 3])?;
	let metadata_size = file.read_u32::<BigEndian>()? as u64;
	let file_size = file.read_u64::<BigEndian>()?;
	let data_offset = file.read_u64::<BigEndian>()?;
	file.read_i64::<BigEndian>()?; // unknown

	// Some wack ass sanity tests, I didn't write these - stolen
	if version > 100
		|| file_size > real_size
		|| metadata_size > real_size
		|| (version as u64) > real_size
		|| data_offset > real_size
		|| file_size < metadata_size
		|| file_size < data_offset
	{
		return Err(Error::new(
			io::ErrorKind::InvalidData,
			"invalid assets file",
		));
	}

	// bro i am not gonna waste my time trying to support both if only little endian is ever going to be used
	// If you ever get this error, you can add support for big endian by reading all data from this point
	// in big endian
	if big_endian {
		return Err(Error::new(
			io::ErrorKind::Unsupported,
			"big endian not supported.",
		));
	}

	// NUL-terminated string LOL 😂
	read_nul_terminated_string(&mut file)?; // unity version

	file.read_u32::<LittleEndian>()?; // target_platform
	let enable_type_tree = file.read_u8()? != 0;

	if enable_type_tree {
		return Err(Error::new(
			io::ErrorKind::Unsupported,
			"enable_type_tree not supported.",
		));
	}

	// Types
	let types_count = file.read_u32::<LittleEndian>()? as usize;
	let mut types = vec![0; types_count];

	for t in types.iter_mut() {
		let class_id = file.read_i32::<LittleEndian>()?;
		file.read_exact(&mut [0; 1 + 2])?; // is_stripped_type + script_type_index
		if class_id == 114 {
			file.read_exact(&mut [0; 16])?; // script_id
		}
		file.read_exact(&mut [0; 16])?; // old_type_hash

		*t = class_id;
	}

	// Objects
	let object_count = file.read_u32::<LittleEndian>()?;
	info!("Reading {object_count} objects from assets file.");
	for i in 0..object_count {
		if i != 0 && i % (object_count / 5) == 0 {
			info!("{i} / {object_count} objects read...");
		}

		// align the stream
		align_stream(&mut file)?;

		file.read_i64::<LittleEndian>()?; // path_id

		let byte_start = file.read_u64::<LittleEndian>()? + data_offset;

		// let byte_size_offset = position + bytes_to_skip + 8 * 2;
		file.read_u32::<LittleEndian>()?; // byte_size

		let type_id = file.read_u32::<LittleEndian>()?;
		let class_id = types[type_id as usize];

		if class_id != 49 {
			// 49 is TextAsset - the only one we need
			continue;
		}

		// now we gotta jump to the actual object data to read it, and then jump back for next iteration
		let position = file.stream_position()?;

		file.seek(io::SeekFrom::Start(byte_start))?;

		let name_length = file.read_u32::<LittleEndian>()?;
		let mut name = vec![0; name_length as usize];
		file.read_exact(&mut name)?;
		let name = match String::from_utf8(name) {
			Ok(s) => s,
			Err(e) => return Err(Error::new(io::ErrorKind::InvalidData, e)),
		};
		align_stream(&mut file)?;

		if !NON_XML_FILES.iter().any(|&n| n == name) {
			// We only want XML files
			let bytes_n = file.read_u32::<LittleEndian>()?;
			let xml_position = file.stream_position()?;
			let mut xml = vec![0; bytes_n as usize];
			file.read_exact(&mut xml)?;

			let xml = match String::from_utf8(xml) {
				Ok(s) => s,
				Err(e) => return Err(Error::new(io::ErrorKind::InvalidData, e)),
			};

			if let Err(e) = process_xml(&xml, &mut force_debuffs, xml_position as usize) {
				error!("Error processing {name} XML asset: {e}");
			}
		}

		file.seek(io::SeekFrom::Start(position))?;
	}

	info!("All assets extracted and read.");

	if let Some(contents) = force_debuffs {
		// rename the original file and write the edited file in its place
		let mut original_path = path.as_os_str().to_owned();
		original_path.push(".rotmguard");
		std::fs::rename(path, &original_path)?;

		std::fs::write(path, &contents)?;

		// Set the owner and group IDs to match with the parent directory instead of being root.
		let parent_dir = path.parent().unwrap_or(&Path::new("."));
		let (o_id, g_id) = match file_owner::owner_group(parent_dir) {
			Ok(r) => r,
			Err(e) => {
				return Err(Error::other(format!(
					"Couldn't get the owner of {parent_dir:?}: {e:?}"
				)));
			}
		};
		match file_owner::set_owner_group(path, o_id, g_id) {
			Ok(_) => {}
			Err(e) => {
				return Err(Error::other(format!(
					"Couldn't set the owner of {path:?}: {e:?}"
				)));
			}
		}

		info!("Assets edited to force anti-debuffs.");

		return Ok(EditedAssetsGuard {
			real_assets_path: Path::new(&original_path).to_path_buf(),
			edited_assets_path: Some(path.to_path_buf()),
		});
	}

	Ok(EditedAssetsGuard {
		real_assets_path: path.to_path_buf(),
		edited_assets_path: None,
	})
}

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

fn align_stream<S: Seek + Read>(stream: &mut S) -> io::Result<()> {
	let position = stream.stream_position()?;
	let bytes_to_skip = (4 - (position % 4)) % 4;
	for _ in 0..bytes_to_skip {
		stream.read_u8()?;
	}

	Ok(())
}

// Parses XML asset and adds to the registry
fn process_xml(
	raw_xml: &str,
	force_debuffs: &mut Option<Vec<u8>>,
	xml_pos: usize,
) -> anyhow::Result<()> {
	let mut xml = xmltree::Element::parse(raw_xml.as_bytes()).unwrap();

	match xml.name.as_str() {
		"Objects" => process_xml_objects(&mut xml.children, force_debuffs.is_some())?,
		"GroundTypes" => process_xml_grounds(&mut xml.children)?,
		_ => return Ok(()), // Not Interested 👍
	}

	if let Some(file) = force_debuffs {
		let mut edited_xml = Vec::with_capacity(raw_xml.len());

		xml.write(&mut edited_xml)?;

		// add spaces to the end to make sure old and edited XMLs have the same
		// length to not fuck up the rest of the file
		let to_add = match raw_xml.len().checked_sub(edited_xml.len()) {
			Some(n) => n,
			None => bail!("Tried to force remove condition effect but XML length increased??"),
		};

		for _ in 0..to_add {
			edited_xml.push(b' '); // hopefully spaces dont fuck up the format?
		}

		file[xml_pos..(xml_pos + raw_xml.len())].copy_from_slice(&edited_xml);
	}

	Ok(())
}

fn process_xml_objects(objects: &mut Vec<XMLNode>, force_debuffs: bool) -> anyhow::Result<()> {
	for object in objects {
		if let XMLNode::Element(object) = object {
			if object.name != "Object" {
				// Again, ONLY INTERESTED IN OBJECTS!
				continue;
			}

			let object_type = object
				.attributes
				.get("type")
				.context("Object has no 'type'")?;

			// parse the goofy ass object type
			let object_type = object_type
				.strip_prefix("0x")
				.context("unexpected Object type format")?;
			let object_type =
				u32::from_str_radix(object_type, 16).context("unexpected Object type format")?;

			let mut projectiles = BTreeMap::new();
			let mut i = 0;
			for parameter in &mut object.children {
				if let XMLNode::Element(parameter) = parameter {
					if parameter.name == "Projectile" {
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
						for projectile_parameter_i in (0..parameter.children.len()).rev() {
							if let XMLNode::Element(projectile_parameter) =
								&parameter.children[projectile_parameter_i]
							{
								if projectile_parameter.name == "ArmorPiercing" {
									armor_piercing = true;
								} else if projectile_parameter.name == "ConditionEffect" {
									if projectile_parameter.children.is_empty()
										|| projectile_parameter.children.len() > 1
									{
										bail!("Invalid Object Projectile ConditionEffect. Must have only text inside");
									}

									if let XMLNode::Text(condition) =
										&projectile_parameter.children[0]
									{
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
										if force_debuffs {
											let debuffs =
												&config().settings.lock().unwrap().debuffs;
											let c = condition.as_str();
											if (c == "Blind" && debuffs.blind)
												|| (c == "Hallucinating" && debuffs.hallucinating)
												|| (c == "Drunk" && debuffs.drunk) || (c
												== "Confused"
												&& debuffs.confused) || (c == "Unstable"
												&& debuffs.unstable) || (c == "Darkness"
												&& debuffs.darkness)
											{
												parameter.children.remove(projectile_parameter_i);
											}
										}
									} else {
										bail!("Invalid Object Projectile ConditionEffect. Value be text");
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
				}
			}

			if !projectiles.is_empty() {
				// save
				PROJECTILES.lock().unwrap().insert(object_type, projectiles);
			}
		}
	}

	Ok(())
}

fn process_xml_grounds(grounds: &mut Vec<XMLNode>) -> anyhow::Result<()> {
	for object in grounds {
		if let XMLNode::Element(object) = object {
			if object.name != "Ground" {
				// ONLY INTERESTED IN GROUND TYPES!
				continue;
			}

			let ground_type = object
				.attributes
				.get("type")
				.context("Ground has no 'type'")?;

			// parse the goofy ass ground type
			let ground_type = ground_type
				.strip_prefix("0x")
				.context("unexpected Ground type format")?;
			let ground_type =
				u32::from_str_radix(ground_type, 16).context("unexpected Ground type format")?;

			let params = object.children.iter().filter_map(|p| {
				if let XMLNode::Element(p) = p {
					Some(p)
				} else {
					None
				}
			});

			if let Some(param) = params.clone().find(|p| p.name == "MaxDamage") {
				if param.children.is_empty() || param.children.len() > 1 {
					bail!("Invalid Ground MaxDamage. Must have only text");
				}

				if let XMLNode::Text(dmg) = &param.children[0] {
					let damage = dmg
						.parse::<i64>()
						.context("Invalid Ground MaxDamage, must be integer")?;

					HAZARDOUS_GROUNDS
						.lock()
						.unwrap()
						.insert(ground_type, damage);
				} else {
					bail!("Invalid Ground MaxDamage. Value be text");
				}
			}

			if let Some(_) = params.clone().find(|p| p.name == "Push") {
				PUSH_GROUNDS.lock().unwrap().insert(ground_type);
			}
		}
	}

	Ok(())
}

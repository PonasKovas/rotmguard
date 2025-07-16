use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};

pub const CONFIG_PATH: &str = "rotmguard.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
	/// A path to the game's resources.assets.
	/// Look in your proton pfx, it can usually be found in somewhere like
	/// C:/users/steamuser/Documents/RealmOfTheMadGod/Production/RotMG Exalt_Data/
	pub assets_res: PathBuf,
	pub settings: Settings,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
	/// HP at which to autonexus. Recommended value 20
	pub autonexus_hp: Mutex<i32>,
	/// Enables damage monitoring, see stats with /dmg command
	pub damage_monitor: bool,
	/// If true, will activate developer mode.
	pub dev_mode: Mutex<bool>,
	/// How many log lines to save up to the event that triggered a log save
	pub log_lines: usize,
	/// Which client-side debuffs to disable
	pub debuffs: Debuffs,
	pub edit_assets: EditAssets,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Debuffs {
	/// If true will be disabled
	#[serde(default)]
	pub blind: bool,
	/// If true will be disabled
	#[serde(default)]
	pub hallucinating: bool,
	/// If true will be disabled
	#[serde(default)]
	pub drunk: bool,
	/// If true will be disabled
	#[serde(default)]
	pub confused: bool,
	/// If true will be disabled
	#[serde(default)]
	pub hexed: bool,
	/// If true will be disabled
	#[serde(default)]
	pub unstable: bool,
	/// If true will be disabled
	#[serde(default)]
	pub darkness: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct EditAssets {
	pub enabled: bool,
	/// If true, will remove the client-side debuffs completely
	pub force_debuffs: bool,
}

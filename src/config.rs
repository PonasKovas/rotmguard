use std::{path::PathBuf, sync::Mutex};

use serde::{Deserialize, Serialize};

pub const CONFIG_PATH: &str = "rotmguard.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
	/// A path to the game's resources.assets.
	/// Look in your proton pfx, it can usually be found in somewhere like
	/// C:/users/steamuser/Documents/RealmOfTheMadGod/Production/RotMG Exalt_Data/
	pub assets_res: PathBuf,
	pub settings: Mutex<Settings>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
	/// HP at which to autonexus. Recommended value 20
	pub autonexus_hp: i64,
	/// Will show a fake name for the client if set.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[serde(default)]
	pub fakename: Option<String>,
	/// If true, will activate developer mode.
	pub dev_mode: bool,
	/// How many log lines to save up to the event that triggered a log save
	pub log_lines: usize,
}

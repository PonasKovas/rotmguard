use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const CONFIG_PATH: &str = "rotmguard.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// A path to the game's resources.assets.
    /// Look in your proton pfx, it can usually be found in somewhere like
    /// C:/users/steamuser/Documents/RealmOfTheMadGod/Production/RotMG Exalt_Data/
    pub assets_res: PathBuf,
}

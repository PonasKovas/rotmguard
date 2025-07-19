use crate::config::Config;
use anyhow::Result;
use std::{
	fs::{OpenOptions, metadata},
	io::{self, Seek, SeekFrom, Write},
	path::PathBuf,
	time::SystemTime,
};
use tracing::{error, info, warn};

// Instructs to overwrite some region in the resources.assets file
pub struct OverwriteRegion {
	pub position: u64,
	pub data: Vec<u8>,
}

/// This cleans up and reverses the changes to resources.assets file on drop
pub struct ReverseChangesGuard {
	// the path to the copied ORIGINAL assets
	assets_path_copy: PathBuf,
	// the path to the edited assets
	assets_path: PathBuf,

	last_modified: Option<SystemTime>,
}

pub fn modify(
	config: &Config,
	to_overwrite: Vec<OverwriteRegion>,
) -> Result<Option<ReverseChangesGuard>> {
	if to_overwrite.is_empty() {
		return Ok(None);
	}

	// first, make a copy of the original file so the modifications can be reversed on exit
	let mut copy_path = config.assets_res.as_os_str().to_owned(); // that feeling when PathBuf::add_extension is still unstable...
	copy_path.push(".rotmguard");
	std::fs::copy(&config.assets_res, &copy_path)?;

	// immediatelly create a guard which will put the copy back as original on drop
	let mut guard = ReverseChangesGuard {
		assets_path_copy: copy_path.into(),
		assets_path: config.assets_res.clone(),
		last_modified: None,
	};

	let mut file = OpenOptions::new().write(true).open(&config.assets_res)?;
	for region in &to_overwrite {
		file.seek(SeekFrom::Start(region.position))?;
		file.write_all(&region.data)?;
	}

	info!(
		"{} regions overwritten in resources.assets",
		to_overwrite.len()
	);

	if let Ok(last_modified) = file.metadata()?.modified() {
		guard.last_modified = Some(last_modified);
	}

	Ok(Some(guard))
}

impl ReverseChangesGuard {
	fn finish(&mut self) -> io::Result<()> {
		// normally we want to delete the edited assets and rename the original back to its place
		// but if the edited assets have been changed on the disk since we wrote them
		// its likely that the user updated rotmg and overwrote it
		//
		// in that case we dont want to do anything, just delete the backup if its still there
		if metadata(&self.assets_path)?.modified().ok() != self.last_modified {
			// uh oh... file changed
			warn!(
				"🚨 resources.assets changed on disk while rotmguard was running. Leaving it as it is."
			);
			std::fs::remove_file(&self.assets_path_copy)?;
		} else {
			// all fine, standard procedure
			std::fs::remove_file(&self.assets_path)?;
			std::fs::rename(&self.assets_path_copy, &self.assets_path)?;

			info!("Successfully reversed changes to resources.assets.");
		}

		Ok(())
	}
}

impl Drop for ReverseChangesGuard {
	fn drop(&mut self) {
		if let Err(e) = self.finish() {
			error!("Error reversing changes to game files: {e:?}");
			error!(
				"To do it manually: delete the `resources.assets` file, and rename `resources.assets.rotgmuard` to `resources.assets`."
			);
			error!("Or just clear all game data and reinstall.");
		}
	}
}

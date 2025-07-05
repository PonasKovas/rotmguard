use std::{
	fs::File,
	io::{self, Error, Read},
	path::{Path, PathBuf},
};
use tracing::{error, info, warn};

/// This cleans up and reverses the changes to resources.assets file on drop
pub struct ReverseChangesGuard {
	// the path to the renamed original assets
	backup_assets_path: PathBuf,
	// the path to the edited assets
	edited_assets_path: PathBuf,
	// BLAKE3 hash of the modified resources.assets file
	hash: blake3::Hash,
}

impl ReverseChangesGuard {
	// renames the original file and writes the edited file in its place
	pub fn new(assets_path: &Path, contents: &[u8]) -> io::Result<Self> {
		let mut new_path = assets_path.as_os_str().to_owned();
		new_path.push(".rotmguard");
		std::fs::rename(assets_path, &new_path)?;

		let guard = ReverseChangesGuard {
			backup_assets_path: Path::new(&new_path).to_path_buf(),
			edited_assets_path: assets_path.to_owned(),
			hash: blake3::hash(contents),
		};

		std::fs::write(assets_path, contents)?;

		// Set the owner and group IDs to match with the parent directory instead of being root.
		let parent_dir = assets_path.parent().unwrap_or(Path::new("."));
		let (o_id, g_id) = match file_owner::owner_group(parent_dir) {
			Ok(r) => r,
			Err(e) => {
				return Err(Error::other(format!(
					"Couldn't get the owner of {parent_dir:?}: {e:?}"
				)));
			}
		};
		match file_owner::set_owner_group(assets_path, o_id, g_id) {
			Ok(_) => {}
			Err(e) => {
				return Err(Error::other(format!(
					"Couldn't set the owner of {path:?}: {e:?}",
					path = assets_path,
				)));
			}
		}

		Ok(guard)
	}

	fn finish(&mut self) -> io::Result<()> {
		// normally we want to delete the edited assets and rename the original back to its place
		// but if the edited assets have been changed on the disk since we wrote them
		// its likely that the user updated rotmg and overwrote it
		//
		// in that case we dont want to do anything, just delete the backup if its still there
		let mut file = File::open(&self.edited_assets_path)?;

		let mut hasher = blake3::Hasher::new();
		let mut buf = [0u8; 64 * 1024];
		loop {
			let n_read = file.read(&mut buf)?;
			if n_read == 0 {
				break;
			}
			hasher.update(&buf[..n_read]);
		}

		let hash = hasher.finalize();

		if self.hash != hash {
			// uh oh... file changed
			warn!(
				"resources.assets changed on disk while rotmguard was running. Leaving it as it is."
			);
			std::fs::remove_file(&self.backup_assets_path)?;
		} else {
			// all fine, standard procedure
			std::fs::remove_file(&self.edited_assets_path)?;
			std::fs::rename(&self.backup_assets_path, &self.edited_assets_path)?;

			info!("Successfully reversed changes to game files.");
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

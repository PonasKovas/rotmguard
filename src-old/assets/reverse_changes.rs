use std::{
	io::{self, Error},
	path::{Path, PathBuf},
};
use tracing::{error, info};

/// This cleans up and reverses the changes to resources.assets file on drop
pub struct ReverseChangesGuard {
	// the path to the renamed original assets
	real_assets_path: PathBuf,
	// the path to the edited assets
	edited_assets_path: PathBuf,
}

impl ReverseChangesGuard {
	// renames the original file and writes the edited file in its place
	pub fn new(assets_path: &Path, contents: &[u8]) -> io::Result<Self> {
		let mut new_path = assets_path.as_os_str().to_owned();
		new_path.push(".rotmguard");
		std::fs::rename(assets_path, &new_path)?;

		let guard = ReverseChangesGuard {
			real_assets_path: Path::new(&new_path).to_path_buf(),
			edited_assets_path: assets_path.to_owned(),
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
}

impl Drop for ReverseChangesGuard {
	fn drop(&mut self) {
		// delete the edited assets and rename original back to its place
		if let Err(e) = std::fs::remove_file(&self.edited_assets_path)
			.and_then(|_| std::fs::rename(&self.real_assets_path, &self.edited_assets_path))
		{
			error!("Error reversing changes to game files: {e:?}");
			error!("To do it manually: delete the `resources.assets` file, and rename `resources.assets.rotgmuard` to `resources.assets`.")
		} else {
			info!("Successfully reversed changes to game files.");
		}
	}
}

//! generic game state used across different cheats

use bullets::Bullets;

pub mod bullets;

pub struct Common {
	pub bullets: Bullets,
}

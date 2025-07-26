//! generic game state used across different cheats

use bullets::Bullets;
use objects::Objects;

pub mod bullets;
pub mod objects;

pub use bullets::{enemyshoot, playershoot, serverplayershoot};
pub use objects::{add_object, object_int_stat, object_str_stat, remove_object};

#[derive(Default)]
pub struct Common {
	pub my_position: (f32, f32),
	pub server_tick_id: u32,
	pub bullets: Bullets,
	pub objects: Objects,
}

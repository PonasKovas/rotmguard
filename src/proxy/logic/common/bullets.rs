use crate::assets::ProjectileInfo;
use lru::LruCache;
use std::num::NonZeroUsize;

const ENEMY_BULLETS_CACHE: usize = 10_000;
const MY_OWN_BULLETS_CACHE: usize = 2_000;

pub struct Bullets {
	pub enemy: LruCache<BulletId, Bullet>,
	pub my_own: LruCache<BulletId, Bullet>,
}

// this game is so coherent that it has duplicating bullet ids, you need them together with the owner id
// to tell them apart
#[derive(PartialEq, Clone, Copy, Debug, Hash, Eq)]
pub struct BulletId {
	pub id: u16,
	pub owner_id: u32,
}

pub struct Bullet {
	pub damage: u16,
	pub properties: ProjectileInfo,
}

impl Default for Bullets {
	fn default() -> Self {
		Self {
			enemy: LruCache::new(NonZeroUsize::new(ENEMY_BULLETS_CACHE).unwrap()),
			my_own: LruCache::new(NonZeroUsize::new(MY_OWN_BULLETS_CACHE).unwrap()),
		}
	}
}

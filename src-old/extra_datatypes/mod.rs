#![allow(dead_code)]

mod conditions;
mod object_status_data;
mod statdata;
mod worldpos;

pub use conditions::{PlayerConditions, PlayerConditions2};
pub use object_status_data::ObjectStatusData;
pub use statdata::{Stat, StatData, StatType};
pub use worldpos::WorldPos;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectileId {
	pub id: u16,
	pub owner_id: ObjectId,
}

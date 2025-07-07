#[allow(dead_code, non_snake_case)]
pub mod STAT_TYPE {
	pub const MAX_HP: u8 = 0;
	pub const HP: u8 = 1;
	pub const MAX_MP: u8 = 3;
	pub const MP: u8 = 4;
	pub const DEF: u8 = 21;
	pub const SPD: u8 = 22;
	pub const VIT: u8 = 26;
	pub const CONDITION: u8 = 29;
	pub const NAME: u8 = 31;
	pub const CURRENT_FAME: u8 = 57;
	pub const CLASS_QUEST_FAME: u8 = 58;
	pub const GUILD_NAME: u8 = 62;
	pub const CONDITION2: u8 = 96;
}

#[allow(dead_code, non_snake_case)]
pub mod CONDITION_BITFLAG {
	pub const SLOW: u64 = 0x8;
	pub const SICK: u64 = 0x10;
	pub const BLIND: u64 = 0x80;
	pub const HALLUCINATING: u64 = 0x100;
	pub const DRUNK: u64 = 0x200;
	pub const CONFUSED: u64 = 0x400;
	pub const BLEEDING: u64 = 0x8000;
	pub const HEALING: u64 = 0x20000;
	pub const IN_COMBAT: u64 = 0x100000;
	pub const INVINCIBLE: u64 = 0x800000;
	pub const INVULNERABLE: u64 = 0x1000000;
	pub const ARMORED: u64 = 0x2000000;
	pub const ARMOR_BROKEN: u64 = 0x4000000;
	pub const HEXED: u64 = 0x8000000;
	pub const UNSTABLE: u64 = 0x20000000;
	pub const DARKNESS: u64 = 0x40000000;
}

#[allow(dead_code, non_snake_case)]
pub mod CONDITION2_BITFLAG {
	pub const CURSED: u64 = 0x40;
	pub const EXPOSED: u64 = 0x20000;
}

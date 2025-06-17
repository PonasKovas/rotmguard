#[derive(Debug, Clone, Copy)]
pub struct PlayerConditions {
	pub bitmask: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct PlayerConditions2 {
	pub bitmask: u64,
}

macro_rules! get_set {
	($getter:ident, $setter:ident, $bit:ident) => {
		pub fn $getter(&self) -> bool {
			(self.bitmask & Self::$bit) != 0
		}
		pub fn $setter(&mut self, val: bool) {
			if val {
				self.bitmask |= Self::$bit;
			} else {
				self.bitmask &= !Self::$bit;
			}
		}
	};
}

impl PlayerConditions {
	const SLOW: u64 = 0x8;
	const SICK: u64 = 0x10;
	const BLIND: u64 = 0x80;
	const HALLUCINATING: u64 = 0x100;
	const DRUNK: u64 = 0x200;
	const CONFUSED: u64 = 0x400;
	const BLEEDING: u64 = 0x8000;
	const HEALING: u64 = 0x20000;
	const IN_COMBAT: u64 = 0x100000;
	const INVINCIBLE: u64 = 0x800000;
	const INVULNERABLE: u64 = 0x1000000;
	const ARMORED: u64 = 0x2000000;
	const ARMOR_BROKEN: u64 = 0x4000000;
	const UNSTABLE: u64 = 0x20000000;
	const DARKNESS: u64 = 0x40000000;

	get_set!(slow, set_slow, SLOW);
	get_set!(sick, set_sick, SICK);
	get_set!(blind, set_blind, BLIND);
	get_set!(hallucinating, set_hallucinating, HALLUCINATING);
	get_set!(drunk, set_drunk, DRUNK);
	get_set!(confused, set_confused, CONFUSED);
	get_set!(bleeding, set_bleeding, BLEEDING);
	get_set!(healing, set_healing, HEALING);
	get_set!(in_combat, set_in_combat, IN_COMBAT);
	get_set!(invincible, set_invincible, INVINCIBLE);
	get_set!(invulnerable, set_invulnerable, INVULNERABLE);
	get_set!(armored, set_armored, ARMORED);
	get_set!(armor_broken, set_armor_broken, ARMOR_BROKEN);
	get_set!(unstable, set_unstable, UNSTABLE);
	get_set!(darkness, set_darkness, DARKNESS);
}

impl PlayerConditions2 {
	const CURSED: u64 = 0x40;
	const EXPOSED: u64 = 0x20000;

	get_set!(cursed, set_cursed, CURSED);
	get_set!(exposed, set_exposed, EXPOSED);
}

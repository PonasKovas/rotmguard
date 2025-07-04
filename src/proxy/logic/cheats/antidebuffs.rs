use crate::proxy::Proxy;

const BLIND_BIT: u64 = 0x80;
const HALLUCINATING_BIT: u64 = 0x100;
const DRUNK_BIT: u64 = 0x200;
const CONFUSED_BIT: u64 = 0x400;
const HEXED_BIT: u64 = 0x8000000;
const UNSTABLE_BIT: u64 = 0x20000000;
const DARKNESS_BIT: u64 = 0x40000000;

/// To be called in NewTick and Update when the condition stat about self is read
/// may modify the stat
pub fn self_condition_stat(proxy: &mut Proxy, stat: &mut i64) {
	let mut bitflags = *stat as u64;

	if proxy.rotmguard.config.settings.debuffs.blind {
		bitflags &= !BLIND_BIT;
	}
	if proxy.rotmguard.config.settings.debuffs.hallucinating {
		bitflags &= !HALLUCINATING_BIT;
	}
	if proxy.rotmguard.config.settings.debuffs.drunk {
		bitflags &= !DRUNK_BIT;
	}
	if proxy.rotmguard.config.settings.debuffs.confused {
		bitflags &= !CONFUSED_BIT;
	}
	if proxy.rotmguard.config.settings.debuffs.hexed {
		bitflags &= !HEXED_BIT;
	}
	if proxy.rotmguard.config.settings.debuffs.unstable {
		bitflags &= !UNSTABLE_BIT;
	}
	if proxy.rotmguard.config.settings.debuffs.darkness {
		bitflags &= !DARKNESS_BIT;
	}

	*stat = bitflags as i64;
}

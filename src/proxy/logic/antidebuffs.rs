use crate::{proxy::Proxy, util::CONDITION_BITFLAG};

/// To be called in NewTick and Update when the condition stat about self is read
/// may modify the stat
pub fn self_condition_stat(proxy: &mut Proxy, stat: &mut i64) {
	let mut bitflags = *stat as u64;

	if proxy.rotmguard.config.settings.debuffs.blind {
		bitflags &= !CONDITION_BITFLAG::BLIND;
	}
	if proxy.rotmguard.config.settings.debuffs.hallucinating {
		bitflags &= !CONDITION_BITFLAG::HALLUCINATING;
	}
	if proxy.rotmguard.config.settings.debuffs.drunk {
		bitflags &= !CONDITION_BITFLAG::DRUNK;
	}
	if proxy.rotmguard.config.settings.debuffs.confused {
		bitflags &= !CONDITION_BITFLAG::CONFUSED;
	}
	if proxy.rotmguard.config.settings.debuffs.hexed {
		bitflags &= !CONDITION_BITFLAG::HEXED;
	}
	if proxy.rotmguard.config.settings.debuffs.unstable {
		bitflags &= !CONDITION_BITFLAG::UNSTABLE;
	}
	if proxy.rotmguard.config.settings.debuffs.darkness {
		bitflags &= !CONDITION_BITFLAG::DARKNESS;
	}

	*stat = bitflags as i64;
}

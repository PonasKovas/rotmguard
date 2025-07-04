use crate::proxy::Proxy;

/// To be called in NewTick and Update when an integer stat about self is read
/// may modify the stat
pub fn self_stat(proxy: &mut Proxy, stat_type: u8, stat: &mut i64) {
	// Only interested in the Condition stat
	if stat_type != 29 {
		return;
	}

	let mut bitflags = *stat as u64;

	if proxy.rotmguard.config.settings.debuffs.blind {
		bitflags &= !0x80;
	}
	if proxy.rotmguard.config.settings.debuffs.hallucinating {
		bitflags &= !0x100;
	}
	if proxy.rotmguard.config.settings.debuffs.drunk {
		bitflags &= !0x200;
	}
	if proxy.rotmguard.config.settings.debuffs.confused {
		bitflags &= !0x400;
	}
	if proxy.rotmguard.config.settings.debuffs.hexed {
		bitflags &= !0x8000000;
	}
	if proxy.rotmguard.config.settings.debuffs.unstable {
		bitflags &= !0x20000000;
	}
	if proxy.rotmguard.config.settings.debuffs.darkness {
		bitflags &= !0x40000000;
	}

	*stat = bitflags as i64;
}

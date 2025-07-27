use crate::{
	proxy::{Proxy, logic::autonexus},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use tracing::warn;

#[derive(TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum AoeEffect {
	None = 0,
	Quiet = 2,
	Weak = 3,
	Slow = 4,
	Sick = 5,
	Dazed = 6,
	Stunned = 7,
	Blind = 8,
	Hallucinating = 9,
	Drunk = 10,
	Confused = 11,
	StunImmune = 12,
	Invisible = 13,
	Paralysed = 14,
	Speedy = 15,
	Bleeding = 16,
	Stasis = 22,
	StasisImmune = 23,
	ArmorBroken = 27,
	NinjaSpeedy = 29,
	Unstable = 30,
	Darkness = 31,
	Petrify = 35,
	PetrifyImmune = 36,
	Curse = 38,
	Silenced = 48,
	Exposed = 49,
	Drought = 61,
	LethalStrike = 62,
	// checked up to 130
}

pub async fn aoe(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let pos_x = View(b, c).try_get_f32()?;
	let pos_y = View(b, c).try_get_f32()?;
	let radius = View(b, c).try_get_f32()?;
	let damage = View(b, c).try_get_u16()?;
	let effect = View(b, c).try_get_u8()?;
	let duration = View(b, c).try_get_f32()?;
	let _orig_type = View(b, c).try_get_u16()?;
	let _color = View(b, c).try_get_u32()?;
	let armor_piercing = View(b, c).try_get_u8()? != 0;

	let effect = match AoeEffect::try_from(effect) {
		Ok(x) => x,
		Err(_) => {
			warn!("unknown aoe effect {effect}");
			AoeEffect::None
		}
	};

	autonexus::aoe(
		proxy,
		pos_x,
		pos_y,
		radius,
		damage,
		effect,
		duration,
		armor_piercing,
	)
	.await;

	Ok(false)
}

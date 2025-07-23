use crate::{
	proxy::{Proxy, logic::autonexus},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn aoe(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let pos_x = View(b, c).try_get_f32()?;
	let pos_y = View(b, c).try_get_f32()?;
	let radius = View(b, c).try_get_f32()?;
	let damage = View(b, c).try_get_u16()?;
	let effect = View(b, c).try_get_u8()?;
	let _duration = View(b, c).try_get_f32()?;
	let _orig_type = View(b, c).try_get_u16()?;
	let _color = View(b, c).try_get_u32()?;
	let armor_piercing = View(b, c).try_get_u8()? != 0;

	autonexus::aoe(proxy, pos_x, pos_y, radius, damage, effect, armor_piercing).await;

	Ok(false)
}

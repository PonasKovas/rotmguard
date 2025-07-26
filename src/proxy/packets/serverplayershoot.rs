use crate::{
	proxy::{Proxy, logic::common},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn serverplayershoot(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let bullet_id = View(b, c).try_get_u16()?;
	let shooter_id = View(b, c).try_get_u32()?;
	let _unknown = View(b, c).try_get_u32()?;
	let _pos_x = View(b, c).try_get_f32()?;
	let _pos_y = View(b, c).try_get_f32()?;
	let _angle = View(b, c).try_get_f32()?;
	let damage = View(b, c).try_get_u16()?;
	let summoner_id = View(b, c).try_get_u32()?;

	let bullet_type = View(b, c).try_get_u8().ok();
	let bullet_count = View(b, c).try_get_u8().ok();
	let _angle_between_bullets = View(b, c).try_get_f32().ok();

	common::serverplayershoot(
		proxy,
		bullet_id,
		shooter_id,
		summoner_id,
		damage,
		bullet_type,
		bullet_count,
	);

	Ok(false)
}

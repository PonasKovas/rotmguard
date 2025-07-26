use crate::{
	proxy::{Proxy, logic::common},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn playershoot(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let _time = View(b, c).try_get_u32()?;
	let bullet_id = View(b, c).try_get_u16()?;
	let weapon_id = View(b, c).try_get_u16()?;
	let projectile_type = View(b, c).try_get_u8()?;
	let _pos_x = View(b, c).try_get_f32()?;
	let _pos_y = View(b, c).try_get_f32()?;
	let _angle = View(b, c).try_get_f32()?;
	let _is_burst = View(b, c).try_get_u8()? != 0;
	let _pattern = View(b, c).try_get_u8()?;
	let _attack_type = View(b, c).try_get_u8()?;
	let _player_pos_x = View(b, c).try_get_f32()?;
	let _player_pos_y = View(b, c).try_get_f32()?;

	common::playershoot(proxy, bullet_id, weapon_id as u32, projectile_type);

	Ok(false)
}

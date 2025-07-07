use crate::{
	proxy::{Proxy, logic::cheats::autonexus},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn enemyshoot(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let bullet_id = View(b, c).try_get_u16()?;
	let owner_id = View(b, c).try_get_u32()?;
	let bullet_type = View(b, c).try_get_u8()?;
	let _pos_x = View(b, c).try_get_f32()?;
	let _pos_y = View(b, c).try_get_f32()?;
	let _angle = View(b, c).try_get_f32()?;
	let damage = View(b, c).try_get_i16()?;

	let (numshots, _angle_between_shots) = if View(b, c).has_remaining() {
		(View(b, c).try_get_u8()?, View(b, c).try_get_f32()?)
	} else {
		(1, 0.0)
	};

	autonexus::new_bullet(proxy, bullet_id, owner_id, bullet_type, damage, numshots).await?;

	Ok(false)
}

use crate::{proxy::Proxy, util::View};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn damage(_proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	// The packet is used to tell the player about damage done to other players and enemies.

	let _target_obj_id = View(b, c).try_get_u32()?;
	let effects_n = View(b, c).try_get_u8()?;
	for _ in 0..effects_n {
		let _effect = View(b, c).try_get_u8()?;
	}
	let _damage_amount = View(b, c).try_get_u16()?;
	let _damage_properties = View(b, c).try_get_u8()?;
	let _bullet_id = View(b, c).try_get_u16()?;
	let _owner_id = View(b, c).try_get_u32()?;

	Ok(false)
}

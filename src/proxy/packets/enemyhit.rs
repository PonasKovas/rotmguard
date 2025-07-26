use crate::{proxy::Proxy, util::View};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn enemyhit(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let _time = View(b, c).try_get_u32()?;
	let bullet_id = View(b, c).try_get_u16()?;
	let shooter_id = View(b, c).try_get_u32()?;
	let target_id = View(b, c).try_get_u32()?;
	let _is_killing = View(b, c).try_get_u8()? != 0;
	let _unknown = View(b, c).try_get_u32()?;

	// damage_monitor::enemyhit(proxy, bullet_id, shooter_id, target_id);

	Ok(false)
}

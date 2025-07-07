use crate::{
	proxy::{Proxy, logic::cheats::autonexus},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn playerhit(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let bullet_id = View(b, c).try_get_u16()?;
	let owner_id = View(b, c).try_get_u32()?;

	autonexus::player_hit(proxy, bullet_id, owner_id).await?;

	Ok(false)
}

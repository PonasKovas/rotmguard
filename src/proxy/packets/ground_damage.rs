use crate::{
	proxy::{Proxy, logic::autonexus},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn ground_damage(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let _time = View(b, c).try_get_u32()?;
	let pos_x = View(b, c).try_get_f32()?;
	let pos_y = View(b, c).try_get_f32()?;

	autonexus::ground_damage(proxy, pos_x as i16, pos_y as i16).await?;

	Ok(false)
}

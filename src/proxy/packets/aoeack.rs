use crate::{
	proxy::{Proxy, logic::autonexus},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn aoeack(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let _time = View(b, c).try_get_u32()?;
	let pos_x = View(b, c).try_get_f32()?;
	let pos_y = View(b, c).try_get_f32()?;

	autonexus::aoeack(proxy, pos_x, pos_y).await?;

	Ok(false)
}

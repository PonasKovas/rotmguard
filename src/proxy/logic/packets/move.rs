use crate::{
	proxy::{Proxy, logic::cheats::autonexus},
	util::View,
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn r#move(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let _tick_id = View(b, c).try_get_u32()?;
	let _time = View(b, c).try_get_u32()?;
	let move_records = View(b, c).try_get_u16()?;

	let mut last_pos = (0.0, 0.0);

	for _ in 0..move_records {
		let _time = View(b, c).try_get_u32()?;
		let pos_x = View(b, c).try_get_f32()?;
		let pos_y = View(b, c).try_get_f32()?;

		last_pos = (pos_x, pos_y);
	}

	proxy.state.position = last_pos;

	autonexus::client_tick_acknowledge(proxy).await;

	Ok(false)
}

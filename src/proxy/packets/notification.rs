use crate::{
	proxy::{
		Proxy,
		logic::{antilag::should_block_object_notification, autonexus, damage_monitor},
	},
	util::{View, read_str},
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn notification(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let notification_type = View(b, c).try_get_u8()?;
	let _extra = View(b, c).try_get_u8()?;

	let mut should_block = false;
	match notification_type {
		6 => {
			// Object text
			let message = read_str(View(b, c))?;
			let object_id = View(b, c).try_get_u32()?;
			let color = View(b, c).try_get_u32()?;

			autonexus::object_notification(proxy, message, object_id, color).await;

			should_block = should_block_object_notification(proxy, object_id, color, message);
		}
		7 => {
			// Player Death
			let json = read_str(View(b, c))?;
			let _picture_type = View(b, c).try_get_u32()?;
			damage_monitor::death_notification(proxy, json);
		}
		_ => {
			let rem = View(b, c).remaining();
			View(b, c).advance(rem);
		}
	}

	Ok(should_block)
}

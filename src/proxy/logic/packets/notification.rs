use crate::{
	proxy::{Proxy, logic::cheats::autonexus},
	util::{View, read_str},
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn notification(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let notification_type = View(b, c).try_get_u8()?;
	let _extra = View(b, c).try_get_u8()?;

	match notification_type {
		6 => {
			let message = read_str(View(b, c))?;
			let object_id = View(b, c).try_get_u32()?;
			let color = View(b, c).try_get_u32()?;

			autonexus::object_notification(proxy, message, object_id, color).await;
		}
		_ => {
			let rem = View(b, c).remaining();
			View(b, c).advance(rem);
		}
	}

	Ok(false)
}

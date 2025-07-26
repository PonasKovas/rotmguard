use crate::{
	proxy::Proxy,
	util::{View, read_str},
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn create_success(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	// The packet is used when joining to tell the client its object id

	let object_id = View(b, c).try_get_u32()?;
	let _char_id = View(b, c).try_get_u32()?;
	let _unknown = read_str(View(b, c))?;

	proxy.state.common.objects.self_id = object_id;

	Ok(false)
}

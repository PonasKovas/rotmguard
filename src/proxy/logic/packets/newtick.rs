use crate::{
	proxy::Proxy,
	util::{OBJECT_STR_STATS, View, read_compressed_int, read_str},
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn newtick(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	// Update object related stats
	///////////////////////////////

	let _tick_id = View(b, c).try_get_u32()?;
	let _tick_time = View(b, c).try_get_u32()?;
	let _real_time_ms = View(b, c).try_get_u32()?;
	let _last_real_time_ms = View(b, c).try_get_u16()?;

	let statuses_n = View(b, c).try_get_u16()?;
	for _ in 0..statuses_n {
		let _object_id = read_compressed_int(View(b, c))? as u32;
		let _position_x = View(b, c).try_get_f32()?;
		let _position_y = View(b, c).try_get_f32()?;
		let n_stats = read_compressed_int(View(b, c))? as usize;

		for _ in 0..n_stats {
			let stat_type = View(b, c).try_get_u8()?;

			if OBJECT_STR_STATS.contains(&stat_type) {
				let _stat = read_str(View(b, c))?;
			} else {
				let _stat = read_compressed_int(View(b, c))?;
			}
			let _secondary = read_compressed_int(View(b, c))?;
		}
	}

	Ok(false)
}

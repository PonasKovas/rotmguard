use crate::{
	proxy::{Proxy, logic::cheats::antidebuffs},
	util::{
		OBJECT_STR_STATS, View, read_compressed_int, read_str, size_as_compressed_int,
		write_compressed_int_exact_size,
	},
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
		let object_id = read_compressed_int(View(b, c))? as u32;
		let _position_x = View(b, c).try_get_f32()?;
		let _position_y = View(b, c).try_get_f32()?;
		let n_stats = read_compressed_int(View(b, c))? as usize;

		for _ in 0..n_stats {
			let stat_type = View(b, c).try_get_u8()?;

			if OBJECT_STR_STATS.contains(&stat_type) {
				let _stat = read_str(View(b, c))?;
			} else {
				let stat_pos = *c;
				let mut stat = read_compressed_int(View(b, c))?;

				if object_id == proxy.state.my_obj_id {
					let original_stat_size = size_as_compressed_int(stat);
					antidebuffs::self_stat(proxy, stat_type, &mut stat);

					// overwrite the stat with the potentially modified one
					write_compressed_int_exact_size(stat, original_stat_size, &mut b[stat_pos..]);
				}
			}
			let _secondary = read_compressed_int(View(b, c))?;
		}
	}

	Ok(false)
}

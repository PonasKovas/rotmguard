use super::CONDITION_STAT_ID;
use crate::{
	proxy::{
		Proxy,
		logic::cheats::{antidebuffs, fakeslow},
	},
	util::{
		OBJECT_STR_STATS, View, read_compressed_int, read_str, size_as_compressed_int,
		write_compressed_int, write_compressed_int_exact_size,
	},
};
use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};

pub async fn newtick(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	// Update object related stats
	///////////////////////////////

	let _tick_id = View(b, c).try_get_u32()?;
	let _tick_time = View(b, c).try_get_u32()?;
	let _real_time_ms = View(b, c).try_get_u32()?;
	let _last_real_time_ms = View(b, c).try_get_u16()?;

	let statuses_n_pos = *c;
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

				// if status about self and is condition stat
				if object_id == proxy.state.my_obj_id && stat_type == CONDITION_STAT_ID {
					proxy.state.condition = stat as u64;

					let original_stat_size = size_as_compressed_int(stat);
					antidebuffs::self_condition_stat(proxy, &mut stat);
					fakeslow::self_condition_stat(proxy, &mut stat);

					// overwrite the stat with the potentially modified one
					write_compressed_int_exact_size(stat, original_stat_size, &mut b[stat_pos..]);
				}
			}
			let _secondary = read_compressed_int(View(b, c))?;
		}
	}

	if let Some((obj_id, (pos_x, pos_y), stats)) = fakeslow::extra_object_status(proxy) {
		// welp... gotta allocate a new buffer to add extra data
		let mut new_buf = BytesMut::with_capacity(
			b.len()
			+ size_as_compressed_int(obj_id as i64) // object id
			+ 2 * 4 // 2 f32s (position)
			+ size_as_compressed_int(stats.len() as i64) // stats_n
			+ stats.len() * 21, // each stat is a u8 and two varints (maximum 10 bytes each)
		);

		new_buf.extend_from_slice(&b);

		// increase the statuses_n
		(&mut new_buf[statuses_n_pos..]).put_u16(statuses_n + 1);

		// add the new status
		write_compressed_int(obj_id as i64, &mut new_buf);
		new_buf.put_f32(pos_x);
		new_buf.put_f32(pos_y);
		write_compressed_int(stats.len() as i64, &mut new_buf);
		for (stat_type, stat, secondary) in stats {
			new_buf.put_u8(stat_type);
			write_compressed_int(stat, &mut new_buf);
			write_compressed_int(secondary, &mut new_buf);
		}

		// then copy all remaining bytes and replace the original buffer with the new buffer
		let new_cursor = new_buf.len();
		*b = new_buf;
		*c = new_cursor;
	}

	Ok(false)
}

use crate::{
	proxy::{
		Proxy,
		logic::cheats::{antidebuffs, autonexus, fakeslow},
	},
	util::{
		OBJECT_STR_STATS, STAT_TYPE, View, read_compressed_int, read_str, size_as_compressed_int,
		write_compressed_int, write_compressed_int_exact_size, write_str,
	},
};
use anyhow::Result;
use bytes::{Buf, BufMut, BytesMut};
use either::Either;
use tracing::error;

pub struct ExtraObject<I> {
	pub obj_id: u32,
	pub pos_x: f32,
	pub pos_y: f32,
	pub stats: I,
}

pub struct StatData<'a> {
	pub stat_type: u8,
	pub data: Either<&'a str, i64>,
	pub secondary: i64,
}

pub async fn newtick(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	// Update object related stats
	///////////////////////////////

	let tick_id = View(b, c).try_get_u32()?;
	let tick_time = View(b, c).try_get_u32()?;
	let _real_time_ms = View(b, c).try_get_u32()?;
	let _last_real_time_ms = View(b, c).try_get_u16()?;

	autonexus::new_tick_start(proxy, tick_id, tick_time);

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
				if object_id == proxy.state.my_obj_id {
					let original_stat_size = size_as_compressed_int(stat);

					autonexus::self_stat(proxy, stat_type, stat).await;

					if stat_type == STAT_TYPE::CONDITION {
						antidebuffs::self_condition_stat(proxy, &mut stat);
						fakeslow::self_condition_stat(proxy, &mut stat);

						// overwrite the stat with the potentially modified one
						write_compressed_int_exact_size(
							stat,
							original_stat_size,
							&mut b[stat_pos..],
						);
					}
				}
			}
			let _secondary = read_compressed_int(View(b, c))?;
		}
	}

	if View(b, c).has_remaining() {
		error!("adding new object status data elements but there are unread bytes??");
		error!("{:?}", View(b, c).slice());
	}

	// now if any cheats want to add extra objects
	for extra_object in fakeslow::extra_object_status(proxy) {
		add_extra(b, statuses_n_pos, extra_object);
	}
	for extra_object in autonexus::extra_object_status(proxy) {
		add_extra(b, statuses_n_pos, extra_object);
	}

	// move the cursor to the end to avoid triggering the warning later
	*c = b.len();

	autonexus::new_tick_finish(proxy).await;

	Ok(false)
}

fn add_extra<'a, I: Iterator<Item = StatData<'a>> + ExactSizeIterator>(
	b: &mut BytesMut,
	statuses_n_pos: usize,
	data: ExtraObject<I>,
) {
	// reserve the space
	b.reserve(
		size_as_compressed_int(data.obj_id as i64) // object id
			+ 2 * 4 // 2 f32s (position)
			+ size_as_compressed_int(data.stats.len() as i64) // stats_n
			+ data.stats.len() * 21, // each stat is a u8 and two varints (optimizing for integer stats here)
	);

	// increase the statuses_n
	let old = View(b, &mut { statuses_n_pos }).get_u16();
	(&mut b[statuses_n_pos..]).put_u16(old + 1);

	// add the new status
	write_compressed_int(data.obj_id as i64, &mut *b);
	b.put_f32(data.pos_x);
	b.put_f32(data.pos_y);
	write_compressed_int(data.stats.len() as i64, &mut *b);
	for stat in data.stats {
		b.put_u8(stat.stat_type);
		match stat.data {
			Either::Left(s) => {
				write_str(s, &mut *b);
			}
			Either::Right(i) => {
				write_compressed_int(i, &mut *b);
			}
		}
		write_compressed_int(stat.secondary, &mut *b);
	}
}

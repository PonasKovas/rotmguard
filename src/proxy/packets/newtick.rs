use crate::{
	proxy::{
		Proxy,
		logic::{antidebuffs, autonexus, damage_monitor, fakeslow},
		packets::common::parse_object_data,
	},
	util::{
		STAT_TYPE, View, size_as_compressed_int, write_compressed_int,
		write_compressed_int_exact_size, write_str,
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
		let mut dmg_monitor_processor;

		parse_object_data!(b, c;
			object(object_id, _pos_x, _pos_y) => {
				dmg_monitor_processor = damage_monitor::ObjectStatusProcessor::update(object_id);
			};
			int_stat(stat_type, stat) => {
				dmg_monitor_processor.add_int_stat(stat_type, stat);

				// if status about self and is condition stat
				if object_id == proxy.state.my_obj_id {
					let original_stat_size = size_as_compressed_int(stat);

					autonexus::self_stat(proxy, stat_type, stat).await;

					if stat_type == STAT_TYPE::CONDITION {
						let mut new_stat = stat;
						antidebuffs::self_condition_stat(proxy, &mut new_stat);
						fakeslow::self_condition_stat(proxy, &mut new_stat);

						// overwrite the stat with the potentially modified one
						let stat_pos = *c - original_stat_size;
						write_compressed_int_exact_size(
							new_stat,
							original_stat_size,
							&mut b[stat_pos..],
						);
					}
				}
			};
			str_stat(stat_type, stat) => {
				dmg_monitor_processor.add_str_stat(stat_type, stat);
			};
		);

		dmg_monitor_processor.finish(proxy);
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

//! Stuff that may repeat in different packets

macro_rules! parse_object_data {
	($b:expr, $c:expr;
    	object ($id:pat, $pos_x:pat, $pos_y:pat) => $object_code:tt $(;)?
    	int_stat ($int_stat_type:pat, $int_stat:pat) => $int_code:tt $(;)?
    	str_stat ($str_stat_type:pat, $str_stat:pat) => $str_code:tt $(;)?
    ) => {
		let $id = crate::util::read_compressed_int(crate::util::View($b, $c))? as u32;
		let $pos_x = crate::util::View($b, $c).try_get_f32()?;
		let $pos_y = crate::util::View($b, $c).try_get_f32()?;

		$object_code

		let __n_stats = crate::util::read_compressed_int(crate::util::View($b, $c))? as usize;

		for _ in 0..__n_stats {
			let __stat_type = crate::util::View($b, $c).try_get_u8()?;

			if crate::util::OBJECT_STR_STATS.contains(&__stat_type) {
				let $str_stat_type = __stat_type;
				let $str_stat = crate::util::read_str(View($b, $c))?;

				$str_code
			} else {
				let $int_stat_type = __stat_type;
				let $int_stat = crate::util::read_compressed_int(crate::util::View($b, $c))?;

				$int_code
			}
			let __secondary = crate::util::read_compressed_int(crate::util::View($b, $c))?;
		}
	};
}

pub(crate) use parse_object_data;

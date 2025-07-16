use crate::{
	proxy::{Proxy, logic::cheats::damage_monitor},
	util::{View, read_str},
};
use anyhow::Result;
use bytes::{Buf, BytesMut};

pub async fn mapinfo(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	let _width = View(b, c).try_get_u32()?;
	let _height = View(b, c).try_get_u32()?;
	let name = read_str(View(b, c))?;
	let _display_name = read_str(View(b, c))?;
	let _realm_name = read_str(View(b, c))?;
	let _seed = View(b, c).try_get_u32()?;
	let _background = View(b, c).try_get_u32()?; // ? this is not the color tho...
	let _difficulty = View(b, c).try_get_f32()?;
	let _allow_teleport = View(b, c).try_get_u8()? != 0;
	let _no_save = View(b, c).try_get_u8()? != 0; // ?
	let _show_displays = View(b, c).try_get_u8()? != 0; // ?
	let _max_players = View(b, c).try_get_u16()?;
	let _game_opened_time = View(b, c).try_get_u32()?;
	let _build_version = read_str(View(b, c))?;
	let _background_color = View(b, c).try_get_u32()?;

	// more data but format is not really known and we dont care
	// supress warning that not all bytes were parsed
	*c = b.len();

	damage_monitor::set_map_name(proxy, name);

	Ok(false)
}

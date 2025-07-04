use crate::{
	proxy::{
		Proxy,
		logic::cheats::{antipush, con},
	},
	util::{BLUE, View, read_str, static_notification},
};
use anyhow::Result;
use bytes::BytesMut;

pub async fn playertext(proxy: &mut Proxy, b: &mut BytesMut, c: &mut usize) -> Result<bool> {
	// The packet is used to handle commands

	let text = read_str(View(b, c))?;

	// not interested in stuff that isnt a command
	// - must start with `/`
	// - must have some text after the `/`, not just `/` alone
	// - the first character after `/` must be either a letter or a number, not a space or something.
	if !text.starts_with('/') && text.len() > 1 && text.chars().nth(1).unwrap().is_alphanumeric() {
		return Ok(false);
	}

	let mut args = text.split(' ');

	let command = args.next().unwrap();
	match command {
		"/hi" | "/rotmguard" => {
			let notification = static_notification!("hi :)", BLUE);
			proxy.send_client(notification.clone()).await;

			Ok(true)
		}
		"/con" => {
			con::con(proxy, args).await;

			Ok(true)
		}
		"/ap" | "/antipush" => {
			antipush::toggle(proxy).await;

			Ok(true)
		}
		_ => Ok(false),
	}
}

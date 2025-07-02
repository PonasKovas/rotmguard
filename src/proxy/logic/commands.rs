use crate::{
	protocol::{PACKET_ID::C2S_PLAYERTEXT, read_str, util::create_notification},
	proxy::Proxy,
};
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use std::sync::OnceLock;

mod con;

pub async fn handle_commands(proxy: &mut Proxy, packet_bytes: &mut BytesMut) -> Result<bool> {
	// only interested in Player Text packet
	if packet_bytes[0] != C2S_PLAYERTEXT {
		return Ok(false);
	}

	let text = read_str(&packet_bytes[1..])?;

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
			static NOTIFICATION: OnceLock<Bytes> = OnceLock::new();
			let notification = NOTIFICATION.get_or_init(|| create_notification("hi :)", 0xb603fc));

			proxy.send_client(notification.clone()).await;

			Ok(true)
		}
		"/con" => {
			con::con(proxy, args).await;

			Ok(true)
		}
		_ => Ok(false),
	}
}

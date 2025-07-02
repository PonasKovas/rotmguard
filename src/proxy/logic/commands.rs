use crate::{
	protocol::{
		read_str,
		util::{create_notification, static_notification},
	},
	proxy::Proxy,
};
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use std::sync::OnceLock;

mod con;

pub async fn commands(proxy: &mut Proxy, packet_bytes: &mut BytesMut) -> Result<bool> {
	// the packet is a PlayerText packet
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
		"/ap" | "/antipush" => {
			proxy.state.antipush_enabled = !proxy.state.antipush_enabled;
			proxy.state.antipush_synced = false;

			let notification = if proxy.state.antipush_enabled {
				static_notification!("Anti push enabled", 0xb603fc)
			} else {
				static_notification!("Anti push disabled", 0x9103fc)
			};

			proxy.send_client(notification).await;

			Ok(true)
		}
		_ => Ok(false),
	}
}

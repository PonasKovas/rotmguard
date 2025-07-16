use crate::{
	proxy::{
		Proxy,
		logic::cheats::{antipush, autonexus, con, fakeslow},
	},
	util::{BLUE, GREEN, RED, View, create_notification, read_str, static_notification},
};
use anyhow::Result;
use bytes::BytesMut;
use std::sync::atomic::Ordering;

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
			proxy.send_client(notification).await;

			Ok(true)
		}
		"/devmode" => {
			let state = {
				let mut dev_mode = proxy.rotmguard.config.settings.dev_mode.lock().unwrap();
				*dev_mode = !*dev_mode;
				*dev_mode
			};

			let notification = if state {
				static_notification!("developer mode on", GREEN)
			} else {
				static_notification!("developer mode off", RED)
			};
			proxy.send_client(notification).await;

			Ok(true)
		}
		"/flushskips" => {
			let total = proxy
				.rotmguard
				.flush_skips
				.total_packets
				.load(Ordering::Relaxed);
			let flushes = proxy.rotmguard.flush_skips.flushes.load(Ordering::Relaxed);
			let total_time = proxy
				.rotmguard
				.flush_skips
				.total_time
				.load(Ordering::Relaxed);

			let percent_flushed = 100.0 * flushes as f32 / total as f32;
			let avg_delay = total_time as f32 / total as f32;
			proxy
				.send_client(create_notification(
					&format!(
						"Total packets: {total}. Flushed: {percent_flushed:.2}%. Avg delay: {avg_delay:.2}us"
					),
					BLUE,
				))
				.await;

			Ok(true)
		}
		"/autonexus" => {
			autonexus::command(proxy, args).await;

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
		"/slow" => {
			fakeslow::toggle(proxy).await;

			Ok(true)
		}
		_ => Ok(false),
	}
}

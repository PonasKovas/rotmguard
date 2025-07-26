use crate::{
	proxy::{
		Proxy,
		logic::{antipush, autonexus, con, fakeslow},
	},
	util::{BLUE, GREEN, RED, View, read_str, static_notification},
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

	let mut args = text.split(' ').map(|s| s.trim()).filter(|s| !s.is_empty());

	let command = args.next().unwrap();
	match command {
		"/hi" | "/rotmguard" => {
			let notification = static_notification!("hi :)", BLUE);
			proxy.send_client(notification).await;

			Ok(true)
		}
		"/dmg" => {
			// damage_monitor::generate_report(proxy).await;

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
		"/antilag" => {
			let state = {
				let mut antilag = proxy.rotmguard.config.settings.antilag.lock().unwrap();
				*antilag = !*antilag;
				*antilag
			};

			let notification = if state {
				static_notification!("antilag on", GREEN)
			} else {
				static_notification!("antilag off", RED)
			};

			proxy.send_client(notification).await;

			Ok(true)
		}
		_ => Ok(false),
	}
}

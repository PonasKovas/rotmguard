use std::time::{Duration, Instant};

use crate::{
	config,
	constants::SERVERS,
	extra_datatypes::WorldPos,
	logging::save_logs,
	packets::{NotificationPacket, Reconnect, ShowEffect},
	proxy::Proxy,
};
use anyhow::Result;
use rand::{thread_rng, Rng};

use super::util::Notification;

pub async fn command(proxy: &mut Proxy, text: &str) -> Result<bool> {
	// `/hi`, `/rotmguard` are simple commands that send a notification
	// useful for checking if you are connected through the proxy.
	if text.starts_with("/hi") || text.starts_with("/rotmguard") {
		let colors = [0xff8080, 0xff8080, 0x80ffac, 0x80c6ff, 0xc480ff];
		let color = colors[thread_rng().gen_range(0..colors.len())];

		Notification::new("hi :)".to_owned())
			.color(color)
			.send(proxy)
			.await?;

		let packet = ShowEffect {
			effect_type: 1,
			target_object_id: Some(proxy.rotmguard.my_object_id),
			pos1: WorldPos { x: 0.0, y: 0.0 },
			pos2: WorldPos { x: 1.0, y: 1.0 },
			color: Some(color),
			duration: Some(5.0),
			unknown: None,
		};
		proxy.send_client(&packet.into()).await?;

		let packet = ShowEffect {
			effect_type: 37,
			target_object_id: Some(proxy.rotmguard.my_object_id),
			pos1: WorldPos { x: 0.0, y: 0.0 },
			pos2: WorldPos { x: 0.0, y: 0.0 },
			color: Some(color),
			duration: Some(0.5),
			unknown: None,
		};
		proxy.send_client(&packet.into()).await?;

		return Ok(false); // dont forward this :)
	} else
	// `/effect <effect id>` allows you to test different visual effects
	// to maybe use them somewhere in this program
	if text.starts_with("/effect ") {
		match text.split(" ").nth(1).unwrap().parse::<u8>() {
			Ok(id) => {
				let packet = ShowEffect {
					effect_type: id,
					target_object_id: Some(proxy.rotmguard.my_object_id),
					pos1: WorldPos { x: 5.0, y: 0.0 },
					pos2: WorldPos { x: 0.0, y: 0.0 },
					color: Some(0xffffff),
					duration: Some(0.5),
					unknown: None,
				};
				proxy.send_client(&packet.into()).await?;
			}
			Err(e) => {
				let packet = NotificationPacket::ErrorMessage {
					text: format!("{e}"),
				};
				proxy.send_client(&packet.into()).await?;
			}
		}
		return Ok(false);
	} else
	// `/fn <name>`, `/name <name>`, `/fakename <name>` allow you to set a fake name
	// Useful to hide your real identity in videos and screenshots or to pretend to be someone else
	// if youre goofy enough 😊
	if text.starts_with("/fn") || text.starts_with("/name") || text.starts_with("/fakename")
	{
		let fake_name = match text.split(" ").nth(1) {
			Some(n) => n.to_owned(),
			None => {
				// generate a random name
				let mut random_name = String::with_capacity(10);
				let chars = "rotmguard"; // a goofy little easter egg 😊
				for _ in 0..10 {
					random_name.push(
						chars
							.chars()
							.nth(thread_rng().gen::<usize>() % chars.len())
							.unwrap(),
					);
				}

				random_name
			}
		};

		proxy.rotmguard.fake_name = Some(fake_name.clone());
		config().settings.lock().unwrap().fakename = Some(fake_name);

		return Ok(false);
	} else
	// `/recsc [seconds]`, `/reccs [seconds]` allow you to save all incoming or outgoing (recsc and reccs respectively)
	// packets for specified number of seconds (or 1 second if unspecified).
	// Useful for inspecting packets and figuring out what they mean 🤯
	if text.starts_with("/recsc") || text.starts_with("/reccs") {
		let time = match text.split(" ").nth(1) {
			Some(t) => match t.parse::<f32>() {
				Ok(t) => t,
				Err(e) => {
					Notification::new(format!("Invalid time period: {e}"))
						.color(0xff3333)
						.send(proxy)
						.await?;

					return Ok(false);
				}
			},
			None => 1.0,
		};

		let message = if text.starts_with("/recsc") {
			proxy.rotmguard.record_sc_until = Some(Instant::now() + Duration::from_secs_f32(time));

			format!("Recording server->client for {time} s")
		} else {
			proxy.rotmguard.record_cs_until = Some(Instant::now() + Duration::from_secs_f32(time));

			format!("Recording client->server for {time} s")
		};
		Notification::new(message)
			.color(0x33ff33)
			.send(proxy)
			.await?;

		return Ok(false);
	} else
	// `/sync` synchronizes the client hp with server hp
	// usually this is done automatically, only useful when developing
	if text.starts_with("/sync") {
		proxy.rotmguard.hp = proxy.rotmguard.player_stats.server_hp as f64;

		return Ok(false);
	} else
	// `/con <server>` quickly and conveniently connects you to the specified server
	// Use a short name for the server: for example if you wanted to connect to EUEast, type eue
	if text.starts_with("/con") {
		let srv = match text.split(" ").nth(1) {
			Some(s) => s,
			None => {
				Notification::new("Specify a server. Example: eue".to_owned())
					.color(0xff3333)
					.send(proxy)
					.await?;

				return Ok(false);
			}
		};

		match SERVERS.get(&srv.to_lowercase()) {
			Some(ip) => {
				let packet = Reconnect {
					hostname: "have fun :)".to_owned(),
					address: ip.to_string(),
					port: 2050,
					game_id: 0xfffffffe,
					key_time: 0xffffffff,
					key: Vec::new(),
				};
				proxy.send_client(&packet.into()).await?;
			}
			None => {
				Notification::new(format!("Server {srv:?} is invalid."))
					.color(0xff3333)
					.send(proxy)
					.await?;
			}
		}

		return Ok(false);
	} else
	// `/devmode` toggles developer mode
	if text.starts_with("/devmode") {
		let message = {
			let mut settings = config().settings.lock().unwrap();

			settings.dev_mode = !settings.dev_mode;

			format!(
				"DEVELOPER MODE {}",
				if settings.dev_mode { "ON" } else { "OFF" }
			)
		};

		Notification::new(message)
			.color(0xffffff)
			.send(proxy)
			.await?;

		return Ok(false);
	} else
	// `/savelogs` saves the rotmguard logs recorded until this moment
	// useful when a bug is noticed to save for further inspection
	// When the player dies the logs are saved automatically
	if text.starts_with("/savelogs") {
		save_logs();
		Notification::new("Logs saved".to_owned())
			.color(0x33ff33)
			.send(proxy)
			.await?;

		return Ok(false);
	}

	Ok(true)
}

use rand::{thread_rng, Rng};

use super::Module;
use crate::{
	extra_datatypes::WorldPos,
	packets::{ClientPacket, ServerPacket, ShowEffect},
	proxy::Proxy,
	util::Notification,
};
use std::io::Result;

pub struct Commands {}

impl Module for Commands {
	async fn client_packet(
		&mut self,
		proxy: &mut Proxy,
		packet: &mut ClientPacket,
	) -> Result<bool> {
		let text = match packet {
			ClientPacket::PlayerText(text) => &text.text,
			_ => return Ok(true),
		};

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
		}

		Ok(true)
	}
	async fn server_packet(
		&mut self,
		proxy: &mut Proxy,
		packet: &mut ServerPacket,
	) -> Result<bool> {
		Ok(true)
	}
	async fn disconnect(&mut self, proxy: &mut Proxy, by_server: bool) -> Result<()> {
		Ok(())
	}
}

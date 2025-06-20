use super::{Module, ModuleInstance, PacketFlow, FORWARD};
use crate::{
	extra_datatypes::{ObjectId, WorldPos},
	gen_this_macro,
	module::BLOCK,
	packets::{ClientPacket, ServerPacket, ShowEffect},
	proxy::Proxy,
	util::notification::Notification,
};
use anyhow::{bail, Result};
use rand::{thread_rng, Rng};
use tracing::info;

gen_this_macro! {general}

#[derive(Debug, Clone)]
pub struct General {}

#[derive(Debug, Clone)]
pub struct GeneralInst {
	// the player's object id
	pub my_object_id: ObjectId,
	// the most recent tick that is to be acknowledged by the client
	pub client_tick_id: u32,
}

impl Module for General {
	type Instance = GeneralInst;

	fn new() -> Self {
		General {}
	}
	fn instance(&self) -> Self::Instance {
		GeneralInst {
			my_object_id: ObjectId(0),
			client_tick_id: 0,
		}
	}
}

impl ModuleInstance for GeneralInst {
	async fn client_packet(proxy: &mut Proxy, packet: &mut ClientPacket) -> Result<PacketFlow> {
		match packet {
			ClientPacket::Move(move_packet) => {
				// this is basically client acknowledging a tick.

				if move_packet.tick_id != general!(proxy).client_tick_id {
					bail!(
						"Client acknowledging tick {} even though {} was expected",
						move_packet.tick_id,
						general!(proxy).client_tick_id
					);
				}

				general!(proxy).client_tick_id += 1;
			}
			ClientPacket::PlayerText(text) => {
				let text = &text.text;

				// `/hi`, `/rotmguard` are simple commands that send a notification
				// useful for checking if you are connected through the proxy.
				if text.starts_with("/hi") || text.starts_with("/rotmguard") {
					let colors = [0xff8080, 0xff8080, 0x80ffac, 0x80c6ff, 0xc480ff];
					let color = colors[thread_rng().gen_range(0..colors.len())];

					Notification::new("hi :)".to_owned())
						.color(color)
						.send(proxy)
						.await;

					let packet = ShowEffect {
						effect_type: 1,
						target_object_id: Some(general!(proxy).my_object_id),
						pos1: WorldPos { x: 0.0, y: 0.0 },
						pos2: WorldPos { x: 1.0, y: 1.0 },
						color: Some(color),
						duration: Some(5.0),
						unknown: None,
					};
					proxy.client.send(packet.into()).await?;

					let packet = ShowEffect {
						effect_type: 37,
						target_object_id: Some(general!(proxy).my_object_id),
						pos1: WorldPos { x: 0.0, y: 0.0 },
						pos2: WorldPos { x: 0.0, y: 0.0 },
						color: Some(color),
						duration: Some(0.5),
						unknown: None,
					};
					proxy.client.send(packet.into()).await?;

					info!(?proxy.modules, "hi ☺️");

					return BLOCK;
				}
				// `/devmode` toggles developer mode
				if text.starts_with("/devmode") {
					let message = {
						let mut dev_mode = proxy.config.settings.dev_mode.lock().unwrap();

						*dev_mode = !*dev_mode;

						format!("DEVELOPER MODE {}", if *dev_mode { "ON" } else { "OFF" })
					};

					Notification::new(message).green().send(proxy).await;

					return BLOCK;
				}
			}
			_ => {}
		}

		FORWARD
	}
	async fn server_packet(proxy: &mut Proxy, packet: &mut ServerPacket) -> Result<PacketFlow> {
		match packet {
			ServerPacket::CreateSuccess(create_success) => {
				general!(proxy).my_object_id = create_success.object_id;
			}
			_ => {}
		}

		FORWARD
	}
}

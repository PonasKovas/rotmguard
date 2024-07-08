use super::{Module, ModuleInstance};
use crate::{
	extra_datatypes::{ObjectId, WorldPos},
	packets::{ClientPacket, ServerPacket, ShowEffect},
	proxy::Proxy,
	util::Notification,
};
use rand::{thread_rng, Rng};
use std::io::Result;
use tracing::{error, info, instrument};

#[derive(Debug, Clone)]
pub struct General {}

#[derive(Debug, Clone)]
pub struct GeneralInst {
	// the player's object id
	pub my_object_id: ObjectId,
	// the current tick id of the last tick that was received from the server
	pub tick_id: u32,
}

impl Module for General {
	type Instance = GeneralInst;

	fn new() -> Self {
		General {}
	}
	fn instance(&self) -> Self::Instance {
		GeneralInst {
			my_object_id: ObjectId(0),
			tick_id: 0,
		}
	}
}

impl ModuleInstance for GeneralInst {
	#[instrument(skip(proxy), fields(modules = ?proxy.modules))]
	async fn client_packet(proxy: &mut Proxy, packet: &mut ClientPacket) -> Result<bool> {
		match packet {
			ClientPacket::Move(move_packet) => {
				// this is basically client acknowledging a tick.
				// so we can start reading data about the next one

				if move_packet.tick_id != proxy.modules.general.tick_id {
					error!(
						client_tick_id = move_packet.tick_id,
						server_tick_id = proxy.modules.general.tick_id,
						"Client and server tick IDs dont match!"
					);
				}

				proxy.pause_server_read = false;
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
						.await?;

					let packet = ShowEffect {
						effect_type: 1,
						target_object_id: Some(proxy.modules.general.my_object_id),
						pos1: WorldPos { x: 0.0, y: 0.0 },
						pos2: WorldPos { x: 1.0, y: 1.0 },
						color: Some(color),
						duration: Some(5.0),
						unknown: None,
					};
					proxy.send_client(&packet.into()).await?;

					let packet = ShowEffect {
						effect_type: 37,
						target_object_id: Some(proxy.modules.general.my_object_id),
						pos1: WorldPos { x: 0.0, y: 0.0 },
						pos2: WorldPos { x: 0.0, y: 0.0 },
						color: Some(color),
						duration: Some(0.5),
						unknown: None,
					};
					proxy.send_client(&packet.into()).await?;

					info!("{:?}", proxy.modules);

					return Ok(false); // dont forward this :)
				}
			}
			_ => {}
		}

		Ok(true)
	}
	#[instrument(skip(proxy), fields(modules = ?proxy.modules))]
	async fn server_packet(proxy: &mut Proxy, packet: &mut ServerPacket) -> Result<bool> {
		match packet {
			ServerPacket::CreateSuccess(create_success) => {
				proxy.modules.general.my_object_id = create_success.object_id;
			}
			ServerPacket::NewTick(new_tick) => {
				proxy.modules.general.tick_id = new_tick.tick_id;

				proxy.pause_server_read = true; // pause reading further from server until client acknowledges this tick
			}
			_ => {}
		}
		Ok(true)
	}
	#[instrument(skip( proxy), fields(modules = ?proxy.modules))]
	async fn disconnect(proxy: &mut Proxy, by_server: bool) -> Result<()> {
		Ok(())
	}
}

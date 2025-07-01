use super::Proxy;
use crate::{
	Rotmguard,
	protocol::packets::{
		C2SPacket, S2CPacket, newtick::create_newtick, notification::create_notification,
	},
};
use anyhow::{Result, bail};
use bytes::Bytes;
use std::sync::OnceLock;

mod antidebuffs;
mod con;

pub struct State {}

pub async fn initialize(rotmguard: &Rotmguard) -> Result<State> {
	let s = State {};

	Ok(s)
}

pub async fn handle_c2s_packet(proxy: &mut Proxy, packet_bytes: Bytes) -> Result<()> {
	let packet = match C2SPacket::parse(&mut packet_bytes.clone()) {
		Ok(Some(p)) => p,
		Ok(None) => {
			// unknown packet. literally dont care about it. forward
			proxy.send_server(packet_bytes).await;

			return Ok(());
		}
		Err(e) => {
			bail!("Error parsing c2s packet: {e}")
		}
	};

	match packet {
		C2SPacket::PlayerText(player_text) => {
			let text = player_text.text.trim();

			if text.starts_with('/') && text.len() >= 2 {
				// commands
				let mut args = text[..].split(' ');

				let command = args.next().unwrap();
				match command {
					"/hi" | "/rotmguard" => {
						static NOTIFICATION: OnceLock<Bytes> = OnceLock::new();
						let notification =
							NOTIFICATION.get_or_init(|| create_notification("hi :)", 0xb603fc));

						proxy.send_client(notification.clone()).await;
					}
					"/con" => con::con(proxy, args).await,
					_ => {
						// some other command
						proxy.send_server(packet_bytes).await; // forward
					}
				}

				return Ok(());
			}
		}
	}

	proxy.send_server(packet_bytes).await;

	Ok(())
}

pub async fn handle_s2c_packet(proxy: &mut Proxy, packet_bytes: Bytes) -> Result<()> {
	let packet = match S2CPacket::parse(&mut packet_bytes.clone()) {
		Ok(Some(p)) => p,
		Ok(None) => {
			// unknown packet. literally dont care about it. forward
			proxy.send_client(packet_bytes).await;

			return Ok(());
		}
		Err(e) => {
			bail!("Error parsing s2c packet: {e}")
		}
	};

	match packet {
		S2CPacket::Notification(notification) => {}
		S2CPacket::Reconnect(reconnect) => {}
		S2CPacket::NewTick(new_tick) => {
			let mut copy = create_newtick(
				new_tick.tick_id,
				new_tick.tick_time,
				new_tick.real_time_ms,
				new_tick.last_real_time_ms,
			);

			for obj in new_tick.statuses.into_iter() {
				let obj = obj?;
				copy.add_object(obj.object_id, obj.position_x, obj.position_y);
				for stat in obj.stats.into_iter() {
					let stat = stat?;
					copy.add_stat(stat);
				}
			}

			proxy.send_client(copy.finish()).await;

			return Ok(());
		}
	}

	proxy.send_client(packet_bytes).await;

	Ok(())
}

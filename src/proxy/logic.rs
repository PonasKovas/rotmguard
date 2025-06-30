use super::Proxy;
use crate::protocol::packets::{C2SPacket, S2CPacket, notification::create_notification};
use anyhow::{Result, bail};
use bytes::Bytes;
use std::sync::OnceLock;

mod con;

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
					"/hi" => {
						static NOTIFICATION: OnceLock<Bytes> = OnceLock::new();
						let notification =
							NOTIFICATION.get_or_init(|| create_notification("hi :)", 0xb603fc));

						proxy.send_client(notification.clone()).await;

						return Ok(()); // dont forward
					}
					"/con" => con::con(proxy, args),
					_ => {}
				}
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
	}

	proxy.send_client(packet_bytes).await;

	Ok(())
}

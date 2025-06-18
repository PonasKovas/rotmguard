use super::Proxy;
use crate::protocol::{RPRead, packet_ids::C2S::PLAYERTEXT, packets::PlayerText};
use anyhow::Result;
use bytes::Bytes;

mod con;

pub async fn handle_c2s_packet(proxy: &mut Proxy, packet: Bytes) -> Result<()> {
	if packet[0] == PLAYERTEXT {
		let text = PlayerText::rp_read(&mut &packet[1..])?.trim();
		if text.starts_with('/') && text.len() >= 2 {
			// commands
			let mut args = text[1..].split(' ');

			let command = args.next().unwrap();
			match command {
				"hi" => {
					// todo
				}
				"con" => con::con(proxy, args),
				_ => {}
			}
		}
	}
	proxy.send_server(packet).await;

	Ok(())
}

pub async fn handle_s2c_packet(proxy: &mut Proxy, packet: Bytes) -> Result<()> {
	proxy.send_client(packet).await;

	Ok(())
}

use super::Proxy;
use crate::packet_ids::{C2S, S2C};
use anyhow::Result;
use bytes::Bytes;

pub async fn handle_c2s_packet(proxy: &mut Proxy, packet: Bytes) -> Result<()> {
	let id = packet[0];
	let to_flush = [C2S::LOAD, C2S::MOVE, C2S::HELLO, C2S::ESCAPE].contains(&id);

	proxy.send_server(packet).await;

	if to_flush {
		proxy.flush_server().await;
	}

	Ok(())
}

pub async fn handle_s2c_packet(proxy: &mut Proxy, packet: Bytes) -> Result<()> {
	let id = packet[0];
	let to_flush = [S2C::FAILURE, S2C::NEWTICK, S2C::RECONNECT, S2C::MAPINFO].contains(&id);

	proxy.send_client(packet).await;

	if to_flush {
		proxy.flush_client().await;
	}

	Ok(())
}

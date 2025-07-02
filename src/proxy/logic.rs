use super::Proxy;
use crate::{Rotmguard, protocol::PACKET_ID};
use anyhow::Result;
use bytes::BytesMut;
use std::collections::BTreeMap;

mod commands;
mod update_packet;

pub struct State {
	hazardous_tiles: BTreeMap<(i16, i16), i64>, // position -> damage
	conveyor_tiles: BTreeMap<(i16, i16), u16>,  // position -> original tile type
	// original tile is stored here so it can be restored when anti-push is disabled
	antipush_enabled: bool,
	antipush_synced: bool,
}

pub async fn initialize(rotmguard: &Rotmguard) -> Result<State> {
	let s = State {
		hazardous_tiles: BTreeMap::new(),
		conveyor_tiles: BTreeMap::new(),
		antipush_enabled: false,
		antipush_synced: true,
	};

	Ok(s)
}

pub async fn handle_c2s_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let block_packet = match packet_bytes[0] {
		PACKET_ID::C2S_PLAYERTEXT => commands::commands(proxy, &mut packet_bytes).await?,
		_ => false,
	};

	if !block_packet {
		proxy.send_server(packet_bytes.freeze()).await;
	}

	Ok(())
}

pub async fn handle_s2c_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let block_packet = match packet_bytes[0] {
		PACKET_ID::S2C_UPDATE => update_packet::update_packet(proxy, &mut packet_bytes).await?,
		_ => false,
	};

	if !block_packet {
		proxy.send_client(packet_bytes.freeze()).await;
	}

	Ok(())
}

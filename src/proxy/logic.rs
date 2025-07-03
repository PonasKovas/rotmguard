use super::Proxy;
use crate::util::{PACKET_ID, View};
use anyhow::Result;
use bytes::{Buf, BytesMut};
use cheats::{antipush::AntiPush, autonexus::Autonexus};

mod cheats;
mod packets;

#[derive(Default)]
pub struct State {
	antipush: AntiPush,
	autonexus: Autonexus,
}

pub async fn handle_c2s_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let cursor = &mut 0;
	let block_packet = match View(&packet_bytes, cursor).get_u8() {
		PACKET_ID::C2S_PLAYERTEXT => packets::playertext(proxy, &mut packet_bytes, cursor).await?,
		_ => false,
	};

	if !block_packet {
		proxy.send_server(packet_bytes.freeze()).await;
	}

	Ok(())
}

pub async fn handle_s2c_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let cursor = &mut 0;
	let block_packet = match View(&packet_bytes, cursor).get_u8() {
		PACKET_ID::S2C_UPDATE => packets::update(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::S2C_NEWTICK => packets::newtick(proxy, &mut packet_bytes, cursor).await?,
		_ => false,
	};

	if !block_packet {
		proxy.send_client(packet_bytes.freeze()).await;
	}

	Ok(())
}

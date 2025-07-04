use super::Proxy;
use crate::util::{PACKET_ID, View};
use anyhow::Result;
use bytes::{Buf, BytesMut};
use cheats::{antipush::AntiPush, autonexus::Autonexus};
use tracing::warn;

mod cheats;
mod packets;

#[derive(Default)]
pub struct State {
	my_obj_id: u32,
	antipush: AntiPush,
	autonexus: Autonexus,
}

pub async fn handle_c2s_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let mut packet_parsed = true;

	let cursor = &mut 0;

	let packet_id = View(&packet_bytes, cursor).get_u8();
	let block_packet = match packet_id {
		PACKET_ID::C2S_PLAYERTEXT => packets::playertext(proxy, &mut packet_bytes, cursor).await?,
		_ => {
			packet_parsed = false;
			false
		}
	};

	if packet_parsed {
		let leftover = View(&packet_bytes, cursor).slice();
		if leftover.len() > 0 {
			warn!(
				"Leftover unparsed bytes at [{packet_id}] packet:\n{:?}",
				&leftover[..leftover.len().min(500)]
			);
		}
	}

	if !block_packet {
		proxy.send_server(packet_bytes.freeze()).await;
	}

	Ok(())
}

pub async fn handle_s2c_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let mut packet_parsed = true;

	let cursor = &mut 0;

	let packet_id = View(&packet_bytes, cursor).get_u8();
	let block_packet = match packet_id {
		PACKET_ID::S2C_UPDATE => packets::update(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::S2C_NEWTICK => packets::newtick(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::S2C_CREATE_SUCCESS => {
			packets::create_success(proxy, &mut packet_bytes, cursor).await?
		}
		_ => {
			packet_parsed = false;
			false
		}
	};

	if packet_parsed {
		let leftover = View(&packet_bytes, cursor).slice();
		if leftover.len() > 0 {
			warn!(
				"Leftover unparsed bytes at [{packet_id}] packet:\n{:?}",
				&leftover[..leftover.len().min(500)]
			);
		}
	}

	if !block_packet {
		proxy.send_client(packet_bytes.freeze()).await;
	}

	Ok(())
}

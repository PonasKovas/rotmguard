use super::Proxy;
use crate::{
	logging::save_logs,
	util::{PACKET_ID, View},
};
use anyhow::Result;
use bytes::{Buf, BytesMut};
use cheats::{antipush::AntiPush, autonexus::Autonexus, fakeslow::FakeSlow};
use tracing::{info, warn};

mod cheats;
mod packets;

#[derive(Default)]
pub struct State {
	my_obj_id: u32,
	antipush: AntiPush,
	fakeslow: FakeSlow,
	autonexus: Autonexus,
}

pub async fn handle_c2s_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let mut packet_parsed = true;

	let cursor = &mut 0;

	let packet_id = View(&packet_bytes, cursor).get_u8();
	let block_packet = match packet_id {
		PACKET_ID::C2S_PLAYERTEXT => packets::playertext(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::C2S_MOVE => packets::r#move(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::C2S_PLAYERHIT => packets::playerhit(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::C2S_GROUNDDAMAGE => {
			packets::ground_damage(proxy, &mut packet_bytes, cursor).await?
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
		PACKET_ID::S2C_ENEMYSHOOT => packets::enemyshoot(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::S2C_NOTIFICATION => {
			packets::notification(proxy, &mut packet_bytes, cursor).await?
		}
		PACKET_ID::S2C_DAMAGE => packets::damage(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::S2C_DEATH => {
			info!("holy shit 💀"); // 🪦 願您在天使的懷抱中找到永恆的和平與安寧。安息。
			save_logs();
			packet_parsed = false;
			false
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

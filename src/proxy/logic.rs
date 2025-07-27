use std::sync::Arc;

use super::{Proxy, packets};
use crate::{
	Rotmguard,
	util::{PACKET_ID, View},
};
use antipush::AntiPush;
use anyhow::Result;
use autonexus::Autonexus;
use bytes::{Buf, BytesMut};
use common::Common;
use damage_monitor::DamageMonitor;
use fakeslow::FakeSlow;
use notify::Notify;
use tracing::{info, warn};

pub mod antidebuffs;
pub mod antilag;
pub mod antipush;
pub mod autonexus;
pub mod common;
pub mod con;
pub mod damage_monitor;
pub mod fakeslow;
pub mod notify;

pub struct State {
	pub common: Common,
	pub antipush: AntiPush,
	pub fakeslow: FakeSlow,
	pub autonexus: Autonexus,
	pub damage_monitor: DamageMonitor,
	pub notify: Notify,
}

impl State {
	pub fn new(rotmguard: &Arc<Rotmguard>) -> Result<Self> {
		Ok(Self {
			common: Common::default(),
			antipush: AntiPush::new(rotmguard)?,
			fakeslow: Default::default(),
			autonexus: Default::default(),
			damage_monitor: DamageMonitor::new(rotmguard),
			notify: Default::default(),
		})
	}
}

pub async fn handle_c2s_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let mut packet_parsed = true;

	let cursor = &mut 0;

	let packet_id = View(&packet_bytes, cursor).get_u8();
	let block_packet = match packet_id {
		PACKET_ID::C2S_PLAYERTEXT => packets::playertext(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::C2S_MOVE => packets::r#move(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::C2S_PLAYERHIT => packets::playerhit(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::C2S_AOEACK => packets::aoeack(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::C2S_GROUNDDAMAGE => {
			packets::ground_damage(proxy, &mut packet_bytes, cursor).await?
		}
		PACKET_ID::C2S_PLAYERSHOOT => {
			packets::playershoot(proxy, &mut packet_bytes, cursor).await?
		}
		PACKET_ID::C2S_ENEMYHIT => packets::enemyhit(proxy, &mut packet_bytes, cursor).await?,
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
		PACKET_ID::S2C_MAPINFO => packets::mapinfo(proxy, &mut packet_bytes, cursor).await?,
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
		PACKET_ID::S2C_AOE => packets::aoe(proxy, &mut packet_bytes, cursor).await?,
		PACKET_ID::S2C_DEATH => {
			info!("holy shit 💀"); // 🪦 願您在天使的懷抱中找到永恆的和平與安寧。安息。
			packet_parsed = false;
			false
		}
		PACKET_ID::S2C_SERVERPLAYERSHOOT => {
			packets::serverplayershoot(proxy, &mut packet_bytes, cursor).await?
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

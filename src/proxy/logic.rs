use super::Proxy;
use crate::Rotmguard;
use anyhow::Result;
use bytes::{Bytes, BytesMut};
use std::{ops::ControlFlow, sync::OnceLock};
use tracing::error;

// mod antidebuffs;
mod commands;

pub struct State {}

pub async fn initialize(rotmguard: &Rotmguard) -> Result<State> {
	let s = State {};

	Ok(s)
}

pub async fn handle_c2s_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let mut block_packet = false;

	block_packet |= commands::handle_commands(proxy, &mut packet_bytes).await?;

	if !block_packet {
		proxy.send_server(packet_bytes.freeze()).await;
	}

	Ok(())
}

pub async fn handle_s2c_packet(proxy: &mut Proxy, mut packet_bytes: BytesMut) -> Result<()> {
	let mut block_packet = false;

	if !block_packet {
		proxy.send_client(packet_bytes.freeze()).await;
	}

	Ok(())
}

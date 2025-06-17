use super::Proxy;
use anyhow::Result;
use bytes::Bytes;

pub async fn handle_c2s_packet(proxy: &mut Proxy, packet: Bytes) -> Result<()> {
	proxy.send_server(packet).await;

	Ok(())
}

pub async fn handle_s2c_packet(proxy: &mut Proxy, packet: Bytes) -> Result<()> {
	proxy.send_client(packet).await;

	Ok(())
}

use crate::{
	packets::{ClientPacket, ServerPacket},
	proxy::Proxy,
};
pub use autonexus::Autonexus;
pub use commands::Commands;
use enum_dispatch::enum_dispatch;
use std::io::Result;

mod autonexus;
mod commands;

#[enum_dispatch]
pub trait Module {
	async fn client_packet(&mut self, proxy: &mut Proxy, packet: &mut ClientPacket)
		-> Result<bool>;
	async fn server_packet(&mut self, proxy: &mut Proxy, packet: &mut ServerPacket)
		-> Result<bool>;
	async fn disconnect(&mut self, proxy: &mut Proxy, by_server: bool) -> Result<()>;
}

#[enum_dispatch(Module)]
pub enum ModuleType {
	Commands,
	Autonexus,
}

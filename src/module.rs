use crate::{
	config::Config,
	packets::{ClientPacket, ServerPacket},
	proxy::Proxy,
};
use antidebuffs::Antidebuffs;
use autonexus::Autonexus;
use general::General;
use stats::Stats;
use std::{io::Result, sync::Arc};
use tracing::instrument;

mod antidebuffs;
mod autonexus;
mod general;
mod stats;

pub const FORWARD: Result<PacketFlow> = Ok(PacketFlow::Forward);
pub const BLOCK: Result<PacketFlow> = Ok(PacketFlow::Block);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketFlow {
	Forward,
	Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySide {
	Server,
	Client,
}

// Types that implement this trait are basically persistent between connections (maps)
// while their instances are local to a single connection
pub trait Module {
	type Instance;

	fn new() -> Self;
	fn instance(&self) -> Self::Instance;
}

// An instance of a module for a separate connection (or proxy if you will)
pub trait ModuleInstance {
	async fn client_packet<'a>(
		proxy: &mut Proxy,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow>;
	async fn server_packet<'a>(
		proxy: &mut Proxy,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow>;
	async fn disconnect(proxy: &mut Proxy, by: ProxySide) -> Result<()>;
}

macro_rules! gen_root_module {
	( $($name:ident : $path:path),* $(,)? ) => {
		#[derive(Debug, Clone)]
		pub struct RootModule {
			$($name : $path,)*
		}

		#[derive(Debug, Clone)]
		pub struct RootModuleInstance {
			$($name : <$path as Module>::Instance,)*
		}

		impl Module for RootModule {
			type Instance = RootModuleInstance;

			fn new() -> RootModule {
				RootModule {
					$($name: <$path as Module>::new(),)*
				}
			}
			fn instance(&self) -> RootModuleInstance {
				RootModuleInstance {
					$($name: self.$name.instance(),)*
				}
			}
		}

		impl ModuleInstance for RootModuleInstance {
			async fn client_packet<'a>(
				proxy: &mut Proxy<'_>,
				packet: &mut ClientPacket<'a>,
			) -> Result<PacketFlow> {
				$(
					if <$path as Module>::Instance::client_packet(proxy, packet).await? == PacketFlow::Block {
						return BLOCK;
					}
				)*

				Ok(PacketFlow::Forward)
			}
			async fn server_packet<'a>(
				proxy: &mut Proxy<'_>,
				packet: &mut ServerPacket<'a>,
			) -> Result<PacketFlow> {
				$(
					if <$path as Module>::Instance::server_packet(proxy, packet).await? == PacketFlow::Block {
						return BLOCK;
					}
				)*

				Ok(PacketFlow::Forward)
			}
			async fn disconnect(proxy: &mut Proxy<'_>, by: ProxySide) -> Result<()> {
				$(
					<$path as Module>::Instance::disconnect(proxy, by).await?;
				)*

				Ok(())
			}
		}
	};
}

gen_root_module! {
	general: General,
	stats: Stats,
	autonexus: Autonexus,
	antidebuffs: Antidebuffs,
}

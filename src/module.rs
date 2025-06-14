use crate::{
	packets::{ClientPacket, ServerPacket},
	proxy::Proxy,
};
use anti_push::AntiPush;
use antidebuffs::Antidebuffs;
use anyhow::Result;
use autonexus::Autonexus;
use con::Con;
use fake_slow::FakeSlow;
use general::General;
use stats::Stats;

mod anti_push;
mod antidebuffs;
mod autonexus;
mod con;
mod fake_slow;
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
#[allow(unused_variables)]
pub trait ModuleInstance {
	async fn pre_client_packet<'a>(
		proxy: &mut Proxy,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		FORWARD
	}
	async fn client_packet<'a>(
		proxy: &mut Proxy,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		FORWARD
	}
	async fn post_client_packet<'a>(
		proxy: &mut Proxy,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		FORWARD
	}

	async fn pre_server_packet<'a>(
		proxy: &mut Proxy,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		FORWARD
	}
	async fn server_packet<'a>(
		proxy: &mut Proxy,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		FORWARD
	}
	async fn post_server_packet<'a>(
		proxy: &mut Proxy,
		packet: &mut ServerPacket<'a>,
	) -> Result<PacketFlow> {
		FORWARD
	}

	async fn disconnect(proxy: &mut Proxy, by: ProxySide) -> Result<()> {
		Ok(())
	}
}

macro_rules! gen_root_module {
	( $($name:ident : $path:path),* $(,)? ) => {
		#[derive(Debug, Clone)]
		pub struct RootModule {
			$( $name : $path, )*
		}

		#[derive(Debug, Clone)]
		pub struct RootModuleInstance {
			$(
				#[allow(dead_code)]
				$name : <$path as Module>::Instance,
			)*
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
				proxy: &mut Proxy,
				packet: &mut ClientPacket<'a>,
			) -> Result<PacketFlow> {
				$(
					if <$path as Module>::Instance::pre_client_packet(proxy, packet).await? == PacketFlow::Block {
						return BLOCK;
					}
				)*
				$(
					if <$path as Module>::Instance::client_packet(proxy, packet).await? == PacketFlow::Block {
						return BLOCK;
					}
				)*
				$(
					if <$path as Module>::Instance::post_client_packet(proxy, packet).await? == PacketFlow::Block {
						return BLOCK;
					}
				)*

				FORWARD
			}
			async fn server_packet<'a>(
				proxy: &mut Proxy,
				packet: &mut ServerPacket<'a>,
			) -> Result<PacketFlow> {
				$(
					if <$path as Module>::Instance::pre_server_packet(proxy, packet).await? == PacketFlow::Block {
						return BLOCK;
					}
				)*
				$(
					if <$path as Module>::Instance::server_packet(proxy, packet).await? == PacketFlow::Block {
						return BLOCK;
					}
				)*
				$(
					if <$path as Module>::Instance::post_server_packet(proxy, packet).await? == PacketFlow::Block {
						return BLOCK;
					}
				)*

				FORWARD
			}
			async fn disconnect(proxy: &mut Proxy, by: ProxySide) -> Result<()> {
				$(
					<$path as Module>::Instance::disconnect(proxy, by).await?;
				)*

				Ok(())
			}

			// The root modules doesnt implement these, it calls these for others instead
			async fn pre_server_packet<'a>(
				_proxy: &mut Proxy,
				_packet: &mut ServerPacket<'a>,
			) -> Result<PacketFlow> {
				unimplemented!();
			}
			async fn post_server_packet<'a>(
				_proxy: &mut Proxy,
				_packet: &mut ServerPacket<'a>,
			) -> Result<PacketFlow> {
				unimplemented!();
			}
			async fn pre_client_packet<'a>(
				_proxy: &mut Proxy,
				_packet: &mut ClientPacket<'a>,
			) -> Result<PacketFlow> {
				unimplemented!();
			}
			async fn post_client_packet<'a>(
				_proxy: &mut Proxy,
				_packet: &mut ClientPacket<'a>,
			) -> Result<PacketFlow> {
				unimplemented!();
			}
		}
	};
}

gen_root_module! {
	general: General,
	stats: Stats,
	autonexus: Autonexus,
	fake_slow: FakeSlow,
	anti_push: AntiPush,
	con: Con,
	antidebuffs: Antidebuffs,
}

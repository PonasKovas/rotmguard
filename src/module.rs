use crate::{
	packets::{ClientPacket, ServerPacket},
	proxy::Proxy,
};
// use autonexus::Autonexus;
use general::General;
use stats::Stats;
use std::io::Result;

// mod autonexus;
mod general;
mod stats;

// Types that implement this trait are basically persistent between connections (maps)
// while their instances are local to a single connection
pub trait Module {
	type Instance;

	fn new() -> Self;
	fn instance(&self) -> Self::Instance;
}

// An instance of a module for a separate connection (or proxy if you will)
pub trait ModuleInstance {
	/// Return TRUE to forward the packet, FALSE to block
	async fn client_packet(proxy: &mut Proxy, packet: &mut ClientPacket) -> Result<bool>;
	/// Return TRUE to forward the packet, FALSE to block
	async fn server_packet(proxy: &mut Proxy, packet: &mut ServerPacket) -> Result<bool>;
	async fn disconnect(proxy: &mut Proxy, by_server: bool) -> Result<()>;
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
			async fn client_packet(
				proxy: &mut Proxy,
				packet: &mut ClientPacket,
			) -> Result<bool> {
				$(
					if !<$path as Module>::Instance::client_packet(proxy, packet).await? {
						return Ok(false);
					}
				)*

				Ok(true)
			}
			async fn server_packet(
				proxy: &mut Proxy,
				packet: &mut ServerPacket,
			) -> Result<bool> {
				$(
					if !<$path as Module>::Instance::server_packet(proxy, packet).await? {
						return Ok(false);
					}
				)*

				Ok(true)
			}
			async fn disconnect(proxy: &mut Proxy, by_server: bool) -> Result<()> {
				$(
					<$path as Module>::Instance::disconnect(proxy, by_server).await?;
				)*

				Ok(())
			}
		}
	};
}

gen_root_module! {
	general: General,
	stats: Stats,
	// autonexus: Autonexus,
}

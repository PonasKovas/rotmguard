#![feature(result_flattening)]

use anyhow::{bail, Context, Result};
use lru::LruCache;
use module::{Module, ModuleType};
use nix::NixPath;
use proxy::Proxy;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::ErrorKind;
use std::num::NonZeroUsize;
use std::sync::{Arc, OnceLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::Mutex;
use tracing::{error, info};

mod asset_extract;
mod config;
mod constants;
mod extra_datatypes;
mod iptables;
mod logging;
mod module;
mod packets;
mod proxy;
mod read;
mod rotmguard;
mod tests;
mod util;
mod write;

type Modules = Arc<Mutex<Vec<ModuleType>>>;

static CONFIG: OnceLock<config::Config> = OnceLock::new();

fn config() -> &'static config::Config {
	CONFIG.get().unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
	// Initialize config
	let raw_config = fs::read_to_string(config::CONFIG_PATH).context("reading config file")?;
	CONFIG
		.set(toml::from_str(&raw_config).context("parsing config file")?)
		.unwrap();

	// Initialize logger
	logging::init_logger()?;

	info!("Reading assets.");

	// Read the resource assets
	if config().assets_res.is_empty() {
		bail!("assets_res not set. Please edit your rotmguard.toml!");
	}
	let _assets_guard = asset_extract::extract_assets(&config().assets_res)?;

	// create an iptables rule to redirect all game traffic to our proxy
	let _iptables_rule = iptables::IpTablesRule::create()?;

	let modules: Modules = Arc::new(Mutex::new(vec![
		module::Commands {}.into(),
		module::Autonexus {
			hp: 0.0,
			tick_when_last_hit: 0,
			bullets: LruCache::new(NonZeroUsize::new(10000).unwrap()),
			objects: BTreeMap::new(),
			hazardous_tiles: HashMap::new(),
		}
		.into(),
	]));

	select! {
		res = server(modules) => res,
		_ = tokio::signal::ctrl_c() => {
			info!("Exiting...");
			Ok(())
		}
	}
}

async fn server(modules: Modules) -> Result<()> {
	let listener = TcpListener::bind("127.0.0.1:2051").await?;

	loop {
		if let Err(e) = accept_con(&listener, Arc::clone(&modules)).await {
			error!("{e:?}");
		}
	}
}

async fn accept_con(listener: &TcpListener, modules: Modules) -> Result<()> {
	let (socket, _) = listener.accept().await?;

	// linux shenanigans 🤓
	// basically, since the connection was forwarded to ourselves using iptables, we need to obtain
	// the original destination address so we can connect to it
	let original_dst = std::net::IpAddr::from(
		nix::sys::socket::getsockopt(&socket, nix::sys::socket::sockopt::OriginalDst)?
			.sin_addr
			.s_addr
			.to_le_bytes(),
	);

	info!("Connecting to {original_dst}");
	let real_server = TcpStream::connect((original_dst, 2051)).await?; // iptables rule will redirect this to port 2050

	let mut proxy = Proxy::new(socket, real_server, modules);

	tokio::spawn(async move {
		if let Err(e) = proxy.run().await {
			if e.kind() != ErrorKind::UnexpectedEof {
				error!("{e:?}");
			}
		}
	});

	Ok(())
}

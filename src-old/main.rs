use anyhow::{Context, Result};
use assets::Assets;
use config::Config;
use module::{Module, RootModule, RootModuleInstance};
use proxy::Proxy;
use std::fs;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tracing::{error, info};

mod assets;
mod config;
mod extra_datatypes;
mod gen_this_macro;
mod iptables;
mod logging;
mod module;
mod packets;
mod proxy;
mod read;
mod tests;
mod util;
mod write;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
	// Initialize config
	let raw_config = fs::read_to_string(config::CONFIG_PATH).context("reading config file")?;
	let config: Arc<Config> = Arc::new(toml::from_str(&raw_config).context("parsing config file")?);

	// Initialize logger
	logging::init_logger(&config)?;

	info!("Reading assets.");

	// Read the resource assets
	let assets = Arc::new(assets::handle_assets(&config)?);

	// create an iptables rule to redirect all game traffic to our proxy
	let _iptables_rule = iptables::IpTablesRule::create()?;

	let modules = RootModule::new();

	select! {
		res = server(config, assets, modules) => res,
		_ = tokio::signal::ctrl_c() => {
			info!("Exiting...");

			Ok(())
		}
	}
}

async fn server(config: Arc<Config>, assets: Arc<Assets>, modules: RootModule) -> Result<()> {
	let listener = TcpListener::bind("127.0.0.1:2051").await?;

	loop {
		if let Err(e) = accept_con(
			&listener,
			Arc::clone(&config),
			Arc::clone(&assets),
			modules.instance(),
		)
		.await
		{
			error!("{e:?}");
		}
	}
}

async fn accept_con(
	listener: &TcpListener,
	config: Arc<Config>,
	assets: Arc<Assets>,
	modules: RootModuleInstance,
) -> Result<()> {
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

	real_server.set_nodelay(true)?;
	socket.set_nodelay(true)?;

	tokio::spawn(async move {
		if let Err(e) = Proxy::run(config, assets, modules, socket, real_server).await {
			error!("{e:?}");
		}
	});

	Ok(())
}

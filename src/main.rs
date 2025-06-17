use anyhow::{Context, Result};
use assets::Assets;
use config::Config;
use std::fs;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tracing::{error, info};

mod assets;
mod config;
mod iptables;
mod logging;
mod packet_ids;
mod proxy;
mod rc4;

struct Rotmguard {
	config: Config,
	assets: Assets,
}

#[tokio::main]
async fn main() -> Result<()> {
	// Initialize config
	let raw_config = fs::read_to_string(config::CONFIG_PATH).context("reading config file")?;
	let config: Config = toml::from_str(&raw_config).context("parsing config file")?;

	// Initialize logger
	logging::init_logger(&config)?;

	info!("Reading assets.");

	// Read the resource assets
	let assets = assets::handle_assets(&config)?;

	// create an iptables rule to redirect all game traffic to our proxy
	let _iptables_rule = iptables::IpTablesRule::create()?;

	let rotmguard = Arc::new(Rotmguard { config, assets });

	select! {
		res = server(rotmguard) => res,
		_ = tokio::signal::ctrl_c() => {
			info!("Exiting...");

			Ok(())
		}
	}
}

async fn server(rotmguard: Arc<Rotmguard>) -> Result<()> {
	let listener = TcpListener::bind("127.0.0.1:2051").await?;

	loop {
		if let Err(e) = accept_con(Arc::clone(&rotmguard), &listener).await {
			error!("{e:?}");
		}
	}
}

async fn accept_con(rotmguard: Arc<Rotmguard>, listener: &TcpListener) -> Result<()> {
	let (client, _) = listener.accept().await?;

	// linux shenanigans 🤓
	// basically, since the connection was forwarded to ourselves using iptables, we need to obtain
	// the original destination address so we can connect to it
	let original_dst = std::net::IpAddr::from(
		nix::sys::socket::getsockopt(&client, nix::sys::socket::sockopt::OriginalDst)?
			.sin_addr
			.s_addr
			.to_le_bytes(),
	);

	info!("Connecting to {original_dst}");
	let server = TcpStream::connect((original_dst, 2051)).await?; // iptables rule will redirect this to port 2050

	server.set_nodelay(true)?;
	client.set_nodelay(true)?;

	tokio::spawn(async move {
		if let Err(e) = proxy::run(rotmguard, client, server).await {
			error!("{e:?}");
		}
	});

	Ok(())
}

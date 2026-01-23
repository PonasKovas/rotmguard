use anyhow::{Context, Result};
use assets::Assets;
use clap::Parser;
use config::Config;
use damage_monitor_http_server::DamageMonitorHttp;
use stats::Stats;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tracing::{error, info};

mod assets;
mod config;
mod damage_monitor_http_server;
mod fetch_server_list;
mod iptables;
mod logging;
mod packet_logger;
mod proxy;
mod rc4;
mod stats;
mod util;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
	/// logs all packet bytes for inspection
	#[arg(long, default_value_t = false)]
	packet_logger: bool,
	/// [internal]
	#[arg(long, action)]
	iptables: Option<u16>,
}

struct Rotmguard {
	args: Args,
	config: Config,
	assets: Assets,
	rotmg_servers: HashMap<String, String>,
	stats: Stats,
	damage_monitor_http: DamageMonitorHttp,
}

// used in many places..
#[derive(Copy, Clone)]
enum Direction {
	C2S,
	S2C,
}

fn main() -> Result<()> {
	let args = Args::parse();

	if let Some(port) = args.iptables {
		return iptables::iptables_actor(port);
	}

	// Initialize config
	let raw_config = fs::read_to_string(config::CONFIG_PATH).context("reading config file")?;
	let config: Config = toml::from_str(&raw_config).context("parsing config file")?;

	// Initialize logger
	logging::init_logger(&config)?;

	if args.packet_logger {
		info!("Packet logging enabled");
	}

	// create an iptables rule to redirect all game traffic to our proxy
	let _iptables_rule = iptables::IpTablesRules::create()?;

	// Read the resource assets
	let assets = assets::handle_assets(&config).context("reading assets")?;

	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async_main(args, config, assets))
}

async fn async_main(args: Args, config: Config, assets: Assets) -> Result<()> {
	let damage_monitor_http = DamageMonitorHttp::new(&config).await?;

	let rotmguard = Arc::new(Rotmguard {
		args,
		config,
		assets,
		rotmg_servers: fetch_server_list::fetch().await?,
		stats: Default::default(),
		damage_monitor_http,
	});

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

	info!("Ready");

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

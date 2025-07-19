use anyhow::{Context, Result};
use assets::Assets;
use config::Config;
use damage_monitor_http_server::DamageMonitorHttp;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::{env, fs};
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
mod util;

struct Rotmguard {
	config: Config,
	assets: Assets,
	rotmg_servers: HashMap<String, String>,
	flush_skips: FlushSkips,
	damage_monitor_http: DamageMonitorHttp,
}

#[derive(Default)]
struct FlushSkips {
	// total packets forwarded/sent
	total_packets: AtomicU64,
	// total IO flushes on the stream
	flushes: AtomicU64,
	// total summed spaces between flushes
	total_time: AtomicU64,
}

fn main() -> Result<()> {
	if let Some(arg) = env::args().nth(1) {
		if arg == iptables::IPTABLES_ACTOR_FLAG {
			return iptables::iptables_actor();
		}
	}

	// Initialize config
	let raw_config = fs::read_to_string(config::CONFIG_PATH).context("reading config file")?;
	let config: Config = toml::from_str(&raw_config).context("parsing config file")?;

	// Initialize logger
	logging::init_logger(&config)?;

	if packet_logger::enabled() {
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
		.block_on(async_main(config, assets))
}

async fn async_main(config: Config, assets: Assets) -> Result<()> {
	let damage_monitor_http = DamageMonitorHttp::new(&config).await?;

	let rotmguard = Arc::new(Rotmguard {
		config,
		assets,
		rotmg_servers: fetch_server_list::fetch().await?,
		flush_skips: Default::default(),
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

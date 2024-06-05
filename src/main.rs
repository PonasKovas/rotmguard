#![allow(warnings)]

use anyhow::{bail, Context, Result};
use hex::FromHex;
use logging::save_logs;
use nix::NixPath;
use proxy::Proxy;
use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, ErrorKind, Read};
use std::sync::{Arc, OnceLock};
use tokio::io::ErrorKind::WouldBlock;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::sync::Notify;
use tracing::{event, span, Level};

mod asset_extract;
mod config;
mod extra_datatypes;
mod iptables;
mod logging;
mod packets;
mod proxy;
pub mod read;
mod rotmguard;
mod tests;
pub mod write;

pub static CONFIG: OnceLock<config::Config> = OnceLock::new();

pub fn config() -> &'static config::Config {
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

	event!(Level::INFO, "Starting rotmguard.");

	// Read the resource assets
	if config().assets_res.is_empty() {
		bail!("assets_res not set. Please edit your rotmguard.toml!");
	}
	asset_extract::extract_assets(&config().assets_res)?;

	// create an iptables rule to redirect all game traffic to our proxy
	let _iptables_rule = iptables::IpTablesRule::create()?;

	// Set up exit notify structure for shutting down clean
	let exit = Arc::new(Notify::new());
	let exit_clone = Arc::clone(&exit);
	ctrlc::set_handler(move || exit_clone.notify_waiters()).expect("Error setting Ctrl-C handler");

	let exit_clone = Arc::clone(&exit);
	select! {
		res = server(exit_clone) => res,
		_ = exit.notified() => { println!("Exiting..."); Ok(()) }
	}
}

async fn server(exit: Arc<Notify>) -> Result<()> {
	let listener = TcpListener::bind("127.0.0.1:2051").await?;

	loop {
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

		println!("Connecting to {original_dst}");
		let real_server = TcpStream::connect((original_dst, 2051)).await?;

		let mut proxy = Proxy::new(socket, real_server);

		let exit_clone = Arc::clone(&exit);
		tokio::spawn(async move {
			select! {
				Err(e) = proxy.run() => {
					if e.kind() != ErrorKind::UnexpectedEof {
						println!("ERROR: {e:?}");
					}
				},
				_ = exit_clone.notified() => {}
			}
		});
	}
}

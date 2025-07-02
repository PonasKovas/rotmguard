use anyhow::{Context, Result};
use std::collections::HashMap;
use tracing::{error, info};

// i host this cloudflare worker to allow to make the server list request to rotmg servers
// (which requires auth) without exposing the login details of the account used here
const SERVER_LIST_PROXY_SERVICE: &str = "http://rotmguard-serverlist-proxy.ponas900.workers.dev";

pub async fn fetch() -> Result<HashMap<String, String>> {
	info!("Fetching server list for /con command...");
	let servers_xml = reqwest::get(SERVER_LIST_PROXY_SERVICE)
		.await?
		.text()
		.await?;

	let servers_xml = xmltree::Element::parse(servers_xml.as_bytes())?;

	let mut servers = HashMap::new();
	for xml in servers_xml.children {
		let mut srv = match xml {
			xmltree::XMLNode::Element(element) => element,
			_ => continue,
		};

		let mut name = srv
			.take_child("Name")
			.map(|c| c.get_text().map(|t| t.into_owned()))
			.flatten()
			.context("Name parameter of Server")?;
		let address = srv
			.take_child("DNS")
			.map(|c| c.get_text().map(|t| t.into_owned()))
			.flatten()
			.context("DNS parameter of Server")?;

		// the Names are like EUEast, but i want them to be shorter like eue
		// so i just remove all lowercase letters, and have a special case for Australia
		// to make it AUS instead of A which is Asia already
		if name == "Australia" {
			name = "aus".to_owned();
		} else {
			name.retain(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
			name.make_ascii_lowercase();
		}

		info!("[SERVER] {name} -> {address}");
		if let Some(_) = servers.insert(name, address) {
			error!("Server name collision! Very bad!");
		}
	}

	Ok(servers)
}

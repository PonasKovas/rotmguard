use super::{Module, ModuleInstance, PacketFlow, BLOCK, FORWARD};
use crate::{
	gen_this_macro,
	packets::{ClientPacket, Reconnect},
	proxy::Proxy,
	util::Notification,
};
use phf::phf_map;
use std::io::Result;

pub static SERVERS: phf::Map<&str, &str> = phf_map! {
	"eue"	=> "18.184.218.174",
	"eusw"	=> "35.180.67.120",
	"use2"	=> "54.209.152.223",
	"eun"	=> "18.159.133.120",
	"use"	=> "54.234.226.24",
	"usw4"	=> "54.235.235.140",
	"euw2"	=> "52.16.86.215",
	"a"		=> "3.0.147.127",
	"uss3"	=> "52.207.206.31",
	"euw"	=> "15.237.60.223",
	"usw"	=> "54.86.47.176",
	"usmw2"	=> "3.140.254.133",
	"usmw"	=> "18.221.120.59",
	"uss"	=> "3.82.126.16",
	"usw3"	=> "18.144.30.153",
	"ussw"	=> "54.153.13.68",
	"usnw"	=> "34.238.176.119",
	"aus"	=> "54.79.72.84"
};

gen_this_macro! {con}

#[derive(Debug, Clone)]
pub struct Con {}

#[derive(Debug, Clone)]
pub struct ConInst {}

impl Module for Con {
	type Instance = ConInst;

	fn new() -> Self {
		Con {}
	}
	fn instance(&self) -> Self::Instance {
		ConInst {}
	}
}

impl ModuleInstance for ConInst {
	async fn client_packet<'a>(
		proxy: &mut Proxy<'_>,
		packet: &mut ClientPacket<'a>,
	) -> Result<PacketFlow> {
		match packet {
			ClientPacket::PlayerText(text) => {
				let text = &text.text;

				// `/con <server>` quickly and conveniently connects you to the specified server
				// Use a short name for the server: for example if you wanted to connect to EUEast, type eue
				if text.starts_with("/con") {
					let srv = match text.split(" ").nth(1) {
						Some(s) => s,
						None => {
							Notification::new("Specify a server. Example: eue".to_owned())
								.blue()
								.send(&mut proxy.write)
								.await?;

							return BLOCK;
						}
					};

					match SERVERS.get(&srv.to_lowercase()) {
						Some(ip) => {
							let packet = Reconnect {
								hostname: "have fun :)".into(),
								address: (*ip).into(),
								port: 2050,
								game_id: 0xfffffffe,
								key_time: 0xffffffff,
								key: Vec::new(),
							};
							proxy.write.send_client(&packet.into()).await?;
						}
						None => {
							Notification::new(format!("Server {srv:?} is invalid."))
								.red()
								.send(&mut proxy.write)
								.await?;
						}
					}

					return BLOCK;
				}

				FORWARD
			}
			_ => FORWARD,
		}
	}
}

use crate::{
	protocol::packets::{notification::create_notification, reconnect::create_reconnect},
	proxy::Proxy,
};
use anyhow::Result;
use bytes::Bytes;
use std::sync::OnceLock;

pub async fn con<'a>(proxy: &mut Proxy, mut args: impl Iterator<Item = &'a str>) {
	let server = match args.next() {
		Some(s) => s,
		None => {
			proxy.send_client(usage_notification()).await;
			return;
		}
	};

	if args.count() > 0 {
		proxy.send_client(usage_notification()).await;
		return;
	}

	match proxy.rotmguard.rotmg_servers.get(server) {
		Some(ip) => {
			proxy
				.send_client(create_reconnect("", ip, 2050, 0xfffffffe, 0xffffffff, &[]))
				.await;
		}
		None => {
			proxy.send_client(invalid_server_notification()).await;
		}
	}
}

fn usage_notification() -> Bytes {
	static NOTIFICATION: OnceLock<Bytes> = OnceLock::new();

	NOTIFICATION
		.get_or_init(|| {
			create_notification(
				"Usage: /con <short server name>. Example: /con eue",
				0xf5cb42,
			)
		})
		.clone()
}

fn invalid_server_notification() -> Bytes {
	static NOTIFICATION: OnceLock<Bytes> = OnceLock::new();

	NOTIFICATION
		.get_or_init(|| {
			create_notification(
				"Invalid server name. Examples: eusw, use, eun, a, aus",
				0xf5cb42,
			)
		})
		.clone()
}

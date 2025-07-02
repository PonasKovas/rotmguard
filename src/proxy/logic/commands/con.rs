use crate::{
	protocol::util::{create_reconnect, static_notification},
	proxy::Proxy,
};
use bytes::Bytes;

pub async fn con<'a>(proxy: &mut Proxy, mut args: impl Iterator<Item = &'a str>) {
	let server = match args.next() {
		Some(s) => s,
		None => {
			proxy.send_client(usage_notification()).await;
			return;
		}
	};

	// if extra args (we already took the server arg)
	if args.count() > 0 {
		proxy.send_client(usage_notification()).await;
		return;
	}

	match proxy
		.rotmguard
		.rotmg_servers
		.get(&server.to_ascii_lowercase())
	{
		Some(ip) => {
			let packet = create_reconnect("have fun :)", ip, 2050, 0xfffffffe, 0xffffffff, &[]);
			proxy.send_client(packet).await;
		}
		None => {
			proxy.send_client(invalid_server_notification()).await;
		}
	}
}

fn usage_notification() -> Bytes {
	static_notification!(
		"Usage: /con <short server name>. Example: /con eue",
		0xf5cb42
	)
}

fn invalid_server_notification() -> Bytes {
	static_notification!(
		"Invalid server name. Examples: eusw, use, eun, a, aus",
		0xf5cb42,
	)
}

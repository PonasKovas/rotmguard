use crate::{
	proxy::{Proxy, logic::autonexus::devmode},
	util::create_notification,
};
use serde::Deserialize;
use tracing::{error, warn};

pub async fn object_notification(proxy: &mut Proxy, message: &str, object_id: u32, color: u32) {
	if color != 0x00ff00 {
		return; // green means heal
	}
	if object_id != proxy.state.common.objects.self_id {
		return; // only interested in myself
	}

	// of course they add a sprinkle of invalid JSON to the protocol
	#[derive(Deserialize)]
	struct H {
		k: String,
		t: T,
	}
	#[derive(Deserialize)]
	struct T {
		amount: String,
	}

	let amount_healed = match json5::from_str::<H>(message) {
		Ok(h) => {
			if h.k != "s.plus_symbol" {
				warn!("Unexpected object notification for heal. k not equal to 's.plus_symbol'");
				return;
			}
			match h.t.amount.parse::<i64>() {
				Ok(n) => n,
				Err(e) => {
					error!("Error parsing heal notification amount: {e:?}");
					return;
				}
			}
		}
		Err(_) => {
			return;
		}
	};

	proxy.state.autonexus.hp += amount_healed as f32;
	// make sure to not over-heal
	proxy.state.autonexus.hp = proxy
		.state
		.autonexus
		.hp
		.min(proxy.state.common.objects.get_self().stats.max_hp as f32);
	if devmode(proxy) {
		proxy
			.send_client(create_notification(
				&format!("HEAL {amount_healed}"),
				0x33ff33,
			))
			.await;
	}
}

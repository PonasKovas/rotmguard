use super::Module;
use crate::{
	gen_this_macro,
	proxy::Proxy,
};
use serde::Deserialize;
use std::borrow::Cow;
use tracing::{debug, error};

gen_this_macro! {autonexus.heals}

#[derive(Debug, Clone)]
pub struct Heals {}

impl Heals {
	pub fn new() -> Self {
		Heals {}
	}
	pub fn heal(proxy: &mut Proxy<'_>, message: &Cow<str>) {
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

		let amount_healed =
			match json5::from_str::<H>(message) {
				Ok(h) => {
					if h.k != "s.plus_symbol" {
						error!("Unexpected object notification for heal. k not equal to 's.plus_symbol'");
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
				Err(e) => {
					error!("Error parsing object notification: {e:?}");
					return;
				}
			};

		autonexus!(proxy).hp = (autonexus!(proxy).hp + amount_healed as f64)
			.min(proxy.modules.stats.get_newest().stats.max_hp as f64);

		debug!(
			heal_amount = amount_healed,
			new_hp = autonexus!(proxy).hp,
			"Healed"
		);
	}
}

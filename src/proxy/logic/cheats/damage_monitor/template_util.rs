use base64::{Engine, prelude::BASE64_STANDARD};
use std::sync::OnceLock;

pub fn icon() -> &'static str {
	static ICON: OnceLock<String> = OnceLock::new();

	ICON.get_or_init(|| {
		BASE64_STANDARD.encode(include_bytes!(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/assets/icon.png"
		)))
	})
}

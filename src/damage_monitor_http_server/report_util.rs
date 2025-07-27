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

pub fn undefined_sprite() -> &'static str {
	static UNDEFINED_SPRITE: OnceLock<String> = OnceLock::new();

	UNDEFINED_SPRITE.get_or_init(|| {
		BASE64_STANDARD.encode(include_bytes!(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/assets/undefined_sprite.png"
		)))
	})
}

pub fn format_number(n: i64) -> String {
	match n.abs() {
		..1_000 => n.to_string(),
		1_000..100_000 => format!("{:.1}K", n as f64 / 1_000.0),
		100_000..1_000_000 => format!("{:.0}K", n as f64 / 1_000.0),
		1_000_000..100_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
		100_000_000..1_000_000_000 => format!("{:.0}M", n as f64 / 1_000_000.0),
		1_000_000_000..100_000_000_000 => format!("{:.1}G", n as f64 / 1_000_000_000.0),
		_ => format!("{:.0}G", n as f64 / 1_000_000_000.0),
	}
}

//! Notification when connection over

use crate::proxy::Proxy;
use anyhow::{Context, Result};
use std::io::Write;
use tempfile::Builder;
use tracing::error;

#[derive(Default)]
pub struct Notify {
	enabled: bool,
}

pub fn enable(proxy: &mut Proxy) {
	proxy.state.notify.enabled = true;
}

fn show_notification() -> Result<()> {
	let icon = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icon.png"));

	let mut tempfile = Builder::new()
		.suffix(".png")
		.tempfile()
		.context("creating tempfile")?;
	tempfile.write_all(icon).context("writing tempfile")?;
	notify_rust::Notification::new()
		.summary("Rotmguard level change")
		.icon(
			tempfile
				.path()
				.to_str()
				.context("getting tempfile path as string")?,
		)
		.show()
		.context("showing notification")?;

	Ok(())
}

impl Drop for Notify {
	fn drop(&mut self) {
		if !self.enabled {
			return;
		}

		if let Err(e) = show_notification() {
			error!("erorr displaying desktop notification: {e:?}");
		}
	}
}

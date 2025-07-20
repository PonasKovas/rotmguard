use crate::config::Config;
use anyhow::Result;
use tracing_subscriber::{EnvFilter, Layer, Registry, layer::SubscriberExt};

pub fn init_logger(_config: &Config) -> Result<()> {
	let filter =
		EnvFilter::try_from_env("ROTMGUARD_LOG").unwrap_or(EnvFilter::new("rotmguard=INFO"));
	let stdout_layer = tracing_subscriber::fmt::layer()
		.with_writer(std::io::stdout)
		.with_filter(filter);

	let subscriber = Registry::default().with(stdout_layer);
	tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

	Ok(())
}

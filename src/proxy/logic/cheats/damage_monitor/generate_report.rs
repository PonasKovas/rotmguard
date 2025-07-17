use crate::{
	proxy::Proxy,
	util::{GREEN, RED, create_notification},
};
use anyhow::Result;
use askama::Template;
use tracing::error;

pub async fn generate_report(proxy: &mut Proxy) {
	if let Err(e) = inner(proxy).await {
		error!("{e}");
		proxy
			.send_client(create_notification(&format!("{e}"), RED))
			.await;
	}
}

async fn inner(proxy: &mut Proxy) -> Result<()> {
	let page = proxy.state.damage_monitor.render()?;

	let id = proxy.rotmguard.damage_monitor_http.add_page(page);

	let port = proxy.rotmguard.damage_monitor_http.port;
	let url = format!("http://127.0.0.1:{port}/{id}");
	if proxy.rotmguard.config.settings.open_browser {
		webbrowser::open(&url)?;
	}
	proxy
		.send_client(create_notification(
			&format!("Report generated. Open at\n{url}"),
			GREEN,
		))
		.await;

	Ok(())
}

use crate::config::Config;
use anyhow::Result;
use hyper::{Response, server::conn::http1::Builder, service::service_fn};
use hyper_util::rt::TokioIo;
use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tracing::{error, info};

#[derive(Default)]
pub struct DamageMonitorHttp {
	pub port: u16,
	inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
	counter: u32,
	pages: BTreeMap<u32, String>,
}

impl DamageMonitorHttp {
	pub async fn new(config: &Config) -> Result<Self> {
		if !config.settings.damage_monitor.enabled {
			return Ok(Self::default());
		}

		let listener = TcpListener::bind("0.0.0.0:0").await?;
		let addr = listener.local_addr()?;

		info!("Damage monitor http server bound on {addr}");

		let inner = Arc::new(Mutex::new(Inner::default()));

		let inner_clone = Arc::clone(&inner);
		tokio::spawn(async move {
			if let Err(e) = server(listener, inner_clone).await {
				error!("{e}");
			}
		});

		Ok(Self {
			port: addr.port(),
			inner,
		})
	}
	pub fn add_page(&self, page: String) -> u32 {
		let mut inner = self.inner.lock().unwrap();
		let id = inner.counter;
		inner.counter += 1;

		inner.pages.insert(id, page);

		id
	}
}

async fn server(listener: TcpListener, inner: Arc<Mutex<Inner>>) -> Result<()> {
	let service = service_fn(async |request| {
		let path = request.uri().path().strip_prefix('/').unwrap();

		let response;
		match path.parse::<u32>() {
			Err(e) => {
				response = Response::builder()
					.status(400)
					.body(format!("Invalid id {path}: {e}"))
					.unwrap();
			}
			Ok(id) => match inner.lock().unwrap().pages.remove(&id) {
				None => {
					response = Response::builder()
					.status(400)
					.body(format!("Page {id} not found. All pages are consumed after viewed, you can't open the same page twice. Do not reload the page after generating to not lose it."))
					.unwrap();
				}
				Some(page) => {
					response = Response::builder().status(200).body(page).unwrap();
				}
			},
		}

		Ok::<_, &'static str>(response)
	});

	loop {
		let (stream, _) = listener.accept().await?;

		Builder::new()
			.serve_connection(TokioIo::new(stream), service.clone())
			.await?;
	}
}

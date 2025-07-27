use crate::config::Config;
use anyhow::Result;
use askama::Template;
use hyper::{Response, server::conn::http1::Builder, service::service_fn};
use hyper_util::rt::TokioIo;
use lru::LruCache;
use std::{
	collections::BTreeMap,
	num::NonZeroUsize,
	sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tracing::{error, info};

mod report_util;

pub enum DamageMonitorHttp {
	Disabled,
	Enabled { port: u16, inner: Arc<Mutex<Inner>> },
}

pub struct Inner {
	// for getting unique ids
	live_counter: usize,
	memory_counter: usize,
	// id -> (lowercase map name, report)
	live_pages: LruCache<usize, (String, Report)>,
	memory: LruCache<usize, (String, Report)>,
}

#[derive(Template)]
#[template(path = "damage_report.html")]
pub struct Report {
	pub map_name: String,
	pub time: String,
	pub enemy_tabs: Vec<EnemyTab>,
	pub all_items: BTreeMap<u32, Option<String>>, // item id -> item sprite base64
	pub all_enemies: BTreeMap<u32, Option<String>>, // enemy object id -> item sprite base64
}

pub struct EnemyTab {
	pub name: String,
	pub object_id: u32,
	pub total_damage: i64,
	pub players: Vec<PlayerRow>,
}

pub struct PlayerRow {
	pub name: String,
	pub is_self: bool,
	pub status: char,
	pub damage: i64,
	pub damage_percent: String,
	pub items: [Option<PlayerItem>; 4],
}

pub struct PlayerItem {
	pub id: u32,
	pub name: String,
	pub enchantments: Vec<String>,
}

impl DamageMonitorHttp {
	pub async fn new(config: &Config) -> Result<Self> {
		if !config.settings.damage_monitor.enabled {
			return Ok(Self::Disabled);
		}

		let listener = TcpListener::bind("0.0.0.0:0").await?;
		let addr = listener.local_addr()?;

		info!("Damage monitor http server bound on {addr}");

		let inner = Arc::new(Mutex::new(Inner {
			live_counter: 0,
			live_pages: LruCache::new(NonZeroUsize::new(15).unwrap()),
			memory_counter: 0,
			memory: LruCache::new(
				NonZeroUsize::new(config.settings.damage_monitor.keep_memory as usize).unwrap(),
			),
		}));

		let inner_clone = Arc::clone(&inner);
		tokio::spawn(async move {
			if let Err(e) = server(listener, inner_clone).await {
				error!("{e}");
			}
		});

		Ok(Self::Enabled {
			port: addr.port(),
			inner,
		})
	}
	pub fn port(&self) -> u16 {
		match self {
			DamageMonitorHttp::Disabled => panic!("damage monitor disabled"),
			DamageMonitorHttp::Enabled { port, inner: _ } => *port,
		}
	}
	fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
		match self {
			DamageMonitorHttp::Disabled => panic!("damage monitor disabled"),
			DamageMonitorHttp::Enabled { port: _, inner } => inner.lock().unwrap(),
		}
	}
	pub fn add_final_report(&self, page: Report) {
		let mut inner = self.inner();
		let id = inner.memory_counter;
		inner.memory_counter += 1;

		inner.memory.put(id, (page.map_name.to_lowercase(), page));
	}
	pub fn add_live_report(&self, page: Report) -> usize {
		let mut inner = self.inner();
		let id = inner.live_counter;
		inner.live_counter += 1;

		inner
			.live_pages
			.put(id, (page.map_name.to_lowercase(), page));

		id
	}
	pub fn find_memory_by_name(&self, substr: &str) -> Option<usize> {
		let lowercase_pat = substr.to_lowercase();
		// lru iterates in most recently used order
		for (&id, (lowercase_name, _page)) in self.inner().memory.iter() {
			if lowercase_name.contains(&lowercase_pat) {
				return Some(id);
			}
		}

		None
	}
	pub fn find_memory_by_offset(&self, offset: usize) -> Option<usize> {
		let inner = self.inner();
		let index = inner.memory_counter.checked_sub(offset)?;

		if inner.memory.contains(&index) {
			Some(index)
		} else {
			None
		}
	}
}

async fn server(listener: TcpListener, inner: Arc<Mutex<Inner>>) -> Result<()> {
	let service = service_fn(async |request| {
		let path = request.uri().path().strip_prefix('/').unwrap();

		let (page_type, id) = match path.split_once('/') {
			Some(x) => x,
			None => {
				return Ok(Response::builder()
					.status(400)
					.body("Invalid path".to_owned())
					.unwrap());
			}
		};

		let mut inner = inner.lock().unwrap();
		let pages = match page_type {
			"live" => &mut inner.live_pages,
			"memory" => &mut inner.memory,
			other => {
				return Ok(Response::builder()
					.status(400)
					.body(format!("Invalid page type {other:?}"))
					.unwrap());
			}
		};

		let id = match id.parse::<usize>() {
			Ok(x) => x,
			Err(e) => {
				return Ok(Response::builder()
					.status(400)
					.body(format!("Invalid id {id:?}: {e}"))
					.unwrap());
			}
		};

		let response = match pages.get(&id) {
			Some((_name, page)) => {
				let page = page.render().unwrap();
				Response::builder().status(200).body(page).unwrap()
			}
			None => Response::builder()
				.status(400)
				.body(format!(
					"Report {id} not found. Available reports: {:?}",
					pages.iter().map(|(k, _)| k).collect::<Vec<_>>()
				))
				.unwrap(),
		};

		Ok::<_, &'static str>(response)
	});

	loop {
		let (stream, _) = listener.accept().await?;

		Builder::new()
			.serve_connection(TokioIo::new(stream), service.clone())
			.await?;
	}
}

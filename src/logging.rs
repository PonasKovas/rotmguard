use crate::config::Config;
use anyhow::Result;
use std::{
	collections::VecDeque,
	fs::{File, create_dir_all},
	io::Write,
	path::Path,
	sync::{Mutex, MutexGuard, OnceLock},
};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt::MakeWriter, layer::SubscriberExt};

struct LogBuffer {
	max_lines: OnceLock<usize>,
	// (number of lines currently, buffer)
	buffer: Mutex<(usize, VecDeque<u8>)>,
}

static LOG_BUFFER: LogBuffer = LogBuffer {
	max_lines: OnceLock::new(),
	buffer: Mutex::new((0, VecDeque::new())),
};

struct LogWriter<'a> {
	lock: MutexGuard<'a, (usize, VecDeque<u8>)>,
}

impl Write for LogWriter<'_> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		self.lock.0 += buf.iter().filter(|b| **b == b'\n').count();

		Write::write(&mut self.lock.1, buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Write::flush(&mut self.lock.1)
	}
}

impl<'a> MakeWriter<'a> for &'static LogBuffer {
	type Writer = LogWriter<'a>;

	fn make_writer(&'a self) -> Self::Writer {
		let mut buffer = self.buffer.lock().unwrap();

		// remove oldest log lines if we're at limit
		while buffer.0 >= *self.max_lines.get().expect("max log lines not set") {
			// find the position of the first line break and remove everything up to it
			let pos = buffer
				.1
				.iter()
				.position(|b| *b == b'\n')
				.expect("log lines at limit but no linebreak found");

			buffer.1.drain(..=pos);
			buffer.0 -= 1;
		}

		LogWriter { lock: buffer }
	}
}

pub fn init_logger(config: &Config) -> Result<()> {
	LOG_BUFFER
		.max_lines
		.set(config.settings.log_lines)
		.expect("max log lines already set");

	// this is for saving logs in a file
	let logbuffer_layer = tracing_subscriber::fmt::layer()
		.with_writer(&LOG_BUFFER)
		.json()
		.with_filter(LevelFilter::TRACE);

	// and this one for printing to stdout
	let filter =
		EnvFilter::try_from_env("ROTMGUARD_LOG").unwrap_or(EnvFilter::new("rotmguard=INFO"));
	let stdout_layer = tracing_subscriber::fmt::layer()
		.with_writer(std::io::stdout)
		.with_filter(filter);

	let subscriber = Registry::default().with(stdout_layer).with(logbuffer_layer);
	tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

	Ok(())
}

pub fn save_logs() {
	tokio::task::spawn_blocking(|| {
		if !Path::new("logs/").exists() {
			if let Err(e) = create_dir_all("logs/") {
				error!("couldn't create directory logs/. {e:?}");
			}
		}

		let path = format!("logs/{}.log", chrono::Local::now());
		let mut log_file = match File::create(&path) {
			Ok(file) => file,
			Err(e) => {
				error!("couldn't create log file: {e:?}");
				return;
			}
		};

		let buffer_lock = LOG_BUFFER.buffer.lock().unwrap();
		let (s1, s2) = buffer_lock.1.as_slices();
		if let Err(e) = log_file.write_all(s1).and(log_file.write_all(s2)) {
			drop(buffer_lock);
			error!("couldn't write to log file: {e:?}");
			return;
		}
		drop(buffer_lock);

		info!("Logs saved!");
	});
}

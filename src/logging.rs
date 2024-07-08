use crate::config::{self, Config};
use anyhow::Result;
use std::{
	collections::VecDeque,
	fs::{create_dir_all, File},
	io::Write,
	path::Path,
	sync::{Mutex, MutexGuard, OnceLock},
};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Layer, Registry};

static LOG_BUFFER: LogBuffer = LogBuffer {
	max_lines: OnceLock::new(),
	buffer: Mutex::new(VecDeque::new()),
};

struct LogBuffer {
	max_lines: OnceLock<usize>,
	buffer: Mutex<VecDeque<Vec<u8>>>,
}

struct LogWriter<'a> {
	lock: MutexGuard<'a, VecDeque<Vec<u8>>>,
}

impl<'a> Write for LogWriter<'a> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let log_line = self.lock.front_mut().unwrap();

		Write::write(log_line, buf)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		let log_line = self.lock.front_mut().unwrap();

		Write::flush(log_line)
	}
}

impl<'a> MakeWriter<'a> for &'static LogBuffer {
	type Writer = LogWriter<'a>;

	fn make_writer(&'a self) -> Self::Writer {
		let mut buffer = self.buffer.lock().unwrap();

		// remove oldest log lines if we're at limit
		while buffer.len() >= *self.max_lines.get().expect("max log lines not set") {
			buffer.pop_back();
		}

		buffer.push_front(Vec::new());

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
	if !Path::new("logs/").exists() {
		if let Err(e) = create_dir_all("logs/") {
			error!("couldn't create directory logs/. {e:?}");
		}

		// Set the owner and group IDs to match with the parent directory instead of being root.
		let (o_id, g_id) =
			file_owner::owner_group(".").expect("Couldnt get owner of current directory");
		file_owner::set_owner_group("logs/", o_id, g_id)
			.expect("Couldnt set the owner of logs/ directory.");
	}

	let path = format!("logs/{}.log", chrono::Local::now());
	let mut log_file = match File::create(&path) {
		Ok(file) => {
			// Set the owner and group IDs to match with the parent directory instead of being root.
			let (o_id, g_id) =
				file_owner::owner_group("logs/").expect("Couldnt get owner of logs/ directory");
			file_owner::set_owner_group(&path, o_id, g_id)
				.expect("Couldnt set the owner of logs/ directory.");

			file
		}
		Err(e) => {
			error!("couldn't create log file: {e:?}");
			return;
		}
	};

	for log_line in LOG_BUFFER.buffer.lock().unwrap().iter().rev() {
		if let Err(e) = log_file.write_all(log_line) {
			error!("couldn't write to log file: {e:?}");
			return;
		}
	}

	info!("Logs saved!");
}

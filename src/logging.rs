use anyhow::Result;
use std::{
	collections::VecDeque,
	io::Write,
	sync::{Mutex, MutexGuard},
};
use tokio::{
	fs::{create_dir_all, File},
	io::AsyncWriteExt,
};
use tracing_subscriber::{fmt::MakeWriter, FmtSubscriber};

use crate::config;

static LOG_BUFFER: LogBuffer = LogBuffer {
	buffer: Mutex::new(VecDeque::new()),
};

struct LogBuffer {
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
		let max_log_lines = config().settings.lock().unwrap().log_lines;
		while buffer.len() >= max_log_lines {
			buffer.pop_back();
		}

		buffer.push_front(Vec::new());

		LogWriter { lock: buffer }
	}
}

pub fn init_logger() -> Result<()> {
	FmtSubscriber::builder()
		.with_env_filter("rotmguard=trace")
		.json()
		.with_writer(&LOG_BUFFER)
		.init();

	Ok(())
}

pub async fn save_logs() {
	if let Err(e) = create_dir_all("logs/").await {
		eprintln!("ERROR: couldn't create directory logs/. {e:?}");
	}
	let mut log_file = match File::create(format!("logs/{}.log", chrono::Local::now())).await {
		Ok(file) => file,
		Err(e) => {
			eprintln!("ERROR: couldn't create log file: {e:?}");
			return;
		}
	};

	for log_line in LOG_BUFFER.buffer.lock().unwrap().iter().rev() {
		if let Err(e) = log_file.write_all(&log_line).await {
			eprintln!("ERROR: couldn't write to log file: {e:?}");
		}
	}

	println!("Logs saved!");
}

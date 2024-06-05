use std::{
    collections::VecDeque,
    io::Write,
    sync::{Mutex, MutexGuard},
};

use anyhow::Result;
use tracing_subscriber::{fmt::MakeWriter, FmtSubscriber};

/// How many log lines to save up to error when logs are saved
const MAX_LOG_LINES: usize = 1000;

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

        log_line.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let log_line = self.lock.front_mut().unwrap();

        log_line.flush()
    }
}

impl<'a> MakeWriter<'a> for &'static LogBuffer {
    type Writer = LogWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        let mut buffer = self.buffer.lock().unwrap();

        // remove oldest log line if we're at limit
        if buffer.len() == MAX_LOG_LINES {
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

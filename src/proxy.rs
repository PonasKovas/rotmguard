use crate::{Rotmguard, packet_logger};
use anyhow::Result;
use bytes::Bytes;
use futures::{StreamExt as _, stream::FuturesUnordered};
use reader::Reader;
use std::sync::Arc;
use tokio::{
	net::TcpStream,
	select,
	sync::mpsc::{Sender, channel},
	task::JoinHandle,
};
use tracing::instrument;

mod logic;
mod reader;
mod writer;

const WRITE_CHANNEL_SIZE: usize = 128;

// RC4 cipher keys (server to client and client to server)
#[rustfmt::skip]
const RC4_KEY_S2C: &[u8; 13] = &[0xC9, 0x1D, 0x9E, 0xEC, 0x42, 0x01, 0x60, 0x73, 0x0D, 0x82, 0x56, 0x04, 0xE0];
#[rustfmt::skip]
const RC4_KEY_C2S: &[u8; 13] = &[0x5A, 0x4D, 0x20, 0x16, 0xBC, 0x16, 0xDC, 0x64, 0x88, 0x31, 0x94, 0xFF, 0xD9];

struct Proxy {
	rotmguard: Arc<Rotmguard>,
	client: Sender<Bytes>,
	server: Sender<Bytes>,
	writer_tasks: FuturesUnordered<JoinHandle<()>>,
	state: logic::State,
}

impl Proxy {
	/// must be a single valid packet, length not included.
	async fn send_client(&self, bytes: Bytes) {
		// dont care if fails
		let _ = self.client.send(bytes).await;
	}
	/// must be a single valid packet, length not included.
	async fn send_server(&self, bytes: Bytes) {
		// dont care if fails
		let _ = self.server.send(bytes).await;
	}
}

#[instrument(skip_all, fields(ip = ?server.peer_addr()?))]
pub async fn run(rotmguard: Arc<Rotmguard>, client: TcpStream, server: TcpStream) -> Result<()> {
	// spawn the writing tasks
	let (s_send, s_recv) = channel(WRITE_CHANNEL_SIZE);
	let (c_send, c_recv) = channel(WRITE_CHANNEL_SIZE);

	let (s_read, s_write) = server.into_split();
	let (c_read, c_write) = client.into_split();

	let w1 = tokio::spawn(writer::task(
		Arc::clone(&rotmguard),
		s_write,
		s_recv,
		RC4_KEY_C2S,
	));
	let w2 = tokio::spawn(writer::task(
		Arc::clone(&rotmguard),
		c_write,
		c_recv,
		RC4_KEY_S2C,
	));

	// This task will be for reading packets and handling them
	let proxy = Proxy {
		rotmguard,
		client: c_send,
		server: s_send,
		writer_tasks: FuturesUnordered::from_iter([w1, w2]),
		state: Default::default(),
	};

	let s_read = Reader::new(s_read, RC4_KEY_S2C);
	let c_read = Reader::new(c_read, RC4_KEY_C2S);

	let mut packet_logger = packet_logger::PacketLogger::new().await?;

	let r = proxy.run(c_read, s_read, &mut packet_logger).await;

	packet_logger.finish().await?;

	r
}

impl Proxy {
	async fn run(
		mut self,
		mut c_read: Reader,
		mut s_read: Reader,
		packet_logger: &mut packet_logger::PacketLogger,
	) -> Result<()> {
		loop {
			select! {
				res = c_read.read_more() => {
					res?;

					while let Some(packet) = c_read.try_get_packet()? {
						packet_logger.add(packet_logger::Direction::C2S, &packet).await?;

						logic::handle_c2s_packet(&mut self, packet).await?;
					}
				},
				res = s_read.read_more() => {
					res?;

					while let Some(packet) = s_read.try_get_packet()? {
						packet_logger.add(packet_logger::Direction::S2C, &packet).await?;


						logic::handle_s2c_packet(&mut self, packet).await?;
					}
				},
				_ = self.writer_tasks.next() => {
					// either writing task ended
					// this will drop the channel senders and inevitably stop the other writing thread
					return Ok(());
				},
			}
		}
	}
}

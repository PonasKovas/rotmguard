use super::ServerPacket;
use crate::{
	extra_datatypes::{ObjectStatusData, WorldPos},
	read::RPRead,
	write::RPWrite,
};
use std::io::{self, Error, ErrorKind, Read, Write};

#[derive(Debug, Clone)]
pub struct Reconnect {
	pub hostname: String,
	pub address: String,
	pub port: u16,
	pub game_id: u32,
	pub key_time: u32,
	pub key: Vec<u8>,
}

impl RPRead for Reconnect {
	fn rp_read<R: Read>(data: &mut R) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		let hostname = String::rp_read(data)?;
		let address = String::rp_read(data)?;
		let port = u16::rp_read(data)?;
		let game_id = u32::rp_read(data)?;
		let key_time = u32::rp_read(data)?;
		let key_len = u16::rp_read(data)?;
		let mut key = vec![0u8; key_len as usize];
		data.read_exact(&mut key)?;

		Ok(Self {
			hostname,
			address,
			port,
			game_id,
			key_time,
			key,
		})
	}
}

impl RPWrite for Reconnect {
	fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.hostname.rp_write(buf)?;
		written += self.address.rp_write(buf)?;
		written += self.port.rp_write(buf)?;
		written += self.game_id.rp_write(buf)?;
		written += self.key_time.rp_write(buf)?;

		written += (self.key.len() as u16).rp_write(buf)?;
		buf.write_all(&self.key)?;
		written += self.key.len();

		Ok(written)
	}
}

impl From<Reconnect> for ServerPacket {
	fn from(value: Reconnect) -> Self {
		Self::Reconnect(value)
	}
}

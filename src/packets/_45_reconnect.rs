use super::ServerPacket;
use crate::{read::RPRead, write::RPWrite};
use anyhow::Result;
use std::{
	borrow::Cow,
	io::{Read, Write},
};

#[derive(Debug, Clone)]
pub struct Reconnect<'a> {
	pub hostname: Cow<'a, str>,
	pub address: Cow<'a, str>,
	pub port: u16,
	pub game_id: u32,
	pub key_time: u32,
	pub key: Vec<u8>,
}

impl<'a> RPRead<'a> for Reconnect<'a> {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let hostname = Cow::rp_read(data)?;
		let address = Cow::rp_read(data)?;
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

impl<'a> RPWrite for Reconnect<'a> {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
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

impl<'a> From<Reconnect<'a>> for ServerPacket<'a> {
	fn from(value: Reconnect<'a>) -> Self {
		Self::Reconnect(value)
	}
}

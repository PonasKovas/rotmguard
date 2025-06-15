use super::ClientPacket;
use crate::{read::RPRead, write::RPWrite};
use anyhow::Result;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct PlayerText {
	pub text: String,
}

impl RPRead for PlayerText {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			text: String::rp_read(data)?,
		})
	}
}

impl RPWrite for PlayerText {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.text.rp_write(buf);

		written
	}
}

impl From<PlayerText> for ClientPacket {
	fn from(value: PlayerText) -> Self {
		Self::PlayerText(value)
	}
}

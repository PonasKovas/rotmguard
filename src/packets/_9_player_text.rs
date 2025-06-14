use super::ClientPacket;
use crate::{read::RPRead, write::RPWrite};
use anyhow::Result;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct PlayerText<'a> {
	pub text: Cow<'a, str>,
}

impl<'a> RPRead<'a> for PlayerText<'a> {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			text: Cow::rp_read(data)?,
		})
	}
}

impl<'a> RPWrite for PlayerText<'a> {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.text.rp_write(buf);

		written
	}
}

impl<'a> From<PlayerText<'a>> for ClientPacket<'a> {
	fn from(value: PlayerText<'a>) -> Self {
		Self::PlayerText(value)
	}
}

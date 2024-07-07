use std::io::{self, Read, Write};

use super::ClientPacket;
use crate::{read::RPRead, write::RPWrite};

#[derive(Debug, Clone)]
pub struct PlayerText {
	pub text: String,
}

impl RPRead for PlayerText {
	fn rp_read<R: Read>(data: &mut R) -> io::Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			text: String::rp_read(data)?,
		})
	}
}

impl RPWrite for PlayerText {
	fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.text.rp_write(buf)?;

		Ok(written)
	}
}

impl From<PlayerText> for ClientPacket {
	fn from(value: PlayerText) -> Self {
		Self::PlayerText(value)
	}
}

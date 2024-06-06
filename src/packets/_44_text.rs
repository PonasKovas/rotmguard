use super::ServerPacket;
use crate::{read::RPRead, write::RPWrite};
use std::io::{self, Read, Write};

#[derive(Debug, Clone)]
pub struct TextPacket {
	pub name: String,
	pub object_id: u32,
	pub num_stars: u16,
	pub bubble_time: u8,
	pub recipient: String,
	pub text: String,
	pub clean_text: String,
	pub is_supporter: bool,
	pub star_background: u32,
}

impl RPRead for TextPacket {
	fn rp_read<R: Read>(data: &mut R) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			name: String::rp_read(data)?,
			object_id: u32::rp_read(data)?,
			num_stars: u16::rp_read(data)?,
			bubble_time: u8::rp_read(data)?,
			recipient: String::rp_read(data)?,
			text: String::rp_read(data)?,
			clean_text: String::rp_read(data)?,
			is_supporter: bool::rp_read(data)?,
			star_background: u32::rp_read(data)?,
		})
	}
}

impl RPWrite for TextPacket {
	fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.name.rp_write(buf)?;
		written += self.object_id.rp_write(buf)?;
		written += self.num_stars.rp_write(buf)?;
		written += self.bubble_time.rp_write(buf)?;
		written += self.recipient.rp_write(buf)?;
		written += self.text.rp_write(buf)?;
		written += self.clean_text.rp_write(buf)?;
		written += self.is_supporter.rp_write(buf)?;
		written += self.star_background.rp_write(buf)?;

		Ok(written)
	}
}

impl From<TextPacket> for ServerPacket {
	fn from(value: TextPacket) -> Self {
		Self::Text(value)
	}
}

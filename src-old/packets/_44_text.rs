use super::ServerPacket;
use crate::{extra_datatypes::ObjectId, read::RPRead, write::RPWrite};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct TextPacket {
	pub name: String,
	pub object_id: ObjectId,
	pub num_stars: u16,
	pub bubble_time: u8,
	pub recipient: String,
	pub text: String,
	pub clean_text: String,
	pub is_supporter: bool,
	pub star_background: u32,
}

impl RPRead for TextPacket {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			name: String::rp_read(data)?,
			object_id: ObjectId(u32::rp_read(data)?),
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
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.name.rp_write(buf);
		written += self.object_id.0.rp_write(buf);
		written += self.num_stars.rp_write(buf);
		written += self.bubble_time.rp_write(buf);
		written += self.recipient.rp_write(buf);
		written += self.text.rp_write(buf);
		written += self.clean_text.rp_write(buf);
		written += self.is_supporter.rp_write(buf);
		written += self.star_background.rp_write(buf);

		written
	}
}

impl From<TextPacket> for ServerPacket {
	fn from(value: TextPacket) -> Self {
		Self::Text(value)
	}
}

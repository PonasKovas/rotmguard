use super::ServerPacket;
use crate::{extra_datatypes::ObjectId, read::RPRead, write::RPWrite};
use std::{
	borrow::Cow,
	io::{self, Read, Write},
};

#[derive(Debug, Clone)]
pub struct TextPacket<'a> {
	pub name: Cow<'a, str>,
	pub object_id: ObjectId,
	pub num_stars: u16,
	pub bubble_time: u8,
	pub recipient: Cow<'a, str>,
	pub text: Cow<'a, str>,
	pub clean_text: Cow<'a, str>,
	pub is_supporter: bool,
	pub star_background: u32,
}

impl<'a> RPRead<'a> for TextPacket<'a> {
	fn rp_read(data: &mut &'a [u8]) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			name: Cow::rp_read(data)?,
			object_id: ObjectId(u32::rp_read(data)?),
			num_stars: u16::rp_read(data)?,
			bubble_time: u8::rp_read(data)?,
			recipient: Cow::rp_read(data)?,
			text: Cow::rp_read(data)?,
			clean_text: Cow::rp_read(data)?,
			is_supporter: bool::rp_read(data)?,
			star_background: u32::rp_read(data)?,
		})
	}
}

impl<'a> RPWrite for TextPacket<'a> {
	fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.name.rp_write(buf)?;
		written += self.object_id.0.rp_write(buf)?;
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

impl<'a> From<TextPacket<'a>> for ServerPacket<'a> {
	fn from(value: TextPacket<'a>) -> Self {
		Self::Text(value)
	}
}

use super::ServerPacket;
use crate::{extra_datatypes::ObjectId, read::RPRead, write::RPWrite};
use anyhow::Result;
use std::{borrow::Cow, io::Write};

#[derive(Debug)]
pub struct NotificationPacket<'a> {
	pub extra: u8,
	pub notification: NotificationType<'a>,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum NotificationType<'a> {
	StatIncrease {
		text: Cow<'a, str>,
	},
	ServerMessage {
		text: Cow<'a, str>,
	},
	ErrorMessage {
		text: Cow<'a, str>,
	},
	StickyMessage {
		text: Cow<'a, str>,
	},
	ObjectText {
		message: Cow<'a, str>,
		object_id: ObjectId,
		color: u32,
	},
	Behavior {
		message: Cow<'a, str>,
		picture_type: u32,
		color: u32,
	},
	Other {
		id: u8,
		data: Cow<'a, [u8]>,
	},
}

impl<'a> RPRead<'a> for NotificationPacket<'a> {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let notification_type = u8::rp_read(data)?;
		let extra = u8::rp_read(data)?;

		let notification = match notification_type {
			0 => NotificationType::StatIncrease {
				text: Cow::rp_read(data)?,
			},
			1 => NotificationType::ServerMessage {
				text: Cow::rp_read(data)?,
			},
			2 => NotificationType::ErrorMessage {
				text: Cow::rp_read(data)?,
			},
			3 => NotificationType::StickyMessage {
				text: Cow::rp_read(data)?,
			},
			6 => NotificationType::ObjectText {
				message: Cow::rp_read(data)?,
				object_id: ObjectId(u32::rp_read(data)?),
				color: u32::rp_read(data)?,
			},
			12 => NotificationType::Behavior {
				message: Cow::rp_read(data)?,
				picture_type: u32::rp_read(data)?,
				color: u32::rp_read(data)?,
			},
			id => NotificationType::Other {
				id,
				data: Cow::Borrowed(data),
			},
		};

		Ok(NotificationPacket {
			extra,
			notification,
		})
	}
}

impl<'a> RPWrite for NotificationPacket<'a> {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
	where
		Self: Sized,
	{
		let mut bytes_written = 0;

		match &self.notification {
			NotificationType::StatIncrease { text } => {
				bytes_written += 0u8.rp_write(buf)?; // notification type
				bytes_written += self.extra.rp_write(buf)?;
				bytes_written += text.rp_write(buf)?;
			}
			NotificationType::ServerMessage { text } => {
				bytes_written += 1u8.rp_write(buf)?; // notification type
				bytes_written += self.extra.rp_write(buf)?;
				bytes_written += text.rp_write(buf)?;
			}
			NotificationType::ErrorMessage { text } => {
				bytes_written += 2u8.rp_write(buf)?; // notification type
				bytes_written += self.extra.rp_write(buf)?;
				bytes_written += text.rp_write(buf)?;
			}
			NotificationType::StickyMessage { text } => {
				bytes_written += 3u8.rp_write(buf)?; // notification type
				bytes_written += self.extra.rp_write(buf)?;
				bytes_written += text.rp_write(buf)?;
			}
			NotificationType::ObjectText {
				message,
				object_id,
				color,
			} => {
				bytes_written += 6u8.rp_write(buf)?; // notification type
				bytes_written += self.extra.rp_write(buf)?;
				bytes_written += message.rp_write(buf)?;
				bytes_written += object_id.0.rp_write(buf)?;
				bytes_written += color.rp_write(buf)?;
			}
			NotificationType::Behavior {
				message,
				picture_type,
				color,
			} => {
				bytes_written += 12u8.rp_write(buf)?; // notification type
				bytes_written += self.extra.rp_write(buf)?;
				bytes_written += message.rp_write(buf)?;
				bytes_written += picture_type.rp_write(buf)?;
				bytes_written += color.rp_write(buf)?;
			}
			NotificationType::Other { id, data } => {
				bytes_written += id.rp_write(buf)?; // notification type
				bytes_written += self.extra.rp_write(buf)?;
				bytes_written += data.len();
				buf.write_all(data)?;
			}
		}

		Ok(bytes_written)
	}
}

impl<'a> From<NotificationPacket<'a>> for ServerPacket<'a> {
	fn from(value: NotificationPacket<'a>) -> Self {
		Self::Notification(value)
	}
}

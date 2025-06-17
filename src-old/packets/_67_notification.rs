use super::ServerPacket;
use crate::{extra_datatypes::ObjectId, read::RPRead, write::RPWrite};
use anyhow::Result;
use std::borrow::Cow;

#[derive(Debug)]
pub struct NotificationPacket {
	pub extra: u8,
	pub notification: NotificationType,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum NotificationType {
	StatIncrease {
		text: String,
	},
	ServerMessage {
		text: String,
	},
	ErrorMessage {
		text: String,
	},
	StickyMessage {
		text: String,
	},
	ObjectText {
		message: String,
		object_id: ObjectId,
		color: u32,
	},
	Behavior {
		message: String,
		picture_type: u32,
		color: u32,
	},
	Other {
		id: u8,
		data: Vec<u8>,
	},
}

impl RPRead for NotificationPacket {
	fn rp_read(data: &mut &[u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let notification_type = u8::rp_read(data)?;
		let extra = u8::rp_read(data)?;

		let notification = match notification_type {
			0 => NotificationType::StatIncrease {
				text: String::rp_read(data)?,
			},
			1 => NotificationType::ServerMessage {
				text: String::rp_read(data)?,
			},
			2 => NotificationType::ErrorMessage {
				text: String::rp_read(data)?,
			},
			3 => NotificationType::StickyMessage {
				text: String::rp_read(data)?,
			},
			6 => NotificationType::ObjectText {
				message: String::rp_read(data)?,
				object_id: ObjectId(u32::rp_read(data)?),
				color: u32::rp_read(data)?,
			},
			12 => NotificationType::Behavior {
				message: String::rp_read(data)?,
				picture_type: u32::rp_read(data)?,
				color: u32::rp_read(data)?,
			},
			id => NotificationType::Other {
				id,
				data: data.to_owned(),
			},
		};

		Ok(NotificationPacket {
			extra,
			notification,
		})
	}
}

impl RPWrite for NotificationPacket {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut bytes_written = 0;

		match &self.notification {
			NotificationType::StatIncrease { text } => {
				bytes_written += 0u8.rp_write(buf); // notification type
				bytes_written += self.extra.rp_write(buf);
				bytes_written += text.rp_write(buf);
			}
			NotificationType::ServerMessage { text } => {
				bytes_written += 1u8.rp_write(buf); // notification type
				bytes_written += self.extra.rp_write(buf);
				bytes_written += text.rp_write(buf);
			}
			NotificationType::ErrorMessage { text } => {
				bytes_written += 2u8.rp_write(buf); // notification type
				bytes_written += self.extra.rp_write(buf);
				bytes_written += text.rp_write(buf);
			}
			NotificationType::StickyMessage { text } => {
				bytes_written += 3u8.rp_write(buf); // notification type
				bytes_written += self.extra.rp_write(buf);
				bytes_written += text.rp_write(buf);
			}
			NotificationType::ObjectText {
				message,
				object_id,
				color,
			} => {
				bytes_written += 6u8.rp_write(buf); // notification type
				bytes_written += self.extra.rp_write(buf);
				bytes_written += message.rp_write(buf);
				bytes_written += object_id.0.rp_write(buf);
				bytes_written += color.rp_write(buf);
			}
			NotificationType::Behavior {
				message,
				picture_type,
				color,
			} => {
				bytes_written += 12u8.rp_write(buf); // notification type
				bytes_written += self.extra.rp_write(buf);
				bytes_written += message.rp_write(buf);
				bytes_written += picture_type.rp_write(buf);
				bytes_written += color.rp_write(buf);
			}
			NotificationType::Other { id, data } => {
				bytes_written += id.rp_write(buf); // notification type
				bytes_written += self.extra.rp_write(buf);
				bytes_written += data.len();
				buf.extend_from_slice(data);
			}
		}

		bytes_written
	}
}

impl From<NotificationPacket> for ServerPacket {
	fn from(value: NotificationPacket) -> Self {
		Self::Notification(value)
	}
}

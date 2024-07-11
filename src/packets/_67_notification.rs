use super::ServerPacket;
use crate::{extra_datatypes::ObjectId, read::RPRead, write::RPWrite};
use std::{
	borrow::Cow,
	io::{Error, Read, Write},
};

#[non_exhaustive]
#[derive(Debug)]
pub enum NotificationPacket<'a> {
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
	TeleportationError {
		text: Cow<'a, str>,
	},
	Global {
		text: Cow<'a, str>,
		ui_extra: u16,
	},
	Queue {
		message_type: u32,
		queue_pos: u16,
	},
	ObjectText {
		message: Cow<'a, str>,
		object_id: ObjectId,
		color: u32,
	},
	PlayerDeath {
		message: Cow<'a, str>,
		picture_type: u32,
	},
	PortalOpened {
		message: Cow<'a, str>,
		picture_type: u32,
	},
	PlayerCallout {
		message: Cow<'a, str>,
		object_id: ObjectId,
		stars: u16,
	},
	ProgressBar {
		message: Option<Cow<'a, str>>,
		max: u32,
		value: u16,
	},
	Behavior {
		message: Cow<'a, str>,
		picture_type: u32,
		color: u32,
	},
	Emote {
		object_id: u32,
		emote_type: u32,
	},
}

impl<'a> RPRead<'a> for NotificationPacket<'a> {
	fn rp_read(data: &mut &'a [u8]) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		let notification_type = u8::rp_read(data)?;
		let extra = u8::rp_read(data)?;

		Ok(match notification_type {
			0 => NotificationPacket::StatIncrease {
				text: Cow::rp_read(data)?,
			},
			1 => NotificationPacket::ServerMessage {
				text: Cow::rp_read(data)?,
			},
			2 => NotificationPacket::ErrorMessage {
				text: Cow::rp_read(data)?,
			},
			3 => NotificationPacket::StickyMessage {
				text: Cow::rp_read(data)?,
			},
			9 => NotificationPacket::TeleportationError {
				text: Cow::rp_read(data)?,
			},
			4 => NotificationPacket::Global {
				text: Cow::rp_read(data)?,
				ui_extra: u16::rp_read(data)?,
			},
			5 => NotificationPacket::Queue {
				message_type: u32::rp_read(data)?,
				queue_pos: u16::rp_read(data)?,
			},
			6 => NotificationPacket::ObjectText {
				message: Cow::rp_read(data)?,
				object_id: ObjectId(u32::rp_read(data)?),
				color: u32::rp_read(data)?,
			},
			7 => NotificationPacket::PlayerDeath {
				message: Cow::rp_read(data)?,
				picture_type: u32::rp_read(data)?,
			},
			8 => NotificationPacket::PortalOpened {
				message: Cow::rp_read(data)?,
				picture_type: u32::rp_read(data)?,
			},
			10 => NotificationPacket::PlayerCallout {
				message: Cow::rp_read(data)?,
				object_id: ObjectId(u32::rp_read(data)?),
				stars: u16::rp_read(data)?,
			},
			11 => {
				let message = if (extra & 3) != 0 {
					Some(Cow::rp_read(data)?)
				} else {
					None
				};
				NotificationPacket::ProgressBar {
					message,
					max: u32::rp_read(data)?,
					value: u16::rp_read(data)?,
				}
			}
			12 => NotificationPacket::Behavior {
				message: Cow::rp_read(data)?,
				picture_type: u32::rp_read(data)?,
				color: u32::rp_read(data)?,
			},
			13 => NotificationPacket::Emote {
				object_id: u32::rp_read(data)?,
				emote_type: u32::rp_read(data)?,
			},
			u => {
				return Err(Error::new(
					std::io::ErrorKind::InvalidData,
					format!("Unknown notification type {u}"),
				));
			}
		})
	}
}

impl<'a> RPWrite for NotificationPacket<'a> {
	fn rp_write<W: Write>(&self, buf: &mut W) -> std::io::Result<usize>
	where
		Self: Sized,
	{
		let mut bytes_written = 0;

		match self {
			NotificationPacket::StatIncrease { text } => {
				bytes_written += 0u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += text.rp_write(buf)?;
			}
			NotificationPacket::ServerMessage { text } => {
				bytes_written += 1u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += text.rp_write(buf)?;
			}
			NotificationPacket::ErrorMessage { text } => {
				bytes_written += 2u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += text.rp_write(buf)?;
			}
			NotificationPacket::StickyMessage { text } => {
				bytes_written += 3u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += text.rp_write(buf)?;
			}
			NotificationPacket::TeleportationError { text } => {
				bytes_written += 9u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += text.rp_write(buf)?;
			}
			NotificationPacket::Global { text, ui_extra } => {
				bytes_written += 4u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += text.rp_write(buf)?;
				bytes_written += ui_extra.rp_write(buf)?;
			}
			NotificationPacket::Queue {
				message_type,
				queue_pos,
			} => {
				bytes_written += 5u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += message_type.rp_write(buf)?;
				bytes_written += queue_pos.rp_write(buf)?;
			}
			NotificationPacket::ObjectText {
				message,
				object_id,
				color,
			} => {
				bytes_written += 6u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += message.rp_write(buf)?;
				bytes_written += object_id.0.rp_write(buf)?;
				bytes_written += color.rp_write(buf)?;
			}
			NotificationPacket::PlayerDeath {
				message,
				picture_type,
			} => {
				bytes_written += 7u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += message.rp_write(buf)?;
				bytes_written += picture_type.rp_write(buf)?;
			}
			NotificationPacket::PortalOpened {
				message,
				picture_type,
			} => {
				bytes_written += 8u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += message.rp_write(buf)?;
				bytes_written += picture_type.rp_write(buf)?;
			}
			NotificationPacket::PlayerCallout {
				message,
				object_id,
				stars,
			} => {
				bytes_written += 10u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += message.rp_write(buf)?;
				bytes_written += object_id.0.rp_write(buf)?;
				bytes_written += stars.rp_write(buf)?;
			}
			NotificationPacket::ProgressBar {
				message,
				max,
				value,
			} => {
				bytes_written += 11u8.rp_write(buf)?; // notification type
				match message {
					Some(message) => {
						bytes_written += 3u8.rp_write(buf)?; // extra
						bytes_written += message.rp_write(buf)?;
					}
					None => {
						bytes_written += 0u8.rp_write(buf)?; // extra
					}
				}

				bytes_written += max.rp_write(buf)?;
				bytes_written += value.rp_write(buf)?;
			}
			NotificationPacket::Behavior {
				message,
				picture_type,
				color,
			} => {
				bytes_written += 12u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += message.rp_write(buf)?;
				bytes_written += picture_type.rp_write(buf)?;
				bytes_written += color.rp_write(buf)?;
			}
			NotificationPacket::Emote {
				object_id,
				emote_type,
			} => {
				bytes_written += 13u8.rp_write(buf)?; // notification type
				bytes_written += 0u8.rp_write(buf)?; // extra
				bytes_written += object_id.rp_write(buf)?;
				bytes_written += emote_type.rp_write(buf)?;
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

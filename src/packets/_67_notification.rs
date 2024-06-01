use crate::write::RPWrite;

use super::ServerPacket;

#[non_exhaustive]
pub enum Notification {
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
    TeleportationError {
        text: String,
    },
    Global {
        text: String,
        ui_extra: u16,
    },
    Queue {
        message_type: u32,
        queue_pos: u16,
    },
    ObjectText {
        message: String,
        object_id: u32,
        color: u32,
    },
    PlayerDeath {
        message: String,
        picture_type: u32,
    },
    PortalOpened {
        message: String,
        picture_type: u32,
    },
    PlayerCallout {
        message: String,
        object_id: u32,
        stars: u16,
    },
    ProgressBar {
        message: Option<String>,
        max: u32,
        value: u16,
    },
    Behavior {
        message: String,
        picture_type: u32,
        color: u32,
    },
    Emote {
        object_id: u32,
        emote_type: u32,
    },
}

impl RPWrite for Notification {
    fn rp_write<W: std::io::prelude::Write>(&self, buf: &mut W) -> std::io::Result<usize>
    where
        Self: Sized,
    {
        let mut bytes_written = 0;

        match self {
            Notification::StatIncrease { text } => {
                bytes_written += 0u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += text.rp_write(buf)?;
            }
            Notification::ServerMessage { text } => {
                bytes_written += 1u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += text.rp_write(buf)?;
            }
            Notification::ErrorMessage { text } => {
                bytes_written += 2u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += text.rp_write(buf)?;
            }
            Notification::StickyMessage { text } => {
                bytes_written += 3u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += text.rp_write(buf)?;
            }
            Notification::TeleportationError { text } => {
                bytes_written += 9u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += text.rp_write(buf)?;
            }
            Notification::Global { text, ui_extra } => {
                bytes_written += 4u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += text.rp_write(buf)?;
                bytes_written += ui_extra.rp_write(buf)?;
            }
            Notification::Queue {
                message_type,
                queue_pos,
            } => {
                bytes_written += 5u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += message_type.rp_write(buf)?;
                bytes_written += queue_pos.rp_write(buf)?;
            }
            Notification::ObjectText {
                message,
                object_id,
                color,
            } => {
                bytes_written += 6u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += message.rp_write(buf)?;
                bytes_written += object_id.rp_write(buf)?;
                bytes_written += color.rp_write(buf)?;
            }
            Notification::PlayerDeath {
                message,
                picture_type,
            } => {
                bytes_written += 7u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += message.rp_write(buf)?;
                bytes_written += picture_type.rp_write(buf)?;
            }
            Notification::PortalOpened {
                message,
                picture_type,
            } => {
                bytes_written += 8u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += message.rp_write(buf)?;
                bytes_written += picture_type.rp_write(buf)?;
            }
            Notification::PlayerCallout {
                message,
                object_id,
                stars,
            } => {
                bytes_written += 10u8.rp_write(buf)?; // notification type
                bytes_written += 0u8.rp_write(buf)?; // extra
                bytes_written += message.rp_write(buf)?;
                bytes_written += object_id.rp_write(buf)?;
                bytes_written += stars.rp_write(buf)?;
            }
            Notification::ProgressBar {
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
            Notification::Behavior {
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
            Notification::Emote {
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

impl From<Notification> for ServerPacket {
    fn from(value: Notification) -> Self {
        Self::Notification(value)
    }
}

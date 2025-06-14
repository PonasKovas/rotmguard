use crate::{
	packets::{NotificationPacket, NotificationType},
	proxy::Proxy,
};
use std::borrow::Cow;

const RED_COLOR: u32 = 0xFF6666;
const GREEN_COLOR: u32 = 0x66FF66;
const BLUE_COLOR: u32 = 0x6666FF;

/// Convenience struct for sending cute little notifications to the client
#[derive(Debug, Clone)]
pub struct Notification {
	text: String,
	color: u32,
}

impl Notification {
	/// Creates a default gray notification
	pub fn new(text: String) -> Self {
		Self {
			text,
			color: 0x888888,
		}
	}
	/// Sets the color
	pub fn color(mut self, color: u32) -> Self {
		self.color = color;
		self
	}
	/// Sets the default red color
	pub fn red(self) -> Self {
		self.color(RED_COLOR)
	}
	/// Sets the default green color
	pub fn green(self) -> Self {
		self.color(GREEN_COLOR)
	}
	/// Sets the default blue color
	pub fn blue(self) -> Self {
		self.color(BLUE_COLOR)
	}
	/// Sends the notification
	pub fn send(self, io: &mut Proxy) {
		let packet = NotificationPacket {
			extra: 0,
			notification: NotificationType::Behavior {
				message: Cow::Owned(self.text),
				picture_type: 0,
				color: self.color,
			},
		};
		io.write_client.add_server_packet(&packet.into());
	}
}

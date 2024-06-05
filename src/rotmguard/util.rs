use crate::packets::NotificationPacket;
use crate::proxy::Proxy;
use std::io;

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
	/// Sends the notification
	pub async fn send(self, proxy: &mut Proxy) -> io::Result<()> {
		let packet = NotificationPacket::Behavior {
			message: self.text,
			picture_type: 0,
			color: self.color,
		};
		proxy.send_client(&packet.into()).await?;

		Ok(())
	}
}

use crate::protocol::{
	RPReadError, RotmgStr, packet_ids::PACKET_ID, read_str, read_u8, read_u32, write_str,
};
use bytes::{BufMut, Bytes, BytesMut};

use super::with_context;

// not trying to handle all possible variants of this packet
// just some basic stuff
pub struct Notification {
	pub notification_type: u8,
	pub text: RotmgStr,
	pub object_id: Option<u32>,
	pub color: Option<u32>,
}

impl Notification {
	pub const ID: u8 = PACKET_ID::S2C_NOTIFICATION;

	with_context!{"Notification packet";
		pub fn parse(bytes: &mut Bytes) -> Result<Notification, RPReadError> {
			let notification_type = read_u8(bytes, "notification type")?;
			read_u8(bytes, "extra")?;
			let text = read_str(bytes, "text")?;

			let mut object_id = None;
			let mut color = None;
			match notification_type {
				6 => {
					object_id = Some(read_u32(bytes, "object id")?);
					color = Some(read_u32(bytes, "color")?);
				}
				_ => {}
			}

			Ok(Notification {
				notification_type,
				text,
				object_id,
				color,
			})
		}
	}
}

// an even more basic function for creating a simple notification originating from rotmguard
pub fn create_notification(text: &str, color: u32) -> Bytes {
	let mut buf = BytesMut::with_capacity(text.len() + 13);

	buf.put_u8(PACKET_ID::S2C_NOTIFICATION);	//	1 // packet id
	buf.put_u8(12);								//	1 // notification type
	buf.put_u8(0);								//	1 // who knows 💀
	write_str(text, &mut buf); 		 			//	2+len
	buf.put_u32(0);								//	4
	buf.put_u32(color); 						//	4
	///////////////////////////////////////////////////////////////////////
	// 									TOTAL:	//	13+len
	///////////////////////////////////////////////////////////////////////

	// 👻

	buf.freeze()
}

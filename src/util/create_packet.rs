use bytes::{BufMut, Bytes, BytesMut};
use super::{size_as_compressed_int, write_compressed_int, write_str, PACKET_ID::{self, C2S_ESCAPE}};

pub const RED: u32 = 0xff8888;
pub const GREEN: u32 = 0x88ff88;
pub const BLUE: u32 = 0x8888ff;

macro_rules! static_notification {
    ($text:expr, $color:expr $(,)?) => {{
        static NOTIFICATION: std::sync::OnceLock<bytes::Bytes> = std::sync::OnceLock::new();
		NOTIFICATION.get_or_init(|| $crate::util::create_notification($text, $color)).clone()
    }};
}
pub(crate) use static_notification;

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

pub fn create_reconnect(hostname: &str, address: &str, port: u16, game_id: u32, key_time: u32, key: &[u8]) -> Bytes {
	let mut buf = BytesMut::with_capacity(hostname.len() + address.len() + key.len() + 17);

	buf.put_u8(PACKET_ID::S2C_RECONNECT);		//	1 // packet id
	write_str(hostname, &mut buf); 		 		//	2+hostname len
	write_str(address, &mut buf); 		 		//	2+address len
	buf.put_u16(port);							//	2 // port
	buf.put_u32(game_id);						//	4 // game id
	buf.put_u32(key_time);						//	4 // key time
	buf.put_u16(key.len() as u16);				//	2 // key len
	buf.extend_from_slice(key);					//  key len
	///////////////////////////////////////////////////////////////////////
	// 									TOTAL:	//	17+hostname len+address len+key len
	///////////////////////////////////////////////////////////////////////

	buf.freeze()
}

pub fn create_escape() -> Bytes {
	Bytes::from_static(&[C2S_ESCAPE])
}

pub fn create_effect(effect_id: u8, object_id: Option<u32>, pos1: (f32, f32), pos2: (f32, f32), color: Option<u32>, duration: Option<f32>) -> Bytes {
	let expected_size = 19 + object_id.map(|id| size_as_compressed_int(id as i64)).unwrap_or(0) + 4 * color.is_some() as usize + 4 * duration.is_some() as usize;
	let mut buf = BytesMut::with_capacity(expected_size);

	buf.put_u8(PACKET_ID::S2C_SHOWEFFECT);		//	1 // packet id
	buf.put_u8(effect_id);						//	1 // effect id
	let bitmask = 0b00011110
			| color.is_some() as u8
			| (duration.is_some() as u8) << 5
			| (object_id.is_some() as u8) << 6;
	buf.put_u8(bitmask);						//	1 // bitmask
	if let Some(id) = object_id {
		write_compressed_int(id as i64, &mut buf); 	// effect id
	}
	buf.put_f32(pos1.0);						//	4
	buf.put_f32(pos1.1);						//	4
	buf.put_f32(pos2.0);						//	4
	buf.put_f32(pos2.1);						//	4
	if let Some(color) = color {
		buf.put_u32(color); 					// 4 // color
	}
	if let Some(duration) = duration {
		buf.put_f32(duration); 					// 4 // duration
	}
	
	///////////////////////////////////////////////////////////////////////
	// 									TOTAL:	//	19+effect_id + color + duration
	///////////////////////////////////////////////////////////////////////

	buf.freeze()
}
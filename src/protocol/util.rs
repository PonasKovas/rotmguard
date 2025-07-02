use bytes::{BufMut, Bytes, BytesMut};
use super::{write_str, PACKET_ID};

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

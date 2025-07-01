use super::with_context;
use crate::protocol::{packet_ids::PACKET_ID, write_str, RPReadError};
use bytes::{BufMut, Bytes, BytesMut};

pub struct Reconnect; // reading not implemented because not needed

impl Reconnect {
	pub const ID: u8 = PACKET_ID::S2C_RECONNECT;

	with_context! { "Reconnect packet";
		pub fn parse(_bytes: &mut Bytes) -> Result<Reconnect, RPReadError> {
			Ok(Reconnect)
		}
	}
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

	// 👻

	buf.freeze()
}

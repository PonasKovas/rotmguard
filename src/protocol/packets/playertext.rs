use super::with_context;
use crate::protocol::{PACKET_ID, RPReadError, RotmgStr, read_str};
use bytes::Bytes;

pub struct PlayerText {
	pub text: RotmgStr,
}

impl PlayerText {
	pub const ID: u8 = PACKET_ID::C2S_PLAYERTEXT;

	with_context! { "PlayerText packet";
		pub fn parse(bytes: &mut Bytes) -> Result<PlayerText, RPReadError> {
			Ok(PlayerText {
				text: read_str(bytes, "text")?,
			})
		}
	}
}

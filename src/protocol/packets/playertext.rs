use crate::protocol::{RPReadError, RotmgStr, packet_ids::PACKET_ID, read_str};
use bytes::Bytes;

pub struct PlayerText {
	pub text: RotmgStr,
}

impl PlayerText {
	pub const ID: u8 = PACKET_ID::C2S_PLAYERTEXT;

	pub fn parse(bytes: &mut Bytes) -> Result<Self, RPReadError> {
		fn parse_inner(bytes: &mut Bytes) -> Result<PlayerText, RPReadError> {
			Ok(PlayerText {
				text: read_str(bytes, "text")?,
			})
		}

		parse_inner(bytes).map_err(|e| RPReadError::WithContext {
			ctx: "PlayerText packet".to_owned(),
			inner: Box::new(e),
		})
	}
}

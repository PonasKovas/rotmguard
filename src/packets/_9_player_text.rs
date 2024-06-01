use super::ClientPacket;
use crate::read::RPRead;

#[derive(Debug, Clone)]
pub struct PlayerText {
    pub text: String,
}

impl RPRead for PlayerText {
    fn rp_read<R: std::io::prelude::Read>(data: &mut R) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            text: String::rp_read(data)?,
        })
    }
}

impl From<PlayerText> for ClientPacket {
    fn from(value: PlayerText) -> Self {
        Self::PlayerText(value)
    }
}

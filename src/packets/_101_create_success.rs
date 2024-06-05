use super::ServerPacket;
use crate::{
	extra_datatypes::{ObjectStatusData, WorldPos},
	read::RPRead,
	write::RPWrite,
};
use std::io::{self, Error, ErrorKind};

#[derive(Debug, Clone)]
pub struct CreateSuccess {
	pub object_id: u32,
	pub char_id: u32,
	pub unknown: String,
}

impl RPRead for CreateSuccess {
	fn rp_read<R: std::io::prelude::Read>(data: &mut R) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			object_id: u32::rp_read(data)?,
			char_id: u32::rp_read(data)?,
			unknown: String::rp_read(data)?,
		})
	}
}

impl From<CreateSuccess> for ServerPacket {
	fn from(value: CreateSuccess) -> Self {
		Self::CreateSuccess(value)
	}
}

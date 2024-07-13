use super::ServerPacket;
use crate::{
	extra_datatypes::{ObjectId, WorldPos},
	read::{read_compressed_int, RPRead},
	write::{write_compressed_int, RPWrite},
};
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct ShowEffect {
	pub effect_type: u8,
	pub target_object_id: Option<ObjectId>,
	pub pos1: WorldPos,
	pub pos2: WorldPos,
	pub color: Option<u32>,
	pub duration: Option<f32>,
	pub unknown: Option<u8>,
}

impl<'a> RPRead<'a> for ShowEffect {
	fn rp_read(data: &mut &'a [u8]) -> std::io::Result<Self>
	where
		Self: Sized,
	{
		let effect_type = u8::rp_read(data)?;
		let bitmask = u8::rp_read(data)?;

		let target_object_id = if (bitmask & 0b01000000) != 0 {
			Some(ObjectId(read_compressed_int(data)? as u32))
		} else {
			None
		};
		let pos1_x = if (bitmask & 0b00000010) != 0 {
			f32::rp_read(data)?
		} else {
			0.0
		};
		let pos1_y = if (bitmask & 0b00000100) != 0 {
			f32::rp_read(data)?
		} else {
			0.0
		};
		let pos2_x = if (bitmask & 0b00001000) != 0 {
			f32::rp_read(data)?
		} else {
			0.0
		};
		let pos2_y = if (bitmask & 0b00010000) != 0 {
			f32::rp_read(data)?
		} else {
			0.0
		};
		let color = if (bitmask & 0b00000001) != 0 {
			Some(u32::rp_read(data)?)
		} else {
			None
		};
		let duration = if (bitmask & 0b00100000) != 0 {
			Some(f32::rp_read(data)?)
		} else {
			None
		};
		let unknown = if (bitmask & 0b10000000) != 0 {
			Some(u8::rp_read(data)?)
		} else {
			None
		};

		Ok(Self {
			effect_type,
			target_object_id,
			pos1: WorldPos {
				x: pos1_x,
				y: pos1_y,
			},
			pos2: WorldPos {
				x: pos2_x,
				y: pos2_y,
			},
			color,
			duration,
			unknown,
		})
	}
}

impl RPWrite for ShowEffect {
	fn rp_write<W: Write>(&self, buf: &mut W) -> io::Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		written += self.effect_type.rp_write(buf)?;

		let bitmask = 0b00011110
			| self.color.is_some() as u8
			| (self.duration.is_some() as u8) << 5
			| (self.target_object_id.is_some() as u8) << 6
			| (self.unknown.is_some() as u8) << 7;

		written += bitmask.rp_write(buf)?;

		if let Some(id) = self.target_object_id {
			written += write_compressed_int(&(id.0 as i64), buf)?;
		}
		written += self.pos1.rp_write(buf)?;
		written += self.pos2.rp_write(buf)?;
		if let Some(color) = self.color {
			written += color.rp_write(buf)?;
		}
		if let Some(duration) = self.duration {
			written += duration.rp_write(buf)?;
		}
		if let Some(unknown) = self.unknown {
			written += unknown.rp_write(buf)?;
		}

		Ok(written)
	}
}

impl<'a> From<ShowEffect> for ServerPacket<'a> {
	fn from(value: ShowEffect) -> Self {
		Self::ShowEffect(value)
	}
}

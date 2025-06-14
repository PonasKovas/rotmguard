use crate::{read::RPRead, write::RPWrite};
use anyhow::Result;
use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct WorldPos {
	pub x: f32,
	pub y: f32,
}

impl<'a> RPRead<'a> for WorldPos {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		Ok(Self {
			x: f32::rp_read(data)?,
			y: f32::rp_read(data)?,
		})
	}
}

impl RPWrite for WorldPos {
	fn rp_write(&self, buf: &mut Vec<u8>) -> usize {
		let mut written = 0;

		written += self.x.rp_write(buf);
		written += self.y.rp_write(buf);

		written
	}
}

impl Add for WorldPos {
	type Output = WorldPos;

	fn add(self, rhs: Self) -> Self::Output {
		WorldPos {
			x: self.x + rhs.x,
			y: self.y + rhs.y,
		}
	}
}

impl Sub for WorldPos {
	type Output = WorldPos;

	fn sub(self, rhs: Self) -> Self::Output {
		WorldPos {
			x: self.x - rhs.x,
			y: self.y - rhs.y,
		}
	}
}

impl Mul<f32> for WorldPos {
	type Output = WorldPos;

	fn mul(self, rhs: f32) -> Self::Output {
		WorldPos {
			x: self.x * rhs,
			y: self.y * rhs,
		}
	}
}

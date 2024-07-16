use crate::{
	read::{read_compressed_int, RPRead},
	write::{write_compressed_int, RPWrite},
};
use anyhow::Result;
use std::{
	borrow::Cow,
	io::{self, Write},
};

#[derive(Debug, Clone)]
pub struct StatData<'a> {
	pub stat_type: StatType,
	pub stat: Stat<'a>,
	pub secondary_stat: i64,
}

#[derive(Debug, Clone)]
pub enum Stat<'a> {
	String(Cow<'a, str>),
	Int(i64),
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
#[non_exhaustive]
pub enum StatType {
	MaxHP = 0,
	HP = 1,
	MaxMP = 3,
	MP = 4,
	Defense = 21,
	Speed = 22,
	Vitality = 26,
	Condition = 29,
	Name = 31,
	CurrentFame = 57,
	ClassQuestFame = 58,
	GuildName = 62,
	Condition2 = 96,
	Other(u8),
}

impl<'a> Stat<'a> {
	pub fn as_int(&self) -> i64 {
		match self {
			Stat::String(s) => s.parse::<i64>().expect("StatType not valid int"), // TODO error handling here
			Stat::Int(i) => *i,
		}
	}
	pub fn as_str<'b>(&'b self) -> &'a str
	where
		'b: 'a,
	{
		match self {
			Stat::String(s) => s.as_ref(),
			Stat::Int(_) => panic!("stat is integer, not string."),
		}
	}
}

impl<'a> RPRead<'a> for StatData<'a> {
	fn rp_read(data: &mut &'a [u8]) -> Result<Self>
	where
		Self: Sized,
	{
		let stat_type = u8::rp_read(data)?;

		let stat =
			if [6, 31, 38, 54, 62, 71, 72, 80, 82, 115, 121, 127, 128, 147].contains(&stat_type) {
				// these are string type stats
				Stat::String(Cow::rp_read(data)?)
			} else {
				// these are normal (int) type stats
				Stat::Int(read_compressed_int(data)?)
			};

		let stat_type = match stat_type {
			0 => StatType::MaxHP,
			1 => StatType::HP,
			3 => StatType::MaxMP,
			4 => StatType::MP,
			21 => StatType::Defense,
			22 => StatType::Speed,
			26 => StatType::Vitality,
			29 => StatType::Condition,
			31 => StatType::Name,
			57 => StatType::CurrentFame,
			58 => StatType::ClassQuestFame,
			62 => StatType::GuildName,
			96 => StatType::Condition2,
			i => StatType::Other(i),
		};

		Ok(Self {
			stat_type,
			stat,
			secondary_stat: read_compressed_int(data)?,
		})
	}
}

impl<'a> RPWrite for StatData<'a> {
	fn rp_write<W: Write>(&self, buf: &mut W) -> Result<usize>
	where
		Self: Sized,
	{
		let mut written = 0;

		let stat_type = match &self.stat_type {
			StatType::Other(i) => *i,
			s => unsafe { *(s as *const _ as *const u8) },
		};

		written += stat_type.rp_write(buf)?;

		match &self.stat {
			Stat::String(s) => {
				written += s.rp_write(buf)?;
			}
			Stat::Int(i) => {
				written += write_compressed_int(i, buf)?;
			}
		}

		written += write_compressed_int(&self.secondary_stat, buf)?;

		Ok(written)
	}
}

use crate::protocol::{RPRead, RPReadError, RPWrite};

pub struct PlayerText;

impl<'a> RPRead<'a> for PlayerText {
	type Out = &'a str;

	fn rp_read(data: &mut &'a [u8]) -> Result<Self::Out, RPReadError> {
		String::rp_read(data)
	}
}
impl RPWrite for PlayerText {
	type Data = str;

	fn rp_write(data: &Self::Data, out: impl bytes::BufMut) {
		String::rp_write(data, out);
	}
}

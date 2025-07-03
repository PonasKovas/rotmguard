use bytes::{Buf, BytesMut};

// Basically Cursor but solves some borrowing and exlusivity issues
// because we need mutable access to the cursor pointer but not to the data itself
// we actually REQUIRE non-exclusive access to the data because we are reading many things at a time
pub struct View<'d, 'c>(
	pub &'d BytesMut,  // byte buffer (actual data) - non-exclusive
	pub &'c mut usize, // cursor - exclusive mutable so can be advanced
);
impl<'d, 'c> View<'d, 'c> {
	pub fn slice(&self) -> &'d [u8] {
		&self.0[*self.1..]
	}
}
impl<'d, 'c> Buf for View<'d, 'c> {
	fn remaining(&self) -> usize {
		self.0.len() - *self.1
	}
	fn chunk(&self) -> &[u8] {
		self.slice()
	}
	fn advance(&mut self, cnt: usize) {
		*self.1 += cnt;
	}
}

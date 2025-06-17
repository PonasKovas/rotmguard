pub struct Rc4 {
	state_vector: [u8; 256],
	i: u8,
	j: u8,
}

impl Rc4 {
	fn s_i(&self) -> u8 {
		self.state_vector[self.i as usize]
	}
	fn s_j(&self) -> u8 {
		self.state_vector[self.j as usize]
	}
	fn ksa(&mut self, key: &[u8]) {
		let mut j = 0u8;
		for (i, &k) in (0..256).zip(key.iter().cycle()) {
			// j = j + S[i] + K[i]
			j = j.wrapping_add(self.state_vector[i]).wrapping_add(k);

			// swap(S[i], S[j])
			self.state_vector.swap(i, j as usize);
		}
	}
}

impl Rc4 {
	/// Constructs a new Rc4 state
	///
	/// `key` must be between `1-256` bytes, will panic otherwise
	pub fn new(key: &[u8]) -> Self {
		assert!(!key.is_empty(), "key cannot be empty");
		assert!(key.len() <= 256, "key cannot be longer than 256 bytes");

		// initialize initial state vector as 0, 1, 2, ..., 255
		let mut iter = 0..=255;
		let state_vector: [u8; 256] = [0; 256].map(|_| iter.next().unwrap());

		let mut rc4 = Self {
			state_vector,
			i: 0,
			j: 0,
		};

		rc4.ksa(key);
		rc4
	}
	#[inline]
	pub fn next_byte(&mut self) -> u8 {
		// i = i + 1
		self.i = self.i.wrapping_add(1);
		// j = j + S[i]
		self.j = self.j.wrapping_add(self.s_i());

		// swap(S[i], S[j])
		self.state_vector.swap(self.i as usize, self.j as usize);

		// t = S[i] + S[j]
		let t = self.s_i().wrapping_add(self.s_j());

		// S[t]
		self.state_vector[t as usize]
	}
	pub fn apply(&mut self, data: &mut [u8]) {
		for byte in data {
			*byte ^= self.next_byte();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rc4() {
		// known sequences, taken from wikipedia
		let known: &[(&str, &[u8])] = &[
			(
				"Key",
				&[0xEB, 0x9F, 0x77, 0x81, 0xB7, 0x34, 0xCA, 0x72, 0xA7, 0x19],
			),
			("Wiki", &[0x60, 0x44, 0xDB, 0x6D, 0x41, 0xB7]),
			("Secret", &[0x04, 0xD4, 0x6B, 0x05, 0x3C, 0xA8, 0x7B, 0x59]),
		];

		for known in known {
			let mut rc4 = Rc4::new(known.0.as_bytes());

			if known.1.iter().any(|&expected| rc4.next_byte() != expected) {
				panic!("{known:?}");
			}
		}
	}
}

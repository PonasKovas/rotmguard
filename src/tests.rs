#[cfg(test)]
mod tests {
	use crate::read::read_compressed_int;
	use crate::write::write_compressed_int;

	#[test]
	fn varint_read_write_consistency() {
		let cases = [
			0i64,
			1,
			63,
			64,
			127,
			128,
			255,
			16383,
			2097151,
			268435455,
			34359738367,
			-1,
			-63,
			-64,
			-127,
			-128,
			-255,
			-16383,
			-2097151,
			-268435455,
			-34359738367,
			2147483647,
			-2147483648,
		];

		let mut buf = Vec::new();
		for case in cases {
			buf.clear();
			let n = write_compressed_int(&case, &mut buf).unwrap();
			assert_eq!(n, buf.len(), "Written bytes n incorrect!");
			assert_eq!(
				case,
				read_compressed_int(&mut &buf[..]).unwrap(),
				"Read/write inconsistency!"
			);
		}
	}
}

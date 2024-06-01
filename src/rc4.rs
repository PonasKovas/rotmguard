pub struct Rc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4 {
    pub fn new(key: &[u8]) -> Self {
        assert!(key.len() <= 256, "RC4 key cannot be longer than 256 bytes");

        let mut s = [0; 256];
        let mut t = [0; 256];
        for i in 0..256 {
            s[i] = i as u8;
            t[i] = key[i % key.len()];
        }

        let mut j: u8 = 0;
        for i in 0..256 {
            j = j.wrapping_add(s[i]).wrapping_add(t[i]);
            s.swap(i as usize, j as usize);
        }

        Self { s, i: 0, j: 0 }
    }
    pub fn next_key(&mut self) -> u8 {
        self.i = self.i.wrapping_add(1);
        self.j = self.j.wrapping_add(self.s[self.i as usize]);
        self.s.swap(self.i as usize, self.j as usize);

        self.s[self.s[self.i as usize].wrapping_add(self.s[self.j as usize]) as usize]
    }
}

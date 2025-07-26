//! Classic Park-Miller LCG

pub struct Rng {
    seed: u32,
}

impl Rng {
    const MULTIPLIER: u64 = 16807;
    const MODULUS: u64 = 2_u64.pow(31) - 1;

    pub fn new(seed: u32) -> Self {
        Self { seed }
    }
    pub fn next(&mut self) -> u32 {
        let product = self.seed as u64 * Self::MULTIPLIER;

        let next_seed = (product % Self::MODULUS) as u32;
        self.seed = next_seed;
        
        self.seed
    }
}

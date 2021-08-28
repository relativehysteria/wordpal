/// A deterministic PRNG (xorshift)
pub struct Rng(u64);

impl Rng {
    /// Creates a new RNG
    pub fn new() -> Self {
        Self(0x1337133713371337)
    }

    /// Returns a pseudo-random (predetermined) number
    pub fn rand(&mut self) -> u64 {
        let ret = self.0;
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 43;
        ret
    }

    /// Returns a pseudo-random (predetermined) number within a given range
    pub fn range(&mut self, min: u64, max: u64) -> u64 {
        (self.rand() % (max - min + 1)) + min
    }
}

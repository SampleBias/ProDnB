use rand_chacha::ChaCha20Rng;
use rand::{SeedableRng, Rng};

#[derive(Debug, Clone)]
pub struct DeterministicRng {
    inner: ChaCha20Rng,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        DeterministicRng {
            inner: ChaCha20Rng::seed_from_u64(seed),
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.inner.gen()
    }

    pub fn next_f32(&mut self) -> f32 {
        self.inner.gen()
    }

    pub fn next_f32_range(&mut self, min: f32, max: f32) -> f32 {
        self.inner.gen_range(min..=max)
    }

    pub fn next_bool(&mut self) -> bool {
        self.inner.gen()
    }

    pub fn next_usize(&mut self) -> usize {
        self.inner.gen()
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        use rand::seq::SliceRandom;
        slice.shuffle(&mut self.inner);
    }
}

use std::{iter::Cycle, slice::Iter};

use rand_core::{
    impls::{next_u32_via_fill, next_u64_via_fill},
    RngCore,
};

static FAKE_RAND_BYTES: &'static [u8] = include_bytes!("./random_bytes.bin");

#[derive(Clone)]
pub struct FakeRand(Cycle<Iter<'static, u8>>);

impl FakeRand {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn skip(&mut self, bytes: usize) {
        for _ in 0..bytes {
            self.0.next().unwrap();
        }
    }
}

impl Default for FakeRand {
    fn default() -> Self {
        FakeRand(FAKE_RAND_BYTES.iter().cycle())
    }
}

impl RngCore for FakeRand {
    fn next_u32(&mut self) -> u32 {
        next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for byte in dest {
            *byte = *self.0.next().unwrap();
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        Ok(self.fill_bytes(dest))
    }
}

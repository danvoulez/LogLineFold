use std::time::{SystemTime, UNIX_EPOCH};

/// Lightweight pseudo-random number generator sufficient for deterministic tests.
#[derive(Clone, Debug)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn seed_from_u64(seed: u64) -> Self {
        let initial = if seed == 0 {
            0xdead_beef_dead_beef
        } else {
            seed
        };
        Self { state: initial }
    }

    pub fn from_entropy() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self::seed_from_u64(nanos as u64 ^ 0xa76f_1234_5678_9abc)
    }

    pub fn gen_range(&mut self, range: std::ops::Range<f64>) -> f64 {
        let span = range.end - range.start;
        range.start + span * self.next_f64()
    }

    fn next_u64(&mut self) -> u64 {
        const MULTIPLIER: u64 = 6364136223846793005;
        const INCREMENT: u64 = 1442695040888963407;
        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        const SCALE: f64 = (1u64 << 53) as f64;
        (self.next_u64() >> 11) as f64 / SCALE
    }
}

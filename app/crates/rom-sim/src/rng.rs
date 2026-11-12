//! A single-state SplitMix64 generator.

/// Reproducible random numbers with an explicitly hashable state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimRng {
    state: u64,
}

impl SimRng {
    /// Starts a SplitMix64 stream at the supplied seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the complete generator state.
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Advances the stream and returns 32 random bits.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((z ^ (z >> 31)) >> 32) as u32
    }

    /// Samples uniformly from [lo, hi); panics if lo is not below hi.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo < hi, "random range must be nonempty");
        let span = hi - lo;
        let threshold = span.wrapping_neg() % span;
        loop {
            let value = self.next_u32();
            if value >= threshold {
                return lo + value % span;
            }
        }
    }
}

/// A stateless rank for two geometric coordinates in `tick`, used for target ties.
pub(crate) fn tie_rank(tick: u64, a: u32, b: u32) -> u64 {
    let mut z = tick
        .wrapping_mul(0x9e3779b97f4a7c15)
        .wrapping_add((u64::from(a) << 32) | u64::from(b));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tie_rank_varies_with_tick() {
        let lower_first = (0..1000)
            .filter(|tick| tie_rank(*tick, 7, 3) < tie_rank(*tick, 7, 9))
            .count();
        assert!((400..=600).contains(&lower_first), "{lower_first}");
        assert_eq!(tie_rank(5, 7, 3), tie_rank(5, 7, 3));
    }

    #[test]
    fn stream_is_pinned_and_ranges_are_bounded() {
        let mut rng = SimRng::new(0);
        assert_eq!(rng.next_u32(), 0xe220a839);
        assert_eq!(rng.next_u32(), 0x6e789e6a);
        for _ in 0..1000 {
            assert!((17..24).contains(&rng.range(17, 24)));
        }
    }
}

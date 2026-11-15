//! Deterministic RNG helpers modeled after `Assembly-CSharp/Randoms.cs`.

/// Deterministic PRNG based on SplitMix64 + xorshift64* for reproducible simulation.
#[derive(Debug, Clone)]
pub struct Randoms {
    state: u64,
    has_next_gaussian: bool,
    next_gaussian: f64,
}

impl Default for Randoms {
    fn default() -> Self {
        Self::from_seed(0x5EED_C0DE_511E_CAFE)
    }
}

impl Randoms {
    pub fn from_seed(seed: u64) -> Self {
        let s = if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed };
        Self {
            state: s,
            has_next_gaussian: false,
            next_gaussian: 0.0,
        }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Returns a uniform `f64` in `[0.0, 1.0)`.
    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        // Use top 53 bits for standard IEEE-754 mantissa precision.
        let bits = self.next_u64() >> 11;
        (bits as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    /// Returns an integer in `[0, high)`. Panics if `high <= 0`.
    pub fn get_int(&mut self, high: i32) -> i32 {
        assert!(high > 0, "Randoms::get_int requires high > 0, got {high}");
        (self.next_f64() * (high as f64)) as i32
    }

    /// Returns an integer in `[low, high)`. If `low == high`, returns `low`.
    pub fn get_in_range_i32(&mut self, low: i32, high: i32) -> i32 {
        if low == high {
            return low;
        }
        low + self.get_int(high - low)
    }

    pub fn get_float(&mut self, high: f32) -> f32 {
        (self.next_f64() * (high as f64)) as f32
    }

    pub fn get_in_range_f32(&mut self, low: f32, high: f32) -> f32 {
        if (low - high).abs() <= f32::EPSILON {
            return low;
        }
        (low as f64 + self.next_f64() * ((high - low) as f64)) as f32
    }

    /// Returns `true` with probability `1 / n`.
    pub fn get_chance(&mut self, n: i32) -> bool {
        self.get_int(n) == 0
    }

    /// Returns `true` with probability `p` in `[0.0, 1.0]`.
    pub fn get_probability(&mut self, p: f32) -> bool {
        self.next_f64() < (p as f64)
    }

    pub fn get_boolean(&mut self) -> bool {
        self.get_int(2) == 0
    }

    /// Box-Muller Gaussian matching `Randoms.GetNormal(mean, dev)`.
    pub fn get_normal(&mut self, mean: f32, dev: f32) -> f32 {
        let z = if self.has_next_gaussian {
            self.has_next_gaussian = false;
            self.next_gaussian
        } else {
            let mut u1 = self.next_f64();
            if u1 < 1e-12 {
                u1 = 1e-12;
            }
            let u2 = self.next_f64();
            let mag = (-2.0 * u1.ln()).sqrt();
            let angle = std::f64::consts::TAU * u2;
            self.next_gaussian = mag * angle.sin();
            self.has_next_gaussian = true;
            mag * angle.cos()
        };
        (z as f32) * dev + mean
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            let idx = self.get_int(items.len() as i32) as usize;
            Some(&items[idx])
        }
    }

    /// Weighted reservoir pick matching `Randoms.Pick(IDictionary<T, float>)`.
    pub fn pick_weighted<'a, T>(&mut self, weighted_items: &[(&'a T, f32)]) -> Option<&'a T> {
        let mut chosen: Option<&'a T> = None;
        let mut total_weight = 0.0f32;
        for &(item, weight) in weighted_items {
            if weight > 0.0 {
                total_weight += weight;
                if total_weight == weight || self.get_float(total_weight) < weight {
                    chosen = Some(item);
                }
            }
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_and_picks_work() {
        let mut rng = Randoms::from_seed(42);
        for _ in 0..100 {
            let v = rng.get_in_range_i32(5, 10);
            assert!((5..10).contains(&v));
            let f = rng.get_in_range_f32(-1.0, 2.5);
            assert!((-1.0..=2.5).contains(&f));
        }
        let items = ["a", "b", "c"];
        assert!(rng.pick(&items).is_some());
        let weighted = [(&items[0], 0.0), (&items[1], 10.0), (&items[2], 0.0)];
        assert_eq!(rng.pick_weighted(&weighted), Some(&"b"));
    }

    #[test]
    fn normal_distribution_is_finite() {
        let mut rng = Randoms::from_seed(123);
        for _ in 0..50 {
            let g = rng.get_normal(0.0, 1.0);
            assert!(g.is_finite());
        }
    }
}

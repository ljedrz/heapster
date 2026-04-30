#[cfg(feature = "serde")]
use std::fmt;
use std::ops::Sub;

#[cfg(feature = "serde")]
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{SeqAccess, Visitor},
};

/// A histogram with power-of-two buckets containing the numbers of
/// entries of different sizes, starting with 2^0 and ending with 2^63.
#[derive(Debug, Clone, Copy)]
pub struct Histogram {
    pub(crate) buckets: [usize; 64],
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            buckets: [0usize; 64],
        }
    }
}

impl Sub<&Histogram> for &Histogram {
    type Output = Histogram;

    fn sub(self, old: &Histogram) -> Histogram {
        let mut out = [0usize; 64];
        for (i, (b_base, b_old)) in self.buckets.iter().zip(&old.buckets).enumerate() {
            out[i] = b_base.saturating_sub(*b_old);
        }
        Histogram { buckets: out }
    }
}

impl Sub<Histogram> for Histogram {
    type Output = Histogram;

    fn sub(self, old: Histogram) -> Histogram {
        &self - &old
    }
}

impl Sub<&Histogram> for Histogram {
    type Output = Histogram;

    fn sub(self, old: &Histogram) -> Histogram {
        &self - old
    }
}

impl Sub<Histogram> for &Histogram {
    type Output = Histogram;

    fn sub(self, old: Histogram) -> Histogram {
        self - &old
    }
}

impl Histogram {
    /// Returns the raw underlying buckets.
    pub fn buckets(&self) -> [usize; 64] {
        self.buckets
    }

    /// Approximates the value at quantile `q` (0.0..=1.0) from a power-of-two histogram.
    ///
    /// Returns `None` if the histogram is empty or `q` is out of range.
    ///
    /// The returned value is **approximate**: it interpolates within the bucket
    /// containing the target rank, assuming uniform distribution across the
    /// bucket's range. Because buckets span [2^k, 2^(k+1)), the absolute error
    /// is bounded by the bucket width — small for small allocations, larger
    /// for large ones. For p50/p90/p99-style outlier hunting this is fine;
    /// for exact percentiles, use a real profiler.
    pub fn quantile(&self, q: f64) -> Option<usize> {
        if !(0.0..=1.0).contains(&q) {
            return None;
        }
        let total: usize = self.buckets.iter().sum();
        if total == 0 {
            return None;
        }

        // Target rank in [1, total]. Using ceil avoids returning bucket k-1 when
        // q lands exactly on a bucket boundary.
        let target = ((total as f64) * q).ceil().max(1.0) as usize;
        let target = target.min(total);

        let mut cumulative = 0usize;
        for (k, &count) in self.buckets.iter().enumerate() {
            if count == 0 {
                continue;
            }
            let next = cumulative + count;
            if next >= target {
                // The target rank falls in bucket k, which covers [2^k, 2^(k+1)).
                let lo = 1usize << k;
                let width = lo; // 2^(k+1) - 2^k == 2^k
                // Position within the bucket: 0-indexed offset of the target.
                let offset_in_bucket = target - cumulative - 1;
                // Linear interpolation: assume the `count` values are spread
                // evenly across the bucket's range.
                let interp = (offset_in_bucket as u128 * width as u128) / count as u128;
                return Some(lo + interp as usize);
            }
            cumulative = next;
        }
        // Unreachable given total > 0, but be safe.
        None
    }
}

#[cfg(feature = "serde")]
impl Serialize for Histogram {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.buckets.as_slice().serialize(s)
    }
}

#[cfg(feature = "serde")]
struct BucketVisitor;

#[cfg(feature = "serde")]
impl<'de> Visitor<'de> for BucketVisitor {
    type Value = Histogram;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of 64 usize values")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut out = [0usize; 64];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = seq
                .next_element()?
                .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
        }
        Ok(Histogram { buckets: out })
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Histogram {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_tuple(64, BucketVisitor)
    }
}

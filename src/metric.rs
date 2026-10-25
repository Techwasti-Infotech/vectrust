use serde::{Deserialize, Serialize};
use crate::error::{Result, VectrustError};

/// Supported distance and similarity metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Metric {
    /// Euclidean distance (L2 norm). Lower is closer.
    #[default]
    Euclidean,
    /// Cosine distance (1.0 - cosine similarity). Lower is closer (range [0, 2]).
    Cosine,
    /// Dot product (or Inner Product). Suitable for normalized vectors or MIPS.
    /// Internal distance is computed as `-dot_product` so that higher similarity yields lower distance.
    DotProduct,
    /// Manhattan distance (L1 norm). Lower is closer.
    Manhattan,
}

impl Metric {
    /// Computes the distance between two vectors according to the metric.
    /// In all metrics, a smaller distance value indicates closer vectors.
    #[inline]
    pub fn distance(&self, a: &[f32], b: &[f32]) -> Result<f32> {
        if a.len() != b.len() {
            return Err(VectrustError::DimensionMismatch {
                expected: a.len(),
                got: b.len(),
            });
        }

        let dist = match self {
            Metric::Euclidean => euclidean_distance(a, b),
            Metric::Cosine => cosine_distance(a, b),
            Metric::DotProduct => dot_product_distance(a, b),
            Metric::Manhattan => manhattan_distance(a, b),
        };

        if dist.is_nan() {
            return Err(VectrustError::InvalidVectorData);
        }

        Ok(dist)
    }

    /// Computes a user-friendly similarity score (higher is more similar).
    #[inline]
    pub fn similarity(&self, a: &[f32], b: &[f32]) -> Result<f32> {
        let dist = self.distance(a, b)?;
        Ok(match self {
            Metric::Euclidean => 1.0 / (1.0 + dist),
            Metric::Cosine => 1.0 - dist,
            Metric::DotProduct => -dist,
            Metric::Manhattan => 1.0 / (1.0 + dist),
        })
    }
}

/// Computes squared Euclidean distance without calculating square root.
/// Ideal for fast comparisons during graph traversal.
#[inline(always)]
pub fn squared_euclidean(a: &[f32], b: &[f32]) -> f32 {
    let chunks_a = a.chunks_exact(8);
    let chunks_b = b.chunks_exact(8);
    let rem_a = chunks_a.remainder();
    let rem_b = chunks_b.remainder();

    let mut sum0 = 0.0f32;
    let mut sum1 = 0.0f32;
    let mut sum2 = 0.0f32;
    let mut sum3 = 0.0f32;
    let mut sum4 = 0.0f32;
    let mut sum5 = 0.0f32;
    let mut sum6 = 0.0f32;
    let mut sum7 = 0.0f32;

    for (ca, cb) in chunks_a.zip(chunks_b) {
        let d0 = ca[0] - cb[0];
        let d1 = ca[1] - cb[1];
        let d2 = ca[2] - cb[2];
        let d3 = ca[3] - cb[3];
        let d4 = ca[4] - cb[4];
        let d5 = ca[5] - cb[5];
        let d6 = ca[6] - cb[6];
        let d7 = ca[7] - cb[7];

        sum0 += d0 * d0;
        sum1 += d1 * d1;
        sum2 += d2 * d2;
        sum3 += d3 * d3;
        sum4 += d4 * d4;
        sum5 += d5 * d5;
        sum6 += d6 * d6;
        sum7 += d7 * d7;
    }

    let mut total = (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7);

    for (&x, &y) in rem_a.iter().zip(rem_b.iter()) {
        let diff = x - y;
        total += diff * diff;
    }

    total
}

/// Computes Euclidean distance.
#[inline]
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    squared_euclidean(a, b).sqrt()
}

/// Computes Dot Product using 8-way unrolled accumulator for auto-vectorization.
#[inline(always)]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let chunks_a = a.chunks_exact(8);
    let chunks_b = b.chunks_exact(8);
    let rem_a = chunks_a.remainder();
    let rem_b = chunks_b.remainder();

    let mut sum0 = 0.0f32;
    let mut sum1 = 0.0f32;
    let mut sum2 = 0.0f32;
    let mut sum3 = 0.0f32;
    let mut sum4 = 0.0f32;
    let mut sum5 = 0.0f32;
    let mut sum6 = 0.0f32;
    let mut sum7 = 0.0f32;

    for (ca, cb) in chunks_a.zip(chunks_b) {
        sum0 += ca[0] * cb[0];
        sum1 += ca[1] * cb[1];
        sum2 += ca[2] * cb[2];
        sum3 += ca[3] * cb[3];
        sum4 += ca[4] * cb[4];
        sum5 += ca[5] * cb[5];
        sum6 += ca[6] * cb[6];
        sum7 += ca[7] * cb[7];
    }

    let mut total = (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7);

    for (&x, &y) in rem_a.iter().zip(rem_b.iter()) {
        total += x * y;
    }

    total
}

/// Computes Dot Product distance: `-dot_product(a, b)`.
#[inline]
pub fn dot_product_distance(a: &[f32], b: &[f32]) -> f32 {
    -dot_product(a, b)
}

/// Computes Cosine distance: `1.0 - (dot(a, b) / (norm(a) * norm(b)))`.
#[inline]
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let chunks_a = a.chunks_exact(8);
    let chunks_b = b.chunks_exact(8);
    let rem_a = chunks_a.remainder();
    let rem_b = chunks_b.remainder();

    let mut dot0 = 0.0f32;
    let mut dot1 = 0.0f32;
    let mut dot2 = 0.0f32;
    let mut dot3 = 0.0f32;
    let mut dot4 = 0.0f32;
    let mut dot5 = 0.0f32;
    let mut dot6 = 0.0f32;
    let mut dot7 = 0.0f32;

    let mut norm_a0 = 0.0f32;
    let mut norm_a1 = 0.0f32;
    let mut norm_a2 = 0.0f32;
    let mut norm_a3 = 0.0f32;
    let mut norm_a4 = 0.0f32;
    let mut norm_a5 = 0.0f32;
    let mut norm_a6 = 0.0f32;
    let mut norm_a7 = 0.0f32;

    let mut norm_b0 = 0.0f32;
    let mut norm_b1 = 0.0f32;
    let mut norm_b2 = 0.0f32;
    let mut norm_b3 = 0.0f32;
    let mut norm_b4 = 0.0f32;
    let mut norm_b5 = 0.0f32;
    let mut norm_b6 = 0.0f32;
    let mut norm_b7 = 0.0f32;

    for (ca, cb) in chunks_a.zip(chunks_b) {
        dot0 += ca[0] * cb[0];
        dot1 += ca[1] * cb[1];
        dot2 += ca[2] * cb[2];
        dot3 += ca[3] * cb[3];
        dot4 += ca[4] * cb[4];
        dot5 += ca[5] * cb[5];
        dot6 += ca[6] * cb[6];
        dot7 += ca[7] * cb[7];

        norm_a0 += ca[0] * ca[0];
        norm_a1 += ca[1] * ca[1];
        norm_a2 += ca[2] * ca[2];
        norm_a3 += ca[3] * ca[3];
        norm_a4 += ca[4] * ca[4];
        norm_a5 += ca[5] * ca[5];
        norm_a6 += ca[6] * ca[6];
        norm_a7 += ca[7] * ca[7];

        norm_b0 += cb[0] * cb[0];
        norm_b1 += cb[1] * cb[1];
        norm_b2 += cb[2] * cb[2];
        norm_b3 += cb[3] * cb[3];
        norm_b4 += cb[4] * cb[4];
        norm_b5 += cb[5] * cb[5];
        norm_b6 += cb[6] * cb[6];
        norm_b7 += cb[7] * cb[7];
    }

    let mut dot = (dot0 + dot1) + (dot2 + dot3) + (dot4 + dot5) + (dot6 + dot7);
    let mut norm_a = (norm_a0 + norm_a1) + (norm_a2 + norm_a3) + (norm_a4 + norm_a5) + (norm_a6 + norm_a7);
    let mut norm_b = (norm_b0 + norm_b1) + (norm_b2 + norm_b3) + (norm_b4 + norm_b5) + (norm_b6 + norm_b7);

    for (&x, &y) in rem_a.iter().zip(rem_b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let norm_product = (norm_a * norm_b).sqrt();
    if norm_product == 0.0 {
        return 1.0; // orthogonal or zero vector fallback
    }

    let sim = dot / norm_product;
    // Clamp to [-1.0, 1.0] to prevent floating point inaccuracies
    let clamped_sim = sim.clamp(-1.0, 1.0);
    1.0 - clamped_sim
}

/// Computes Manhattan (L1) distance.
#[inline]
pub fn manhattan_distance(a: &[f32], b: &[f32]) -> f32 {
    let chunks_a = a.chunks_exact(8);
    let chunks_b = b.chunks_exact(8);
    let rem_a = chunks_a.remainder();
    let rem_b = chunks_b.remainder();

    let mut sum0 = 0.0f32;
    let mut sum1 = 0.0f32;
    let mut sum2 = 0.0f32;
    let mut sum3 = 0.0f32;
    let mut sum4 = 0.0f32;
    let mut sum5 = 0.0f32;
    let mut sum6 = 0.0f32;
    let mut sum7 = 0.0f32;

    for (ca, cb) in chunks_a.zip(chunks_b) {
        sum0 += (ca[0] - cb[0]).abs();
        sum1 += (ca[1] - cb[1]).abs();
        sum2 += (ca[2] - cb[2]).abs();
        sum3 += (ca[3] - cb[3]).abs();
        sum4 += (ca[4] - cb[4]).abs();
        sum5 += (ca[5] - cb[5]).abs();
        sum6 += (ca[6] - cb[6]).abs();
        sum7 += (ca[7] - cb[7]).abs();
    }

    let mut total = (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7);

    for (&x, &y) in rem_a.iter().zip(rem_b.iter()) {
        total += (x - y).abs();
    }

    total
}

/// Normalizes a vector in-place to unit Euclidean length (L2 norm = 1.0).
#[inline]
pub fn normalize_vector(v: &mut [f32]) {
    let norm_sq = dot_product(v, v);
    if norm_sq > 0.0 {
        let inv_norm = 1.0 / norm_sq.sqrt();
        for val in v.iter_mut() {
            *val *= inv_norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 6.0, 3.0];
        // (4-1)^2 + (6-2)^2 + (3-3)^2 = 9 + 16 + 0 = 25 -> sqrt = 5.0
        assert_eq!(euclidean_distance(&a, &b), 5.0);
    }

    #[test]
    fn test_cosine() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_distance(&a, &b) - 0.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_distance(&a, &c) - 1.0).abs() < 1e-6);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_distance(&a, &d) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        // 4 + 10 + 18 = 32
        assert_eq!(dot_product(&a, &b), 32.0);
    }

    #[test]
    fn test_manhattan() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 0.0, 7.0];
        // |1-4| + |2-0| + |3-7| = 3 + 2 + 4 = 9
        assert_eq!(manhattan_distance(&a, &b), 9.0);
    }
}

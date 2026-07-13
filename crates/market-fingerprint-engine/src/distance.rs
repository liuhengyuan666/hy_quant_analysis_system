//! Distance metrics for feature vector comparison.
//!
//! All distance metrics are consumers of `FeatureVector` and are replaceable
//! via the `DistanceMetric` trait.

use crate::normalize::FeatureVector;

/// A pluggable distance metric between two `FeatureVector`s.
///
/// Implementors compute a distance in [0, 1] where 0 means identical
/// and 1 means maximally dissimilar.
pub trait DistanceMetric {
    /// Compute the distance between two feature vectors.
    ///
    /// Returns a value clamped to [0, 1] where lower values indicate
    /// higher similarity.
    fn distance(&self, a: &FeatureVector, b: &FeatureVector) -> f64;
}

/// Default distance metric using cosine distance.
///
/// Distance = 1.0 - cosine_similarity, clamped to [0, 1].
/// Cosine similarity is computed as `dot(a, b) / (|a| * |b|)`.
#[derive(Debug, Clone, Copy)]
pub struct CosineDistance;

impl DistanceMetric for CosineDistance {
    fn distance(&self, a: &FeatureVector, b: &FeatureVector) -> f64 {
        let dot = a.environment * b.environment
            + a.signal * b.signal
            + a.stretch * b.stretch
            + a.rotation * b.rotation
            + a.confirmation * b.confirmation
            + a.recovery * b.recovery;

        let norm_a = (a.environment.powi(2)
            + a.signal.powi(2)
            + a.stretch.powi(2)
            + a.rotation.powi(2)
            + a.confirmation.powi(2)
            + a.recovery.powi(2))
        .sqrt();

        let norm_b = (b.environment.powi(2)
            + b.signal.powi(2)
            + b.stretch.powi(2)
            + b.rotation.powi(2)
            + b.confirmation.powi(2)
            + b.recovery.powi(2))
        .sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 1.0; // zero vector = maximally dissimilar
        }

        let cos_sim = dot / (norm_a * norm_b);
        // Clamp cosine similarity to [-1, 1] for floating-point safety, then invert
        let clamped = cos_sim.clamp(-1.0, 1.0);
        let distance = 1.0 - clamped;
        distance.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fv(values: [f64; 6]) -> FeatureVector {
        FeatureVector {
            environment: values[0],
            signal: values[1],
            stretch: values[2],
            rotation: values[3],
            confirmation: values[4],
            recovery: values[5],
        }
    }

    #[test]
    fn identical_vectors_distance_zero() {
        let metric = CosineDistance;
        let a = fv([1.0, 2.0, 3.0, 0.5, -1.0, 0.0]);
        let d = metric.distance(&a, &a);
        assert!((d - 0.0).abs() < 1e-10);
    }

    #[test]
    fn opposite_vectors_distance_max() {
        let metric = CosineDistance;
        let a = fv([1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let b = fv([-1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let d = metric.distance(&a, &b);
        // cos_sim = -1, distance = 1 - (-1) = 2, clamped to 1.0
        assert!((d - 1.0).abs() < 1e-10);
    }

    #[test]
    fn orthogonal_vectors_distance_one() {
        let metric = CosineDistance;
        let a = fv([1.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let b = fv([0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
        let d = metric.distance(&a, &b);
        // cos_sim = 0, distance = 1.0
        assert!((d - 1.0).abs() < 1e-10);
    }

    #[test]
    fn zero_vectors_distance_one() {
        let metric = CosineDistance;
        let a = fv([0.0; 6]);
        let b = fv([1.0; 6]);
        let d = metric.distance(&a, &b);
        assert!((d - 1.0).abs() < 1e-10);
    }

    #[test]
    fn distance_in_range() {
        let metric = CosineDistance;
        let a = fv([0.5, 0.3, -0.2, 1.0, -0.5, 0.1]);
        let b = fv([0.6, 0.2, -0.1, 0.8, -0.4, 0.2]);
        let d = metric.distance(&a, &b);
        assert!(d >= 0.0);
        assert!(d <= 1.0);
    }
}

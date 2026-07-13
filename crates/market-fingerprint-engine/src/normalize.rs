//! Feature Vector normalization.
//!
//! Converts `MarketFingerprint` objects into 6-dimensional `FeatureVector` scalars
//! and applies z-score normalization across the historical window.

use crate::fingerprint::MarketFingerprint;

/// A 6-dimensional normalized feature vector derived from a `MarketFingerprint`.
///
/// All fields are z-score normalized across the historical window so that
/// each dimension contributes equally to distance calculations.
#[derive(Debug, Clone, Copy)]
pub struct FeatureVector {
    /// Environment composite score (z-score).
    pub environment: f64,
    /// Signal composite score (z-score).
    pub signal: f64,
    /// Market stretch composite score (z-score).
    pub stretch: f64,
    /// Rotation momentum scalar — average momentum of top 3 symbols (z-score).
    pub rotation: f64,
    /// Confirmation composite score (z-score).
    pub confirmation: f64,
    /// Recovery index (z-score).
    pub recovery: f64,
}

/// Extract the rotation scalar from a `MarketFingerprint`.
///
/// Uses the average momentum score of the top 3 rotation symbols.
/// Returns 0.0 if no rotation data is available.
fn rotation_scalar(fp: &MarketFingerprint) -> f64 {
    let mut scores: Vec<f64> = fp.observation.rotation.iter().map(|(_, s)| *s).collect();
    scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let top: Vec<f64> = scores.into_iter().take(3).collect();
    if top.is_empty() {
        0.0
    } else {
        top.iter().sum::<f64>() / top.len() as f64
    }
}

/// Extract all 6 raw scalars from a fingerprint before normalization.
fn extract_raw(fp: &MarketFingerprint) -> [f64; 6] {
    [
        fp.observation.environment,
        fp.observation.signal,
        fp.observation.stretch,
        rotation_scalar(fp),
        fp.evolution.confirmation,
        fp.evolution.recovery,
    ]
}

/// Z-score normalize a slice of values.
///
/// Returns `(value - mean) / std` for each value. If `std == 0`, returns `0.0`.
fn zscore(values: &[f64]) -> Vec<f64> {
    let n = values.len() as f64;
    if n < 2.0 {
        return vec![0.0; values.len()];
    }
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std = variance.sqrt();
    if std == 0.0 {
        vec![0.0; values.len()]
    } else {
        values.iter().map(|v| (v - mean) / std).collect()
    }
}

/// Normalize a slice of fingerprints into z-scored `FeatureVector`s.
///
/// Each of the 6 dimensions is normalized independently across the window.
pub fn normalize_all(fingerprints: &[MarketFingerprint]) -> Vec<FeatureVector> {
    if fingerprints.is_empty() {
        return Vec::new();
    }

    // Extract raw scalars per dimension
    let mut raw: [Vec<f64>; 6] = [
        Vec::with_capacity(fingerprints.len()),
        Vec::with_capacity(fingerprints.len()),
        Vec::with_capacity(fingerprints.len()),
        Vec::with_capacity(fingerprints.len()),
        Vec::with_capacity(fingerprints.len()),
        Vec::with_capacity(fingerprints.len()),
    ];

    for fp in fingerprints {
        let r = extract_raw(fp);
        for i in 0..6 {
            raw[i].push(r[i]);
        }
    }

    // Z-score each dimension
    let normalized: [Vec<f64>; 6] = [
        zscore(&raw[0]),
        zscore(&raw[1]),
        zscore(&raw[2]),
        zscore(&raw[3]),
        zscore(&raw[4]),
        zscore(&raw[5]),
    ];

    // Build FeatureVectors
    (0..fingerprints.len())
        .map(|i| FeatureVector {
            environment: normalized[0][i],
            signal: normalized[1][i],
            stretch: normalized[2][i],
            rotation: normalized[3][i],
            confirmation: normalized[4][i],
            recovery: normalized[5][i],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::{EvolutionVector, ObservationVector};
    use chrono::NaiveDate;
    use core_domain::AnalysisScope;

    fn make_fp(values: [f64; 6]) -> MarketFingerprint {
        MarketFingerprint {
            scope: AnalysisScope::Global,
            date: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
            observation: ObservationVector {
                environment: values[0],
                signal: values[1],
                stretch: values[2],
                rotation: vec![("A".to_string(), values[3]), ("B".to_string(), values[3] * 0.8)],
            },
            evolution: EvolutionVector {
                confirmation: values[4],
                recovery: values[5],
            },
        }
    }

    #[test]
    fn normalize_all_returns_correct_count() {
        let fps: Vec<MarketFingerprint> = (0..10)
            .map(|i| make_fp([i as f64 * 10.0; 6]))
            .collect();
        let result = normalize_all(&fps);
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn normalize_all_empty_returns_empty() {
        let result = normalize_all(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn normalize_all_constant_values_are_zero() {
        let fps: Vec<MarketFingerprint> = (0..10)
            .map(|_| make_fp([50.0; 6]))
            .collect();
        let result = normalize_all(&fps);
        for fv in &result {
            assert!((fv.environment - 0.0).abs() < 1e-10);
            assert!((fv.signal - 0.0).abs() < 1e-10);
            assert!((fv.stretch - 0.0).abs() < 1e-10);
            assert!((fv.rotation - 0.0).abs() < 1e-10);
            assert!((fv.confirmation - 0.0).abs() < 1e-10);
            assert!((fv.recovery - 0.0).abs() < 1e-10);
        }
    }

    #[test]
    fn normalize_all_single_fingerprint_returns_zero() {
        let fps = vec![make_fp([10.0, 20.0, 30.0, 40.0, 50.0, 60.0])];
        let result = normalize_all(&fps);
        assert_eq!(result.len(), 1);
        // Single element means std is 0, all values are 0
        let fv = &result[0];
        assert!((fv.environment - 0.0).abs() < 1e-10);
        assert!((fv.signal - 0.0).abs() < 1e-10);
        assert!((fv.stretch - 0.0).abs() < 1e-10);
        assert!((fv.rotation - 0.0).abs() < 1e-10);
        assert!((fv.confirmation - 0.0).abs() < 1e-10);
        assert!((fv.recovery - 0.0).abs() < 1e-10);
    }

    #[test]
    fn rotation_scalar_uses_top3_avg() {
        let fp = MarketFingerprint {
            scope: AnalysisScope::Global,
            date: NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
            observation: ObservationVector {
                environment: 50.0,
                signal: 50.0,
                stretch: 50.0,
                rotation: vec![
                    ("A".to_string(), 100.0),
                    ("B".to_string(), 90.0),
                    ("C".to_string(), 80.0),
                    ("D".to_string(), 10.0),
                    ("E".to_string(), 5.0),
                ],
            },
            evolution: EvolutionVector {
                confirmation: 50.0,
                recovery: 50.0,
            },
        };
        let scalar = rotation_scalar(&fp);
        assert!((scalar - 90.0).abs() < 0.01); // (100+90+80)/3 = 90
    }

    #[test]
    fn rotation_scalar_fewer_than_3_uses_all() {
        let fp = MarketFingerprint {
            scope: AnalysisScope::Global,
            date: NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
            observation: ObservationVector {
                environment: 50.0,
                signal: 50.0,
                stretch: 50.0,
                rotation: vec![("A".to_string(), 100.0), ("B".to_string(), 60.0)],
            },
            evolution: EvolutionVector {
                confirmation: 50.0,
                recovery: 50.0,
            },
        };
        let scalar = rotation_scalar(&fp);
        assert!((scalar - 80.0).abs() < 0.01); // (100+60)/2 = 80
    }

    #[test]
    fn rotation_scalar_empty_returns_zero() {
        let fp = MarketFingerprint {
            scope: AnalysisScope::Global,
            date: NaiveDate::from_ymd_opt(2026, 7, 1).unwrap(),
            observation: ObservationVector {
                environment: 50.0,
                signal: 50.0,
                stretch: 50.0,
                rotation: vec![],
            },
            evolution: EvolutionVector {
                confirmation: 50.0,
                recovery: 50.0,
            },
        };
        let scalar = rotation_scalar(&fp);
        assert!((scalar - 0.0).abs() < 1e-10);
    }
}

//! Canonical Market Fingerprint types.

use chrono::NaiveDate;
use core_domain::AnalysisScope;
use serde::{Deserialize, Serialize};

/// A canonical historical feature representation of a market state on a given date.
///
/// This struct is intentionally grouped by semantic layer so that future additions
/// (e.g. Consensus, Fear, Liquidity) can be added without reshaping the whole contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketFingerprint {
    pub scope: AnalysisScope,
    pub date: NaiveDate,
    pub observation: ObservationVector,
    pub evolution: EvolutionVector,
}

/// Observation-layer features: what the market is doing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationVector {
    /// Environment layer composite score (0-100).
    pub environment: f64,
    /// Signal layer composite score (0-100).
    pub signal: f64,
    /// Market stretch composite score (0-100).
    pub stretch: f64,
    /// Rotation leadership structure: symbol -> momentum score.
    pub rotation: Vec<(String, f64)>,
}

/// Evolution-layer features: where the market is evolving.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionVector {
    /// Market confirmation composite score (0-100).
    pub confirmation: f64,
    /// Recovery index (0-100).
    pub recovery: f64,
}

impl MarketFingerprint {
    /// Version of the fingerprint schema. Bump when adding new top-level fields.
    pub const SCHEMA_VERSION: u32 = 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_fields_are_grouped() {
        let fp = MarketFingerprint {
            scope: AnalysisScope::Global,
            date: NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
            observation: ObservationVector {
                environment: 55.0,
                signal: 72.0,
                stretch: 35.0,
                rotation: vec![("TECH".to_string(), 95.0)],
            },
            evolution: EvolutionVector {
                confirmation: 60.0,
                recovery: 42.0,
            },
        };

        assert_eq!(fp.observation.environment, 55.0);
        assert_eq!(fp.evolution.recovery, 42.0);
    }

    #[test]
    fn fingerprint_serde_roundtrip() {
        let fp = MarketFingerprint {
            scope: AnalysisScope::Cn,
            date: NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
            observation: ObservationVector {
                environment: 55.0,
                signal: 72.0,
                stretch: 35.0,
                rotation: vec![("TECH".to_string(), 95.0)],
            },
            evolution: EvolutionVector {
                confirmation: 60.0,
                recovery: 42.0,
            },
        };

        let json = serde_json::to_string(&fp).unwrap();
        let parsed: MarketFingerprint = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.scope, AnalysisScope::Cn);
        assert_eq!(parsed.observation.signal, 72.0);
    }
}

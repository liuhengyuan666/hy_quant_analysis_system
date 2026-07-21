use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single validation case for the Execution Platform.
///
/// Each case is a historical symbol/date that exercises a specific Decision
/// Boundary. It is not a test assertion by itself; it is the input to a manual
/// review process documented in `research/validation/execution/README.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCase {
    pub id: String,
    pub symbol: String,
    pub date: NaiveDate,
    pub scope: String,
    pub market_regime: String,
    pub pattern_type: String,
    pub expected_decision: String,
    pub reason: String,
    pub validated_by: String,
    pub notes: String,
}

/// A historical candidate discovered from persisted data that has complete
/// inputs (signal, strategy state, daily bar) and can therefore be run through
/// the Execution Pipeline. It is a pre-cursor to a `ValidationCase`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCandidate {
    pub symbol: String,
    pub date: NaiveDate,
    pub scope: String,
    pub signal_label: String,
    pub signal_score: f64,
    pub strategy_state: String,
    pub market_regime_label: String,
    pub decision_state: String,
    pub confidence: f64,
    pub risk: String,
    pub evidence_count: usize,
}

/// Schema metadata describing the YAML fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSchema {
    pub id: String,
    pub symbol: String,
    pub date: String,
    pub scope: String,
    pub market_regime: String,
    pub pattern_type: String,
    pub expected_decision: String,
    pub reason: String,
    pub validated_by: String,
    pub notes: String,
}

/// The golden validation suite for the Execution Platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSuite {
    pub suite: String,
    pub version: String,
    pub description: String,
    pub schema: ValidationSchema,
    pub cases: Vec<ValidationCase>,
}

impl ValidationSuite {
    /// Loads a validation suite from a YAML file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path.as_ref())
            .map_err(|e| anyhow::anyhow!("failed to read suite file: {}", e))?;
        let suite: Self = serde_yaml::from_str(&text)
            .map_err(|e| anyhow::anyhow!("failed to parse suite YAML: {}", e))?;
        Ok(suite)
    }

    /// Returns the number of cases in the suite.
    pub fn len(&self) -> usize {
        self.cases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn suite_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR not set")
        )
        .join("../../research/validation/execution/execution_validation_suite.yaml")
    }

    #[test]
    fn golden_suite_loads() {
        let suite = ValidationSuite::from_file(suite_path()).expect("suite should load");
        assert_eq!(suite.suite, "execution-platform-v2");
        assert_eq!(suite.version, "1.0.1");
        assert_eq!(suite.len(), 10);
    }

    #[test]
    fn suite_covers_decision_boundaries() {
        let suite = ValidationSuite::from_file(suite_path()).unwrap();
        let decisions: std::collections::HashSet<_> = suite
            .cases
            .iter()
            .map(|c| c.expected_decision.as_str())
            .collect();
        assert!(decisions.contains("BuyNow"));
        assert!(decisions.contains("Wait"));
        assert!(decisions.contains("Reduce"));
    }

    #[test]
    fn all_cases_have_required_fields() {
        let suite = ValidationSuite::from_file(suite_path()).unwrap();
        for case in &suite.cases {
            assert!(!case.id.is_empty());
            assert!(!case.symbol.is_empty());
            assert!(!case.scope.is_empty());
            assert!(!case.pattern_type.is_empty());
            assert!(!case.expected_decision.is_empty());
            assert!(!case.reason.is_empty());
        }
    }
}

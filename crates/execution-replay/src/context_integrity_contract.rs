use serde::{Deserialize, Serialize};

/// A single rule in the `ExecutionContextIntegrityContract`.
///
/// The contract protects the ResearchContext → ExecutionEvent fact lineage by
/// forbidding fields that are constant, placeholder-valued, low-variance, or
/// dominated by a single value. Each rule is evaluated against the population of
/// `ExecutionResearchRecord`s in a replay dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextIntegrityRule {
    pub field_name: String,
    pub min_variance: f64,
    pub min_unique_ratio: f64,
    pub max_dominant_value_ratio: f64,
    pub known_placeholders: Vec<f64>,
}

/// Default contract for V8 Execution Platform ResearchContext-derived fields.
///
/// This contract is the canonical Fact Integrity Firewall. Any new field that
/// flows from `ResearchContext` into `ExecutionMarketView` should be added here
/// before it is consumed by Evidence Modeling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContextIntegrityContract {
    pub rules: Vec<ContextIntegrityRule>,
}

impl ExecutionContextIntegrityContract {
    /// Returns the standard contract used by the CI gate and CLI validator.
    pub fn v8_default() -> Self {
        Self {
            rules: vec![
                ContextIntegrityRule {
                    field_name: "confirmation.trend.score".into(),
                    min_variance: 1.0,
                    min_unique_ratio: 0.001,
                    max_dominant_value_ratio: 0.95,
                    known_placeholders: vec![],
                },
                ContextIntegrityRule {
                    field_name: "confirmation.participation.score".into(),
                    min_variance: 1.0,
                    min_unique_ratio: 0.001,
                    max_dominant_value_ratio: 0.95,
                    known_placeholders: vec![],
                },
                ContextIntegrityRule {
                    field_name: "confirmation.risk.score".into(),
                    min_variance: 1.0,
                    min_unique_ratio: 0.001,
                    max_dominant_value_ratio: 0.95,
                    known_placeholders: vec![],
                },
                ContextIntegrityRule {
                    field_name: "breadth.breadth_pct".into(),
                    min_variance: 1.0,
                    min_unique_ratio: 0.001,
                    max_dominant_value_ratio: 0.95,
                    known_placeholders: vec![50.0],
                },
                ContextIntegrityRule {
                    field_name: "breadth.delta_5d".into(),
                    min_variance: 0.1,
                    min_unique_ratio: 0.001,
                    max_dominant_value_ratio: 0.95,
                    known_placeholders: vec![0.0],
                },
                ContextIntegrityRule {
                    field_name: "breadth.sma5".into(),
                    min_variance: 1.0,
                    min_unique_ratio: 0.001,
                    max_dominant_value_ratio: 0.95,
                    known_placeholders: vec![0.0],
                },
                ContextIntegrityRule {
                    field_name: "recovery.score".into(),
                    min_variance: 1.0,
                    min_unique_ratio: 0.001,
                    max_dominant_value_ratio: 0.95,
                    known_placeholders: vec![],
                },
                ContextIntegrityRule {
                    field_name: "leadership_stability".into(),
                    min_variance: 1e-6,
                    min_unique_ratio: 0.001,
                    max_dominant_value_ratio: 0.95,
                    known_placeholders: vec![0.5],
                },
            ],
        }
    }

    /// Finds the rule for a given field, if one exists.
    pub fn rule_for(&self, field_name: &str) -> Option<&ContextIntegrityRule> {
        self.rules.iter().find(|r| r.field_name == field_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_contract_covers_all_research_context_fields() {
        let contract = ExecutionContextIntegrityContract::v8_default();
        let names: Vec<_> = contract.rules.iter().map(|r| r.field_name.clone()).collect();
        assert!(names.contains(&"confirmation.trend.score".to_string()));
        assert!(names.contains(&"breadth.breadth_pct".to_string()));
        assert!(names.contains(&"leadership_stability".to_string()));
        assert_eq!(contract.rules.len(), 8);
    }

    #[test]
    fn rule_for_returns_expected_rule() {
        let contract = ExecutionContextIntegrityContract::v8_default();
        assert!(contract.rule_for("breadth.breadth_pct").is_some());
        assert!(contract.rule_for("unknown").is_none());
    }
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    context_integrity_contract::{ContextIntegrityRule, ExecutionContextIntegrityContract},
    ExecutionResearchRecord,
};

/// Strict pass/fail validation of ResearchContext-derived fields.
///
/// `ContextIntegrityValidation` is the result of applying the
/// `ExecutionContextIntegrityContract` to a replay dataset. It is intentionally
/// separate from the diagnostic `ContextIntegrityReport`: this struct answers
/// "can we safely proceed with Evidence Modeling?" with a boolean `passed`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextIntegrityValidation {
    pub total_records: usize,
    pub fields: Vec<FieldValidation>,
    pub passed: bool,
    pub verdict: String,
}

/// Per-field validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValidation {
    pub field_name: String,
    pub sample_count: usize,
    pub unique_values: usize,
    pub unique_ratio: f64,
    pub dominant_value: Option<f64>,
    pub dominant_value_ratio: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub variance: f64,
    pub known_placeholder_detected: bool,
    pub placeholder_value: Option<f64>,
    pub violations: Vec<ContextIntegrityViolation>,
    pub passed: bool,
}

/// A specific contract breach for a field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "detail")]
pub enum ContextIntegrityViolation {
    Placeholder { value: f64 },
    LowVariance { variance: f64, threshold: f64 },
    LowUniqueRatio { ratio: f64, threshold: f64 },
    HighDominantValueRatio { ratio: f64, threshold: f64, value: f64 },
}

/// Validates a population of records against the default V8 contract.
///
/// Returns a `ContextIntegrityValidation` with `passed == true` only when every
/// audited field satisfies all contract rules.
pub fn validate_execution_context(
    records: &[ExecutionResearchRecord],
) -> ContextIntegrityValidation {
    validate_with_contract(records, &ExecutionContextIntegrityContract::v8_default())
}

/// Validates a population of records against an arbitrary contract.
pub fn validate_with_contract(
    records: &[ExecutionResearchRecord],
    contract: &ExecutionContextIntegrityContract,
) -> ContextIntegrityValidation {
    let extractors: Vec<(&str, fn(&ExecutionResearchRecord) -> f64)> = vec![
        (
            "confirmation.trend.score",
            |r| r.event.request.market_view.confirmation.trend.score,
        ),
        (
            "confirmation.participation.score",
            |r| r.event.request.market_view.confirmation.participation.score,
        ),
        (
            "confirmation.risk.score",
            |r| r.event.request.market_view.confirmation.risk.score,
        ),
        (
            "breadth.breadth_pct",
            |r| r.event.request.market_view.breadth.breadth_pct,
        ),
        (
            "breadth.delta_5d",
            |r| r.event.request.market_view.breadth.delta_5d.unwrap_or(0.0),
        ),
        (
            "breadth.sma5",
            |r| r.event.request.market_view.breadth.sma5.unwrap_or(0.0),
        ),
        ("recovery.score", |r| r.event.request.market_view.recovery.score),
        (
            "leadership_stability",
            |r| r.event.request.market_view.leadership_stability,
        ),
    ];

    let mut fields = Vec::new();
    for (name, extractor) in extractors {
        if let Some(rule) = contract.rule_for(name) {
            let validation = validate_field(records, name, extractor, rule);
            fields.push(validation);
        }
    }

    let passed = fields.iter().all(|f| f.passed);
    let total_records = records.len();
    let verdict = build_verdict(&fields, total_records);

    ContextIntegrityValidation {
        total_records,
        fields,
        passed,
        verdict,
    }
}

fn validate_field(
    records: &[ExecutionResearchRecord],
    field_name: &str,
    extractor: fn(&ExecutionResearchRecord) -> f64,
    rule: &ContextIntegrityRule,
) -> FieldValidation {
    let values: Vec<f64> = records.iter().map(extractor).collect();
    let sample_count = values.len();

    let mut value_counts: HashMap<i64, usize> = HashMap::new();
    for v in &values {
        let key = (*v * 1_000_000.0).round() as i64;
        *value_counts.entry(key).or_insert(0) += 1;
    }

    let unique_values = value_counts.len();
    let unique_ratio = if sample_count == 0 {
        0.0
    } else {
        unique_values as f64 / sample_count as f64
    };

    let (dominant_value, dominant_count) = value_counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(k, count)| (*k as f64 / 1_000_000.0, *count))
        .unwrap_or((0.0, 0));
    let dominant_value_ratio = if sample_count == 0 {
        0.0
    } else {
        dominant_count as f64 / sample_count as f64
    };

    let min = if values.is_empty() {
        0.0
    } else {
        values
            .iter()
            .cloned()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    };
    let max = if values.is_empty() {
        0.0
    } else {
        values
            .iter()
            .cloned()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    };
    let mean = if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    };
    let variance = if values.is_empty() {
        0.0
    } else {
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
    };

    let mut violations = Vec::new();
    let mut known_placeholder_detected = false;
    let mut placeholder_value: Option<f64> = None;

    for ph in &rule.known_placeholders {
        if values.iter().all(|v| (v - ph).abs() < 1e-9) {
            known_placeholder_detected = true;
            placeholder_value = Some(*ph);
            violations.push(ContextIntegrityViolation::Placeholder { value: *ph });
        }
    }

    if variance < rule.min_variance {
        violations.push(ContextIntegrityViolation::LowVariance {
            variance,
            threshold: rule.min_variance,
        });
    }

    if unique_ratio < rule.min_unique_ratio {
        violations.push(ContextIntegrityViolation::LowUniqueRatio {
            ratio: unique_ratio,
            threshold: rule.min_unique_ratio,
        });
    }

    if dominant_value_ratio > rule.max_dominant_value_ratio {
        violations.push(ContextIntegrityViolation::HighDominantValueRatio {
            ratio: dominant_value_ratio,
            threshold: rule.max_dominant_value_ratio,
            value: dominant_value,
        });
    }

    let passed = violations.is_empty();

    FieldValidation {
        field_name: field_name.to_string(),
        sample_count,
        unique_values,
        unique_ratio,
        dominant_value: Some(dominant_value),
        dominant_value_ratio,
        min,
        max,
        mean,
        variance,
        known_placeholder_detected,
        placeholder_value,
        violations,
        passed,
    }
}

fn build_verdict(fields: &[FieldValidation], total_records: usize) -> String {
    let failed: Vec<&FieldValidation> = fields.iter().filter(|f| !f.passed).collect();

    if failed.is_empty() {
        return format!(
            "Context Integrity Gate PASS: all {} fields satisfy the V8 Fact Integrity Contract across {} records.",
            fields.len(),
            total_records
        );
    }

    let mut parts = Vec::new();
    parts.push(format!(
        "Context Integrity Gate FAIL: {}/{} fields violate the V8 Fact Integrity Contract across {} records.",
        failed.len(),
        fields.len(),
        total_records
    ));
    for f in &failed {
        let violation_names: Vec<String> = f
            .violations
            .iter()
            .map(|v| match v {
                ContextIntegrityViolation::Placeholder { value } => {
                    format!("placeholder({:.2})", value)
                }
                ContextIntegrityViolation::LowVariance { variance, threshold } => {
                    format!("low_variance({:.4e} < {:.4e})", variance, threshold)
                }
                ContextIntegrityViolation::LowUniqueRatio { ratio, threshold } => {
                    format!("low_unique_ratio({:.4e} < {:.4e})", ratio, threshold)
                }
                ContextIntegrityViolation::HighDominantValueRatio {
                    ratio,
                    threshold,
                    value,
                } => {
                    format!(
                        "high_dominant_ratio({:.2}% > {:.2}% at {:.2})",
                        ratio * 100.0,
                        threshold * 100.0,
                        value
                    )
                }
            })
            .collect();
        parts.push(format!(
            "- {}: {} | min={:.2} max={:.2} variance={:.4e} unique_ratio={:.4e} dominant_ratio={:.2}%",
            f.field_name,
            violation_names.join(", "),
            f.min,
            f.max,
            f.variance,
            f.unique_ratio,
            f.dominant_value_ratio * 100.0
        ));
    }
    parts.push("Evidence Modeling and Transition Analysis must remain blocked until these fields are fixed.".to_string());
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use execution_engine::v2::assessment::{ExecutionAssessment, RiskLevel};
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::evidence::{Evidence, EvidenceKind, EvidencePayload, EvidenceSource};
    use execution_engine::v2::feature::IntradayFeatures;
    use execution_engine::v2::request::{
        ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
    };
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn make_record_with_fields(
        trend_score: f64,
        participation_score: f64,
        risk_score: f64,
        breadth_pct: f64,
        delta_5d: f64,
        sma5: f64,
        recovery_score: f64,
        leadership_stability: f64,
    ) -> ExecutionResearchRecord {
        let policy = ExecutionPolicy::default();
        let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap();
        let request = ExecutionRequest {
            symbol: "000001".into(),
            date,
            signal: core_domain::SignalSnapshot {
                date,
                symbol: "000001".into(),
                final_score: 70.0,
                signal_label: SignalLabel::Buy,
                analysis_scope: "CN".into(),
                regime_basis_scope: "CN".into(),
                reason: core_domain::SignalReason {
                    best_strategy: StrategyKind::MomentumRight,
                    strategy_score: 0.0,
                    strategy_contribution: 0.0,
                    alignment: 0,
                    aligned_strategies: vec![],
                    alignment_contribution: 0.0,
                    regime: core_domain::RegimeReason {
                        trend_score: 0.0,
                        risk_score: 0.0,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    rotation: core_domain::RotationReason {
                        momentum_score: 0.0,
                        rank: None,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    final_score: 70.0,
                    label: SignalLabel::Buy,
                    summary: "test".into(),
                },
            },
            strategy_state: core_domain::StrategyStateSnapshot {
                date,
                scope: "CN".into(),
                state: StrategyState::NoTrade,
                state_score: 50.0,
                transition_reason: "test".into(),
                recommended_position_pct: 0.0,
            },
            quote: QuoteSnapshot {
                symbol: "000001".into(),
                ts: Utc::now(),
                open: 10.0,
                high: 11.0,
                low: 9.0,
                close: 10.5,
                volume: 1_000_000.0,
                prev_close: 10.0,
            },
            volume_ma20: 500_000.0,
            market_view: ExecutionMarketView {
                research_version: "1".into(),
                market_regime_label: "Bullish".into(),
                confirmation: ConfirmationSummary {
                    trend: ConfirmationDimension {
                        score: trend_score,
                        label: "Moderate".into(),
                    },
                    participation: ConfirmationDimension {
                        score: participation_score,
                        label: "Moderate".into(),
                    },
                    risk: ConfirmationDimension {
                        score: risk_score,
                        label: "Moderate".into(),
                    },
                    overall: "Moderate".into(),
                },
                breadth: BreadthSummary {
                    breadth_pct,
                    sma5: Some(sma5),
                    delta_5d: Some(delta_5d),
                    condition: "moderate".into(),
                },
                recovery: RecoverySummary {
                    score: recovery_score,
                    drivers: vec![],
                },
                rotation_state: "mixed".into(),
                leadership_stability,
            },
            policy,
        };

        let features = IntradayFeatures {
            symbol: "000001".into(),
            today_return: 0.0,
            open_return: 0.0,
            gap_pct: 0.0,
            close_position: 0.5,
            amplitude_pct: 0.02,
            upper_shadow_pct: 0.0,
            lower_shadow_pct: 0.0,
            volume_ratio: 1.0,
            body_ratio: 0.3,
            gap_fill_ratio: 0.0,
        };

        let assessment = ExecutionAssessment {
            confidence: 0.5,
            consensus: 0.6,
            coverage: 1.0,
            risk: RiskLevel::Medium,
            dominant_direction: -0.4,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };
        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: execution_engine::types::ExecutionState::Wait,
            confidence: 0.5,
            risk: RiskLevel::Medium,
            evidences: vec![Evidence {
                kind: EvidenceKind::Breadth,
                confidence: 0.8,
                direction: -1.0,
                source: EvidenceSource::ResearchContext,
                payload: EvidencePayload::Empty,
            }],
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };

        let event = ExecutionEvent::new(request, features, vec![], vec![], assessment, decision);
        ExecutionResearchRecord {
            event,
            outcome: crate::ExecutionOutcome::default(),
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn gate_fails_on_placeholder_breadth() {
        let records = vec![
            make_record_with_fields(50.0, 50.0, 50.0, 50.0, 0.0, 0.0, 55.0, 0.5),
            make_record_with_fields(50.0, 50.0, 50.0, 50.0, 0.0, 0.0, 55.0, 0.5),
            make_record_with_fields(50.0, 50.0, 50.0, 50.0, 0.0, 0.0, 55.0, 0.5),
        ];
        let validation = validate_execution_context(&records);
        assert!(!validation.passed);

        let breadth = validation
            .fields
            .iter()
            .find(|f| f.field_name == "breadth.breadth_pct")
            .expect("breadth field");
        assert!(!breadth.passed);
        assert!(breadth.known_placeholder_detected);
        assert_eq!(breadth.placeholder_value, Some(50.0));
        assert!(breadth
            .violations
            .iter()
            .any(|v| matches!(v, ContextIntegrityViolation::Placeholder { value: 50.0 })));
    }

    #[test]
    fn gate_fails_on_constant_field() {
        let records = vec![
            make_record_with_fields(35.0, 60.0, 45.0, 30.0, 2.0, 15.0, 60.0, 0.7),
            make_record_with_fields(35.0, 60.0, 45.0, 30.0, 2.0, 15.0, 60.0, 0.7),
            make_record_with_fields(35.0, 60.0, 45.0, 30.0, 2.0, 15.0, 60.0, 0.7),
        ];
        let validation = validate_execution_context(&records);
        assert!(!validation.passed);

        let breadth = validation
            .fields
            .iter()
            .find(|f| f.field_name == "breadth.breadth_pct")
            .expect("breadth field");
        assert!(!breadth.passed);
        assert!(breadth
            .violations
            .iter()
            .any(|v| matches!(v, ContextIntegrityViolation::HighDominantValueRatio { .. })));
    }

    #[test]
    fn gate_passes_on_variable_fields() {
        let records = vec![
            make_record_with_fields(35.0, 20.0, 15.0, 30.0, -5.0, 10.0, 22.0, 0.1),
            make_record_with_fields(45.0, 35.0, 27.0, 45.0, 2.0, 30.0, 40.0, 0.4),
            make_record_with_fields(55.0, 50.0, 39.0, 60.0, 7.0, 50.0, 58.0, 0.7),
        ];
        let validation = validate_execution_context(&records);
        assert!(validation.passed, "expected gate to pass: {:?}", validation);
    }

    #[test]
    fn gate_detects_high_dominant_ratio_soft_pollution() {
        // 99% of records are breadth=50.0, but one is different so variance > 0.
        // Other fields are varied so only breadth should fail.
        let mut records = vec![];
        for i in 0..99 {
            records.push(make_record_with_fields(
                35.0 + (i % 10) as f64,
                20.0 + (i % 15) as f64,
                15.0 + (i % 12) as f64,
                50.0,
                -5.0 + (i % 8) as f64,
                10.0 + (i % 20) as f64,
                22.0 + (i % 18) as f64,
                0.1 + ((i % 5) as f64) * 0.15,
            ));
        }
        records.push(make_record_with_fields(
            42.0, 35.0, 27.0, 51.0, 2.0, 30.0, 40.0, 0.4,
        ));
        let validation = validate_execution_context(&records);
        assert!(!validation.passed);

        let breadth = validation
            .fields
            .iter()
            .find(|f| f.field_name == "breadth.breadth_pct")
            .expect("breadth field");
        assert!(!breadth.passed);
        assert!(breadth
            .violations
            .iter()
            .any(|v| matches!(v, ContextIntegrityViolation::HighDominantValueRatio { .. })));
    }

    #[test]
    fn gate_verdict_matches_passed_state() {
        let records = vec![
            make_record_with_fields(35.0, 20.0, 15.0, 30.0, -5.0, 10.0, 22.0, 0.1),
            make_record_with_fields(45.0, 35.0, 27.0, 45.0, 2.0, 30.0, 40.0, 0.4),
            make_record_with_fields(55.0, 50.0, 39.0, 60.0, 7.0, 50.0, 58.0, 0.7),
        ];
        let validation = validate_execution_context(&records);
        assert!(validation.verdict.starts_with("Context Integrity Gate PASS"));
    }
}

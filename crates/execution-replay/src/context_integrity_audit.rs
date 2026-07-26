use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ExecutionResearchRecord;

/// 2B-0: ResearchContext Fact Integrity Audit.
///
/// Inspects all ResearchContext-derived fields in the ExecutionMarketView of
/// ExecutionResearchRecords and reports variance, distribution, and placeholder
/// detection. This is a read-only gate: it does not modify the Execution Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextIntegrityReport {
    pub total_records: usize,
    pub fields: Vec<FieldIntegrityReport>,
    pub verdict: String,
}

/// Per-field integrity report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldIntegrityReport {
    pub field_name: String,
    pub sample_count: usize,
    pub unique_values: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub variance: f64,
    pub known_placeholders: Vec<f64>,
    pub placeholder_detected: bool,
    pub placeholder_value: Option<f64>,
    pub status: FieldIntegrityStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FieldIntegrityStatus {
    Pass,
    Constant,
    Placeholder,
    LowVariance,
}

/// Computes a Context Integrity Report over a set of records.
pub fn compute_context_integrity_report(records: &[ExecutionResearchRecord]) -> ContextIntegrityReport {
    let total_records = records.len();

    let mut fields = Vec::new();

    fields.push(audit_field(
        records,
        "confirmation.trend.score",
        |r| r.event.request.market_view.confirmation.trend.score,
        vec![],
    ));
    fields.push(audit_field(
        records,
        "confirmation.participation.score",
        |r| r.event.request.market_view.confirmation.participation.score,
        vec![],
    ));
    fields.push(audit_field(
        records,
        "confirmation.risk.score",
        |r| r.event.request.market_view.confirmation.risk.score,
        vec![],
    ));
    fields.push(audit_field(
        records,
        "breadth.breadth_pct",
        |r| r.event.request.market_view.breadth.breadth_pct,
        vec![50.0],
    ));
    fields.push(audit_field(
        records,
        "breadth.delta_5d",
        |r| r.event.request.market_view.breadth.delta_5d.unwrap_or(0.0),
        vec![0.0],
    ));
    fields.push(audit_field(
        records,
        "breadth.sma5",
        |r| r.event.request.market_view.breadth.sma5.unwrap_or(0.0),
        vec![0.0],
    ));
    fields.push(audit_field(
        records,
        "recovery.score",
        |r| r.event.request.market_view.recovery.score,
        vec![],
    ));
    fields.push(audit_field(
        records,
        "leadership_stability",
        |r| r.event.request.market_view.leadership_stability,
        vec![0.5],
    ));

    let verdict = build_verdict(&fields, total_records);

    ContextIntegrityReport {
        total_records,
        fields,
        verdict,
    }
}

fn audit_field(
    records: &[ExecutionResearchRecord],
    field_name: &str,
    extractor: fn(&ExecutionResearchRecord) -> f64,
    known_placeholders: Vec<f64>,
) -> FieldIntegrityReport {
    let values: Vec<f64> = records.iter().map(extractor).collect();
    let sample_count = values.len();

    let mut unique_set: HashMap<i64, usize> = HashMap::new();
    for v in &values {
        let key = (*v * 1_000_000.0).round() as i64;
        *unique_set.entry(key).or_insert(0) += 1;
    }
    let unique_values = unique_set.len();

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
        let mean = mean;
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64
    };

    let mut placeholder_detected = false;
    let mut placeholder_value: Option<f64> = None;

    for ph in &known_placeholders {
        if values.iter().all(|v| (v - ph).abs() < 1e-9) {
            placeholder_detected = true;
            placeholder_value = Some(*ph);
            break;
        }
    }

    let status = if placeholder_detected {
        FieldIntegrityStatus::Placeholder
    } else if unique_values == 1 {
        FieldIntegrityStatus::Constant
    } else if variance < 1e-6 {
        FieldIntegrityStatus::LowVariance
    } else {
        FieldIntegrityStatus::Pass
    };

    FieldIntegrityReport {
        field_name: field_name.to_string(),
        sample_count,
        unique_values,
        min,
        max,
        mean,
        variance,
        known_placeholders,
        placeholder_detected,
        placeholder_value,
        status,
    }
}

fn build_verdict(fields: &[FieldIntegrityReport], total_records: usize) -> String {
    let failed: Vec<&FieldIntegrityReport> = fields
        .iter()
        .filter(|f| f.status != FieldIntegrityStatus::Pass)
        .collect();

    if failed.is_empty() {
        return format!(
            "All {} ResearchContext-derived fields pass the Fact Integrity Gate. Total records: {}.",
            fields.len(),
            total_records
        );
    }

    let mut parts = Vec::new();
    parts.push(format!(
        "Fact Integrity Gate FAILED. {}/{} fields are flagged across {} records. Blocked fields:",
        failed.len(),
        fields.len(),
        total_records
    ));
    for f in &failed {
        parts.push(format!(
            "- {}: status={:?}, unique_values={}, min={:.2}, max={:.2}, placeholder={:?}",
            f.field_name, f.status, f.unique_values, f.min, f.max, f.placeholder_value
        ));
    }
    parts.push("Transition Evidence work must remain blocked until these fields are fixed.".to_string());
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

    fn make_record_with_breadth(breadth_pct: f64) -> ExecutionResearchRecord {
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
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    participation: ConfirmationDimension {
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    risk: ConfirmationDimension {
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    overall: "Moderate".into(),
                },
                breadth: BreadthSummary {
                    breadth_pct,
                    sma5: None,
                    delta_5d: None,
                    condition: "moderate".into(),
                },
                recovery: RecoverySummary {
                    score: 55.0,
                    drivers: vec![],
                },
                rotation_state: "mixed".into(),
                leadership_stability: 0.5,
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
            state: execution_engine::types::ExecutionState::Maintain,
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
    fn detects_placeholder_breadth() {
        let records = vec![
            make_record_with_breadth(50.0),
            make_record_with_breadth(50.0),
            make_record_with_breadth(50.0),
        ];
        let report = compute_context_integrity_report(&records);
        let breadth = report
            .fields
            .iter()
            .find(|f| f.field_name == "breadth.breadth_pct")
            .expect("breadth field");
        assert_eq!(breadth.status, FieldIntegrityStatus::Placeholder);
        assert_eq!(breadth.placeholder_value, Some(50.0));
    }

    #[test]
    fn detects_constant_field() {
        let records = vec![
            make_record_with_breadth(30.0),
            make_record_with_breadth(30.0),
            make_record_with_breadth(30.0),
        ];
        let report = compute_context_integrity_report(&records);
        let breadth = report
            .fields
            .iter()
            .find(|f| f.field_name == "breadth.breadth_pct")
            .expect("breadth field");
        assert_eq!(breadth.status, FieldIntegrityStatus::Constant);
    }

    #[test]
    fn passes_variable_field() {
        let records = vec![
            make_record_with_breadth(30.0),
            make_record_with_breadth(45.0),
            make_record_with_breadth(60.0),
        ];
        let report = compute_context_integrity_report(&records);
        let breadth = report
            .fields
            .iter()
            .find(|f| f.field_name == "breadth.breadth_pct")
            .expect("breadth field");
        assert_eq!(breadth.status, FieldIntegrityStatus::Pass);
    }
}

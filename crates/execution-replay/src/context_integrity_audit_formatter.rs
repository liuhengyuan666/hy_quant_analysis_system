use crate::context_integrity_audit::{ContextIntegrityReport, FieldIntegrityStatus};

/// Markdown / JSON formatter for `ContextIntegrityReport`.
pub struct ContextIntegrityAuditFormatter;

impl ContextIntegrityAuditFormatter {
    /// Renders the report as Markdown.
    pub fn markdown(report: &ContextIntegrityReport) -> String {
        let mut lines = Vec::new();

        lines.push("# ResearchContext Fact Integrity Audit".into());
        lines.push(String::new());
        lines.push(format!("**Total Records:** {}", report.total_records));
        lines.push(format!(
            "**Fields Audited:** {} | **Failed:** {}",
            report.fields.len(),
            report.fields.iter().filter(|f| f.status != FieldIntegrityStatus::Pass).count()
        ));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(report.verdict.clone());
        lines.push(String::new());

        lines.push("## Field Integrity Report".into());
        lines.push(String::new());
        lines.push("| Field | Status | Samples | Unique | Min | Max | Mean | Variance | Placeholder |".into());
        lines.push("|-------|--------|--------:|-------:|----:|----:|-----:|---------:|-------------|".into());
        for field in &report.fields {
            lines.push(format!(
                "| {} | {} | {} | {} | {:.2} | {:.2} | {:.2} | {:.4} | {} |",
                field.field_name,
                format_status(field.status),
                field.sample_count,
                field.unique_values,
                field.min,
                field.max,
                field.mean,
                field.variance,
                format_placeholder(field.placeholder_value)
            ));
        }
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the report as JSON.
    pub fn json(report: &ContextIntegrityReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_default()
    }
}

fn format_status(status: FieldIntegrityStatus) -> String {
    match status {
        FieldIntegrityStatus::Pass => "PASS".into(),
        FieldIntegrityStatus::Constant => "CONSTANT".into(),
        FieldIntegrityStatus::Placeholder => "PLACEHOLDER".into(),
        FieldIntegrityStatus::LowVariance => "LOW_VARIANCE".into(),
    }
}

fn format_placeholder(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.2}", v),
        None => "-".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_integrity_audit::{
        compute_context_integrity_report, FieldIntegrityReport, FieldIntegrityStatus,
    };
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

    fn make_record() -> crate::ExecutionResearchRecord {
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
                    breadth_pct: 50.0,
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
        crate::ExecutionResearchRecord {
            event,
            outcome: crate::ExecutionOutcome::default(),
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn markdown_contains_table() {
        let report = compute_context_integrity_report(&[make_record(), make_record()]);
        let text = ContextIntegrityAuditFormatter::markdown(&report);
        assert!(text.contains("Field Integrity Report"));
        assert!(text.contains("breadth.breadth_pct"));
        assert!(text.contains("PLACEHOLDER"));
    }

    #[test]
    fn json_round_trips() {
        let report = compute_context_integrity_report(&[make_record()]);
        let text = ContextIntegrityAuditFormatter::json(&report);
        assert!(text.contains("total_records"));
    }
}

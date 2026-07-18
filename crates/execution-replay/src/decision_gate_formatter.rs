use serde_json;

use crate::decision_gate::DecisionGateAnalysis;

/// Formatter for Decision Gate Analysis.
pub struct DecisionGateFormatter;

impl DecisionGateFormatter {
    /// Returns compact JSON.
    pub fn json(analysis: &DecisionGateAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_else(|_| "{}".into())
    }

    /// Returns a Markdown report.
    pub fn markdown(analysis: &DecisionGateAnalysis) -> String {
        let mut lines = Vec::new();
        lines.push("# Decision Gate Analysis".into());
        lines.push("".into());
        lines.push(format!("**Total Records:** {}", analysis.total_records));
        lines.push(format!("**Reduce Candidates:** {}", analysis.total_candidates));
        lines.push(format!("**Final Reduce:** {}", analysis.final_reduce));
        lines.push("".into());
        lines.push(format!("- Reduce threshold: {:.3}", analysis.reduce_threshold));
        lines.push(format!("- Confidence threshold: {:.3}", analysis.confidence_threshold));
        lines.push(format!("- Consensus threshold: {:.3}", analysis.consensus_threshold));
        lines.push("".into());
        lines.push("This analysis counts records where `assessment.dominant_direction < reduce_threshold`".into());
        lines.push("and identifies which DecisionEngine gate blocked them from becoming Reduce.".into());
        lines.push("".into());

        lines.push("## Funnel".into());
        lines.push("".into());
        lines.push(format!("```"));
        lines.push(format!("Bearish Assessment Candidates"));
        lines.push(format!("dominant_direction < {:.3}", analysis.reduce_threshold));
        lines.push(format!("{}\n", analysis.total_candidates));
        lines.push(format!("  |"));
        lines.push(format!("  +-- Risk Critical: {}", analysis.blocked_by_risk_critical));
        lines.push(format!("  |"));
        lines.push(format!("  +-- Risk High: {}", analysis.blocked_by_risk_high));
        lines.push(format!("  |"));
        lines.push(format!("  +-- Confidence too low: {}", analysis.blocked_by_confidence));
        lines.push(format!("  |"));
        lines.push(format!("  +-- Consensus too low: {}", analysis.blocked_by_consensus));
        lines.push(format!("  |"));
        lines.push(format!("  +-- Passed all gates: {}", analysis.passed_all_gates));
        lines.push(format!("  |"));
        lines.push(format!("  +-- Final Reduce: {}", analysis.final_reduce));
        lines.push(format!("```"));
        lines.push("".into());

        lines.push("## Summary Table".into());
        lines.push("".into());
        lines.push("| Gate | Count | % of Candidates |".into());
        lines.push("|------|------:|----------------:|".into());
        let total = analysis.total_candidates.max(1);
        lines.push(format!(
            "| Risk Critical | {} | {:.1}% |",
            analysis.blocked_by_risk_critical,
            analysis.blocked_by_risk_critical as f64 / total as f64 * 100.0
        ));
        lines.push(format!(
            "| Risk High | {} | {:.1}% |",
            analysis.blocked_by_risk_high,
            analysis.blocked_by_risk_high as f64 / total as f64 * 100.0
        ));
        lines.push(format!(
            "| Confidence too low | {} | {:.1}% |",
            analysis.blocked_by_confidence,
            analysis.blocked_by_confidence as f64 / total as f64 * 100.0
        ));
        lines.push(format!(
            "| Consensus too low | {} | {:.1}% |",
            analysis.blocked_by_consensus,
            analysis.blocked_by_consensus as f64 / total as f64 * 100.0
        ));
        lines.push(format!(
            "| Passed all gates | {} | {:.1}% |",
            analysis.passed_all_gates,
            analysis.passed_all_gates as f64 / total as f64 * 100.0
        ));
        lines.push(format!("| **Final Reduce** | {} | {:.1}% |", analysis.final_reduce, analysis.final_reduce as f64 / total as f64 * 100.0));
        lines.push("".into());

        lines.push("## Interpretation".into());
        lines.push("".into());
        if analysis.total_candidates == 0 {
            lines.push("No Reduce candidates found. This means no record's `dominant_direction` fell below the reduce threshold.".into());
        } else if analysis.blocked_by_risk_critical + analysis.blocked_by_risk_high > analysis.total_candidates / 2 {
            lines.push("The majority of bearish candidates are blocked by the **Risk** gate. The engine treats bearish market states as too risky to act, even for Reduce. This suggests a risk semantics issue: 'High Risk' currently means 'do nothing' rather than 'exit position'.".into());
        } else if analysis.blocked_by_confidence > analysis.blocked_by_consensus {
            lines.push("Most bearish candidates are blocked by the **Confidence** gate. The evidence is directionally bearish, but the engine is not confident enough to act. This suggests evidence confidence is too low or the threshold is too high.".into());
        } else if analysis.blocked_by_consensus > 0 {
            lines.push("Most bearish candidates are blocked by the **Consensus** gate. Evidence directions are not sufficiently aligned. This suggests too many conflicting/neutral evidences dilute the bearish consensus.".into());
        } else if analysis.passed_all_gates > 0 && analysis.final_reduce == 0 {
            lines.push(format!("{} candidate(s) passed all gates but still did not become Reduce. This is a DecisionEngine bug or an undocumented gate.", analysis.passed_all_gates));
        } else {
            lines.push("No single gate dominates. The bottleneck is likely a combination of confidence/consensus and risk semantics.".into());
        }
        lines.push("".into());

        lines.push("## Per-Record Detail (first 50)".into());
        lines.push("".into());
        lines.push("| # | Symbol | Date | Direction | Confidence | Consensus | Risk | State | Primary Gate | State |".into());
        lines.push("|---|--------|------|-----------|------------|-----------|------|-------|--------------|-------|".into());
        for (i, record) in analysis.records.iter().take(50).enumerate() {
            lines.push(format!(
                "| {} | {} | {} | {:.3} | {:.3} | {:.3} | {:?} | {:?} | {} | {:?} |",
                i + 1,
                record.symbol,
                record.date,
                record.dominant_direction,
                record.confidence,
                record.consensus,
                record.risk,
                record.decision_state,
                record.primary_blocking_gate,
                record.strategy_state
            ));
        }
        if analysis.records.len() > 50 {
            lines.push(format!("| ... | ... | ... | ... | ... | ... | ... | ... | ... | ... |"));
            lines.push(format!("| | | | | | | | | | ({} records total, see JSON for full list) |", analysis.records.len()));
        }
        lines.push("".into());

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_gate::{
        compute_decision_gate_analysis, DecisionGateAnalysis, GateFailureReason,
    };
    use chrono::Utc;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use execution_engine::v2::assessment::{ExecutionAssessment, RiskLevel};
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::request::{
        ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
    };
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn make_record(direction: f64) -> crate::ExecutionResearchRecord {
        let assessment = ExecutionAssessment {
            confidence: 0.8,
            consensus: 0.8,
            coverage: 0.75,
            risk: RiskLevel::Medium,
            dominant_direction: direction,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };
        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: execution_engine::ExecutionState::Reduce,
            confidence: 0.8,
            risk: RiskLevel::Medium,
            evidences: vec![],
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };
        let policy = ExecutionPolicy::default();
        let request = ExecutionRequest {
            symbol: "000001".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            signal: core_domain::SignalSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
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
                date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
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
                low: 9.5,
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
                    score: 50.0,
                    drivers: vec![],
                },
                rotation_state: "mixed".into(),
                leadership_stability: 0.5,
            },
            policy,
        };
        let event = ExecutionEvent::new(request, Default::default(), vec![], vec![], assessment, decision);
        crate::ExecutionResearchRecord {
            event,
            outcome: Default::default(),
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn markdown_contains_header() {
        let analysis = compute_decision_gate_analysis(&[]);
        let md = DecisionGateFormatter::markdown(&analysis);
        assert!(md.contains("Decision Gate Analysis"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = compute_decision_gate_analysis(&[make_record(-0.5)]);
        let json = DecisionGateFormatter::json(&analysis);
        let restored: DecisionGateAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_candidates, 1);
    }
}

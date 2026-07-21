use crate::transition_analysis::{
    BreadthDeteriorationBreakdown, FailureBreakdown, LeadershipDecayBreakdown,
    LeadershipDecayHorizonAnalysis, TransitionAnalysis, TransitionBreakdown,
};

/// Markdown / JSON formatter for `TransitionAnalysis`.
pub struct TransitionAnalysisFormatter;

impl TransitionAnalysisFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &TransitionAnalysis) -> String {
        let mut lines = Vec::new();

        lines.push(format!("# Transition Evidence Analysis: {}", analysis.candidate));
        lines.push(String::new());
        lines.push(format!(
            "**Total Records:** {} | **Samples:** {}",
            analysis.total_records, analysis.samples
        ));
        lines.push(format!(
            "**Baseline Negative T+20:** {:.1}% | **Baseline Negative T+60:** {:.1}%",
            analysis.baseline_negative_t20_rate * 100.0,
            analysis.baseline_negative_t60_rate * 100.0
        ));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(analysis.verdict.clone());
        lines.push(String::new());

        lines.push("## Signal Performance".into());
        lines.push(String::new());
        lines.push("| Metric | Value |".into());
        lines.push("|--------|------:|".into());
        lines.push(format!(
            "| Negative T+20 Rate | {:.1}% |",
            analysis.negative_t20_rate * 100.0
        ));
        lines.push(format!(
            "| Negative T+60 Rate | {:.1}% |",
            analysis.negative_t60_rate * 100.0
        ));
        lines.push(format!("| Precision T+20 | {:.1}% |", analysis.precision_t20 * 100.0));
        lines.push(format!("| Precision T+60 | {:.1}% |", analysis.precision_t60 * 100.0));
        lines.push(format!("| Lift T+20 | {:.2} |", analysis.lift_t20));
        lines.push(format!("| Lift T+60 | {:.2} |", analysis.lift_t60));
        lines.push(format!("| Average T+20 | {:.2}% |", analysis.avg_t20 * 100.0));
        lines.push(format!("| Average T+60 | {:.2}% |", analysis.avg_t60 * 100.0));
        lines.push(String::new());

        lines.push(format!("## {} Breakdown", analysis.candidate));
        lines.push(String::new());
        lines.push(format_breakdown(&analysis.breakdown));
        lines.push(String::new());

        lines.push("## ADR-101 Validation Gate".into());
        lines.push(String::new());
        lines.push(format_gate(analysis));
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &TransitionAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

fn format_breakdown(breakdown: &TransitionBreakdown) -> String {
    match breakdown {
        TransitionBreakdown::RecoveryFailure(b) => format_recovery_failure_breakdown(b),
        TransitionBreakdown::BreadthDeterioration(b) => format_breadth_deterioration_breakdown(b),
        TransitionBreakdown::LeadershipDecay(b) => format_leadership_decay_breakdown(b),
    }
}

fn format_recovery_failure_breakdown(breakdown: &FailureBreakdown) -> String {
    let mut lines = Vec::new();
    lines.push("| Combination | Count |".into());
    lines.push("|-------------|------:|".into());
    lines.push(format!("| Full Failure | {} |", breakdown.full_failure));
    lines.push(format!("| Price + Breadth | {} |", breakdown.price_breadth));
    lines.push(format!("| Price + Leadership | {} |", breakdown.price_leadership));
    lines.push(format!(
        "| Breadth + Leadership | {} |",
        breakdown.breadth_leadership
    ));
    lines.push(format!("| Price Only | {} |", breakdown.price_only));
    lines.push(format!("| Breadth Only | {} |", breakdown.breadth_only));
    lines.push(format!("| Leadership Only | {} |", breakdown.leadership_only));
    lines.join("\n")
}

fn format_breadth_deterioration_breakdown(breakdown: &BreadthDeteriorationBreakdown) -> String {
    let mut lines = Vec::new();
    lines.push("| Trigger | Count |".into());
    lines.push("|---------|------:|".into());
    lines.push(format!("| delta_5d < -15% only | {} |", breakdown.delta_5d_only));
    lines.push(format!("| delta_10d < -25% only | {} |", breakdown.delta_10d_only));
    lines.push(format!("| both | {} |", breakdown.both));
    lines.join("\n")
}

fn format_leadership_decay_breakdown(breakdown: &LeadershipDecayBreakdown) -> String {
    let mut lines = Vec::new();
    lines.push("| Trigger | Count |".into());
    lines.push("|---------|------:|".into());
    lines.push(format!("| delta_5d < -15% only | {} |", breakdown.delta_5d_only));
    lines.push(format!("| delta_10d < -25% only | {} |", breakdown.delta_10d_only));
    lines.push(format!("| both | {} |", breakdown.both));
    lines.join("\n")
}

fn format_gate(analysis: &TransitionAnalysis) -> String {
    let sample_ok = analysis.samples >= 30;
    let precision_ok = analysis.precision_t20 >= 0.50;
    let lift_ok = analysis.lift_t20 >= 1.2;

    let mut lines = Vec::new();
    lines.push(format!(
        "- Sample size >= 30: {} ({})",
        if sample_ok { "PASS" } else { "FAIL" },
        analysis.samples
    ));
    lines.push(format!(
        "- Precision T+20 >= 50%: {} ({:.1}%)",
        if precision_ok { "PASS" } else { "FAIL" },
        analysis.precision_t20 * 100.0
    ));
    lines.push(format!(
        "- Lift T+20 >= 1.2: {} ({:.2})",
        if lift_ok { "PASS" } else { "FAIL" },
        analysis.lift_t20
    ));

    if sample_ok && precision_ok && lift_ok {
        lines.push("\n**Overall: PASS** - candidate meets ADR-101 thresholds.".into());
    } else if analysis.samples == 0 {
        lines.push("\n**Overall: NO DATA** - no samples detected.".into());
    } else {
        lines.push("\n**Overall: FAIL** - candidate does not meet ADR-101 thresholds.".into());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transition_analysis::{
        FailureBreakdown, LeadershipDecayBreakdown, TransitionBreakdown, TransitionCandidate,
    };

    fn make_recovery_analysis() -> TransitionAnalysis {
        TransitionAnalysis {
            candidate: TransitionCandidate::RecoveryFailure,
            total_records: 1000,
            samples: 87,
            baseline_negative_t20_rate: 0.345,
            baseline_negative_t60_rate: 0.317,
            negative_t20_rate: 0.586,
            negative_t60_rate: 0.552,
            precision_t20: 0.586,
            precision_t60: 0.552,
            lift_t20: 1.70,
            lift_t60: 1.74,
            avg_t20: -0.02,
            avg_t60: -0.05,
            breakdown: TransitionBreakdown::RecoveryFailure(FailureBreakdown {
                price_only: 5,
                breadth_only: 8,
                leadership_only: 3,
                price_breadth: 35,
                price_leadership: 10,
                breadth_leadership: 12,
                full_failure: 14,
            }),
            verdict: "test".into(),
        }
    }

    fn make_breadth_analysis() -> TransitionAnalysis {
        TransitionAnalysis {
            candidate: TransitionCandidate::BreadthDeterioration,
            total_records: 1000,
            samples: 320,
            baseline_negative_t20_rate: 0.472,
            baseline_negative_t60_rate: 0.409,
            negative_t20_rate: 0.620,
            negative_t60_rate: 0.580,
            precision_t20: 0.620,
            precision_t60: 0.580,
            lift_t20: 1.31,
            lift_t60: 1.42,
            avg_t20: -0.03,
            avg_t60: -0.06,
            breakdown: TransitionBreakdown::BreadthDeterioration(BreadthDeteriorationBreakdown {
                delta_5d_only: 200,
                delta_10d_only: 120,
                both: 80,
            }),
            verdict: "test".into(),
        }
    }

    #[test]
    fn markdown_contains_recovery_breakdown() {
        let text = TransitionAnalysisFormatter::markdown(&make_recovery_analysis());
        assert!(text.contains("Signal Performance"));
        assert!(text.contains("RecoveryFailure Breakdown"));
        assert!(text.contains("Full Failure"));
    }

    #[test]
    fn markdown_contains_breadth_breakdown() {
        let text = TransitionAnalysisFormatter::markdown(&make_breadth_analysis());
        assert!(text.contains("Signal Performance"));
        assert!(text.contains("BreadthDeterioration Breakdown"));
        assert!(text.contains("delta_5d < -15% only"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = make_recovery_analysis();
        let text = TransitionAnalysisFormatter::json(&analysis);
        assert!(text.contains("samples"));
        assert!(text.contains("lift_t20"));
    }
}

/// Markdown / JSON formatter for `LeadershipDecayHorizonAnalysis`.
pub struct LeadershipDecayHorizonFormatter;

impl LeadershipDecayHorizonFormatter {
    /// Renders the horizon analysis as Markdown.
    pub fn markdown(analysis: &LeadershipDecayHorizonAnalysis) -> String {
        let mut lines = Vec::new();

        lines.push("# LeadershipDecay Horizon Analysis".into());
        lines.push(String::new());
        lines.push(format!(
            "**Total Records:** {} | **LeadershipDecay Samples:** {}",
            analysis.total_records, analysis.leadership_decay_samples
        ));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(analysis.verdict.clone());
        lines.push(String::new());

        lines.push("## Breakdown".into());
        lines.push(String::new());
        lines.push("| Trigger | Count |".into());
        lines.push("|---------|------:|".into());
        lines.push(format!(
            "| delta_5d < -15% only | {} |",
            analysis.breakdown.delta_5d_only
        ));
        lines.push(format!(
            "| delta_10d < -25% only | {} |",
            analysis.breakdown.delta_10d_only
        ));
        lines.push(format!("| both | {} |", analysis.breakdown.both));
        lines.push(String::new());

        lines.push("## Horizon Profile".into());
        lines.push(String::new());
        lines.push("| Horizon | Samples | Negative Rate | Baseline | Lift | Precision | Avg Return | Median Return | Avg Max DD |".into());
        lines.push("|---------|--------:|--------------:|---------:|-----:|----------:|-----------:|--------------:|-----------:|".into());
        for p in &analysis.profiles {
            lines.push(format!(
                "| {} | {} | {:.1}% | {:.1}% | {:.2} | {:.1}% | {:.2}% | {:.2}% | {:.2}% |",
                p.horizon_label,
                p.sample_count,
                p.signal_negative_rate * 100.0,
                p.baseline_negative_rate * 100.0,
                p.lift,
                p.precision * 100.0,
                p.avg_signal_return * 100.0,
                p.median_signal_return * 100.0,
                p.max_drawdown_mean_signal * 100.0
            ));
        }
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the horizon analysis as JSON.
    pub fn json(analysis: &LeadershipDecayHorizonAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod horizon_tests {
    use super::*;
    use crate::transition_analysis::{compute_leadership_decay_horizon_analysis, HorizonProfile};
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

    fn make_record(
        date: chrono::NaiveDate,
        leadership_stability: f64,
        t5_return: f64,
        t20_return: f64,
    ) -> crate::ExecutionResearchRecord {
        let policy = ExecutionPolicy::default();
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
                    trend: ConfirmationDimension { score: 50.0, label: "Moderate".into() },
                    participation: ConfirmationDimension { score: 50.0, label: "Moderate".into() },
                    risk: ConfirmationDimension { score: 50.0, label: "Moderate".into() },
                    overall: "Moderate".into(),
                },
                breadth: BreadthSummary { breadth_pct: 50.0, sma5: None, delta_5d: None, condition: "moderate".into() },
                recovery: RecoverySummary { score: 50.0, drivers: vec![] },
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
        crate::ExecutionResearchRecord {
            event,
            outcome: crate::ExecutionOutcome {
                t5_return: Some(t5_return),
                t20_return: Some(t20_return),
                t60_return: Some(t20_return * 1.5),
                t120_return: Some(t20_return * 2.0),
                mfe: None,
                mae: None,
                max_drawdown: None,
                holding_days: None,
                benchmark_return: None,
                alpha: None,
                stop_loss_hit: None,
                take_profit_hit: None,
            },
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn markdown_contains_horizon_table() {
        let d0 = chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let d1 = chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
        let d2 = chrono::NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let d3 = chrono::NaiveDate::from_ymd_opt(2026, 7, 4).unwrap();
        let d4 = chrono::NaiveDate::from_ymd_opt(2026, 7, 5).unwrap();
        let d5 = chrono::NaiveDate::from_ymd_opt(2026, 7, 6).unwrap();
        // leadership drops from 0.8 -> 0.6 over 5 days, all negative outcomes.
        let records = vec![
            make_record(d0, 0.80, -0.01, -0.05),
            make_record(d1, 0.78, -0.02, -0.06),
            make_record(d2, 0.75, -0.03, -0.07),
            make_record(d3, 0.70, -0.04, -0.08),
            make_record(d4, 0.65, -0.05, -0.09),
            make_record(d5, 0.60, -0.06, -0.10),
        ];
        let analysis = compute_leadership_decay_horizon_analysis(&records);
        let text = LeadershipDecayHorizonFormatter::markdown(&analysis);
        assert!(text.contains("LeadershipDecay Horizon Analysis"));
        assert!(text.contains("T+5"));
        assert!(text.contains("T+20"));
        assert!(text.contains("T+60"));
        assert!(text.contains("T+120"));
    }

    #[test]
    fn json_round_trips() {
        let d0 = chrono::NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let d1 = chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
        let records = vec![
            make_record(d0, 0.80, 0.01, 0.05),
            make_record(d1, 0.60, 0.02, 0.06),
        ];
        let analysis = compute_leadership_decay_horizon_analysis(&records);
        let text = LeadershipDecayHorizonFormatter::json(&analysis);
        assert!(text.contains("total_records"));
        assert!(text.contains("profiles"));
    }
}

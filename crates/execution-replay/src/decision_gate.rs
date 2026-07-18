use chrono::NaiveDate;
use execution_engine::v2::assessment::RiskLevel;
use execution_engine::v2::evidence::EvidenceKind;
use execution_engine::ExecutionState;
use serde::{Deserialize, Serialize};

use crate::ExecutionResearchRecord;

/// The reason a Reduce candidate was blocked by the DecisionEngine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum GateFailureReason {
    RiskCritical,
    RiskHigh,
    ConfidenceTooLow,
    ConsensusTooLow,
    PassedAllGates, // Candidate should have become Reduce but did not
}

impl std::fmt::Display for GateFailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateFailureReason::RiskCritical => write!(f, "RiskCritical"),
            GateFailureReason::RiskHigh => write!(f, "RiskHigh"),
            GateFailureReason::ConfidenceTooLow => write!(f, "ConfidenceTooLow"),
            GateFailureReason::ConsensusTooLow => write!(f, "ConsensusTooLow"),
            GateFailureReason::PassedAllGates => write!(f, "PassedAllGates"),
        }
    }
}

/// Per-record detail for a Reduce candidate that did not become Reduce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionGateRecord {
    pub execution_id: String,
    pub symbol: String,
    pub date: NaiveDate,
    pub dominant_direction: f64,
    pub confidence: f64,
    pub consensus: f64,
    pub coverage: f64,
    pub risk: RiskLevel,
    pub decision_state: ExecutionState,
    pub strategy_state: core_domain::StrategyState,
    pub strategy_state_score: f64,
    pub evidences: Vec<EvidenceKind>,
    /// The first gate in DecisionEngine order that blocked this candidate.
    pub primary_blocking_gate: GateFailureReason,
    /// All gates that were failed at the time of decision (may include gates after the first).
    pub all_blocking_gates: Vec<GateFailureReason>,
    pub passed_all_gates: bool,
}

/// Decision Gate Analysis: why bearish Assessments do not become Reduce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionGateAnalysis {
    pub total_records: usize,
    pub total_candidates: usize,
    pub blocked_by_risk_critical: usize,
    pub blocked_by_risk_high: usize,
    pub blocked_by_confidence: usize,
    pub blocked_by_consensus: usize,
    pub passed_all_gates: usize,
    pub final_reduce: usize,
    pub reduce_threshold: f64,
    pub confidence_threshold: f64,
    pub consensus_threshold: f64,
    pub records: Vec<DecisionGateRecord>,
}

/// Computes the Decision Gate Analysis for a set of records.
///
/// A "Reduce candidate" is a record whose `assessment.dominant_direction < policy.reduce_threshold`.
/// For each candidate, this function reproduces the first gate in the DecisionEngine that would
/// block it from becoming Reduce, matching the exact order used by the engine:
///
/// 1. Risk Critical
/// 2. Risk High
/// 3. Confidence below threshold
/// 4. Consensus below threshold
/// 5. Direction below reduce_threshold (passed)
///
/// Records that are not Reduce candidates are counted in `total_records` but not included in
/// `records` or gate counters.
pub fn compute_decision_gate_analysis(records: &[ExecutionResearchRecord]) -> DecisionGateAnalysis {
    let mut analysis = DecisionGateAnalysis {
        total_records: records.len(),
        total_candidates: 0,
        blocked_by_risk_critical: 0,
        blocked_by_risk_high: 0,
        blocked_by_confidence: 0,
        blocked_by_consensus: 0,
        passed_all_gates: 0,
        final_reduce: 0,
        reduce_threshold: 0.0,
        confidence_threshold: 0.0,
        consensus_threshold: 0.0,
        records: Vec::new(),
    };

    for record in records {
        let event = &record.event;
        let assessment = &event.decision.assessment;
        let policy = &event.policy;

        analysis.reduce_threshold = policy.reduce_threshold;
        analysis.confidence_threshold = policy.confidence_threshold;
        analysis.consensus_threshold = policy.consensus_threshold;

        if event.decision.state == ExecutionState::Reduce {
            analysis.final_reduce += 1;
        }

        if assessment.dominant_direction >= policy.reduce_threshold {
            continue;
        }

        analysis.total_candidates += 1;

        let mut gates = Vec::new();
        if assessment.risk == RiskLevel::Critical {
            gates.push(GateFailureReason::RiskCritical);
        }
        if assessment.risk == RiskLevel::High {
            gates.push(GateFailureReason::RiskHigh);
        }
        if assessment.confidence < policy.confidence_threshold {
            gates.push(GateFailureReason::ConfidenceTooLow);
        }
        if assessment.consensus < policy.consensus_threshold {
            gates.push(GateFailureReason::ConsensusTooLow);
        }

        let passed = gates.is_empty();
        let primary_gate = if passed {
            GateFailureReason::PassedAllGates
        } else {
            gates[0]
        };

        match primary_gate {
            GateFailureReason::RiskCritical => analysis.blocked_by_risk_critical += 1,
            GateFailureReason::RiskHigh => analysis.blocked_by_risk_high += 1,
            GateFailureReason::ConfidenceTooLow => analysis.blocked_by_confidence += 1,
            GateFailureReason::ConsensusTooLow => analysis.blocked_by_consensus += 1,
            GateFailureReason::PassedAllGates => analysis.passed_all_gates += 1,
        }

        let evidence_kinds: Vec<EvidenceKind> = event
            .evidences
            .iter()
            .map(|e| e.kind)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let detail = DecisionGateRecord {
            execution_id: event.execution_id.clone(),
            symbol: event.decision.symbol.clone(),
            date: event.request.date,
            dominant_direction: assessment.dominant_direction,
            confidence: assessment.confidence,
            consensus: assessment.consensus,
            coverage: assessment.coverage,
            risk: assessment.risk,
            decision_state: event.decision.state,
            strategy_state: event.request.strategy_state.state.clone(),
            strategy_state_score: event.request.strategy_state.state_score,
            evidences: evidence_kinds,
            primary_blocking_gate: primary_gate,
            all_blocking_gates: gates,
            passed_all_gates: passed,
        };
        analysis.records.push(detail);
    }

    analysis
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use execution_engine::v2::assessment::ExecutionAssessment;
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::request::{
        ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
    };
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn make_record(
        direction: f64,
        risk: RiskLevel,
        confidence: f64,
        consensus: f64,
        decision_state: ExecutionState,
    ) -> ExecutionResearchRecord {
        let assessment = ExecutionAssessment {
            confidence,
            consensus,
            coverage: 0.75,
            risk,
            dominant_direction: direction,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };
        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: decision_state,
            confidence,
            risk,
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
        let event = ExecutionEvent::new(
            request,
            Default::default(),
            vec![],
            vec![],
            assessment,
            decision,
        );
        ExecutionResearchRecord {
            event,
            outcome: Default::default(),
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn decision_gate_counts_by_reason() {
        let records = vec![
            make_record(-0.5, RiskLevel::Critical, 0.8, 0.8, ExecutionState::Wait), // risk critical
            make_record(-0.5, RiskLevel::High, 0.8, 0.8, ExecutionState::Wait),      // risk high
            make_record(-0.5, RiskLevel::Medium, 0.5, 0.8, ExecutionState::Wait),   // confidence low
            make_record(-0.5, RiskLevel::Medium, 0.8, 0.2, ExecutionState::Wait),  // consensus low
            make_record(-0.5, RiskLevel::Medium, 0.8, 0.8, ExecutionState::Reduce),  // passed
        ];
        let analysis = compute_decision_gate_analysis(&records);
        assert_eq!(analysis.total_candidates, 5);
        assert_eq!(analysis.blocked_by_risk_critical, 1);
        assert_eq!(analysis.blocked_by_risk_high, 1);
        assert_eq!(analysis.blocked_by_confidence, 1);
        assert_eq!(analysis.blocked_by_consensus, 1);
        assert_eq!(analysis.passed_all_gates, 1);
    }

    #[test]
    fn non_candidates_are_excluded() {
        let records = vec![
            make_record(0.5, RiskLevel::Medium, 0.8, 0.8, ExecutionState::BuyNow),
            make_record(-0.5, RiskLevel::Medium, 0.8, 0.8, ExecutionState::Reduce),
        ];
        let analysis = compute_decision_gate_analysis(&records);
        assert_eq!(analysis.total_records, 2);
        assert_eq!(analysis.total_candidates, 1);
        assert_eq!(analysis.passed_all_gates, 1);
    }
}

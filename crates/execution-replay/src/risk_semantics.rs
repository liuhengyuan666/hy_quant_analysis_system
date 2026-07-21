use chrono::NaiveDate;
use execution_engine::v2::assessment::RiskLevel;
use execution_engine::v2::evidence::EvidenceKind;
use execution_engine::ExecutionState;
use serde::{Deserialize, Serialize};

use crate::{ExecutionOutcome, ExecutionResearchRecord};

/// Percentile summary for a numeric distribution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NumericSummary {
    pub count: usize,
    pub mean: f64,
    pub min: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub max: f64,
}

impl NumericSummary {
    fn compute(values: &[f64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = sorted.len();
        let p25_idx = (len as f64 * 0.25) as usize;
        let p50_idx = (len as f64 * 0.50) as usize;
        let p75_idx = (len as f64 * 0.75) as usize;
        Some(Self {
            count: len,
            mean: values.iter().sum::<f64>() / len as f64,
            min: sorted[0],
            p25: sorted[p25_idx.clamp(0, len - 1)],
            p50: sorted[p50_idx.clamp(0, len - 1)],
            p75: sorted[p75_idx.clamp(0, len - 1)],
            max: sorted[len - 1],
        })
    }
}

/// Composition of a specific evidence kind within a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceComposition {
    pub evidence_kind: EvidenceKind,
    pub count: usize,
    pub pct_of_group: f64,
}

/// Risk distribution across all records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDistribution {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
    pub total: usize,
}

/// Outcome summary for a group of records.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutcomeSummary {
    pub count: usize,
    pub t20_mean: Option<f64>,
    pub t60_mean: Option<f64>,
    pub t120_mean: Option<f64>,
    pub mae_mean: Option<f64>,
    pub max_drawdown_mean: Option<f64>,
    pub negative_t20_ratio: Option<f64>,
    pub negative_t60_ratio: Option<f64>,
    pub negative_t120_ratio: Option<f64>,
}

impl OutcomeSummary {
    fn compute(records: &[&ExecutionResearchRecord]) -> Self {
        let t20s: Vec<f64> = records
            .iter()
            .filter_map(|r| r.outcome.t20_return)
            .collect();
        let t60s: Vec<f64> = records
            .iter()
            .filter_map(|r| r.outcome.t60_return)
            .collect();
        let t120s: Vec<f64> = records
            .iter()
            .filter_map(|r| r.outcome.t120_return)
            .collect();
        let maes: Vec<f64> = records
            .iter()
            .filter_map(|r| r.outcome.mae)
            .collect();
        let mdds: Vec<f64> = records
            .iter()
            .filter_map(|r| r.outcome.max_drawdown)
            .collect();

        fn mean(values: &[f64]) -> Option<f64> {
            if values.is_empty() {
                None
            } else {
                Some(values.iter().sum::<f64>() / values.len() as f64)
            }
        }

        fn negative_ratio(values: &[f64]) -> Option<f64> {
            if values.is_empty() {
                None
            } else {
                Some(values.iter().filter(|&&v| v < 0.0).count() as f64 / values.len() as f64)
            }
        }

        Self {
            count: records.len(),
            t20_mean: mean(&t20s),
            t60_mean: mean(&t60s),
            t120_mean: mean(&t120s),
            mae_mean: mean(&maes),
            max_drawdown_mean: mean(&mdds),
            negative_t20_ratio: negative_ratio(&t20s),
            negative_t60_ratio: negative_ratio(&t60s),
            negative_t120_ratio: negative_ratio(&t120s),
        }
    }
}

/// Decision context for a group of records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    pub count: usize,
    pub direction_summary: NumericSummary,
    pub confidence_summary: NumericSummary,
    pub consensus_summary: NumericSummary,
    pub coverage_summary: NumericSummary,
    pub decision_breakdown: std::collections::HashMap<ExecutionState, usize>,
}

/// Per-record detail for a RiskHigh blocked Reduce candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskHighCandidateRecord {
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
    pub evidences: Vec<EvidenceKind>,
    pub outcome: ExecutionOutcome,
}

/// Proposed risk semantic mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSemanticMapping {
    pub entry_risk: Vec<EvidenceKind>,
    pub holding_risk: Vec<EvidenceKind>,
    pub ambiguous: Vec<EvidenceKind>,
}

/// Risk Semantics Review output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSemanticsReview {
    pub total_records: usize,
    pub risk_distribution: RiskDistribution,
    pub high_risk_evidence_composition: Vec<EvidenceComposition>,
    pub high_risk_decision_context: DecisionContext,
    pub high_risk_outcome: OutcomeSummary,
    pub medium_risk_outcome: OutcomeSummary,
    pub low_risk_outcome: OutcomeSummary,
    pub high_risk_wait_outcome: OutcomeSummary,
    pub high_risk_reduce_outcome: Option<OutcomeSummary>,
    pub blocked_reduce_candidates_outcome: OutcomeSummary,
    pub blocked_reduce_candidates: Vec<RiskHighCandidateRecord>,
    pub semantic_mapping: RiskSemanticMapping,
    pub risk_threshold_low: f64,
    pub risk_threshold_high: f64,
}

/// Computes the Risk Semantics Review.
///
/// This review analyzes records grouped by RiskLevel, with a focus on `RiskLevel::High`.
/// It does not modify any engine logic; it only reports empirical composition and outcomes.
pub fn compute_risk_semantics_review(records: &[ExecutionResearchRecord]) -> RiskSemanticsReview {
    let risk_kinds = [
        EvidenceKind::Distribution,
        EvidenceKind::MomentumFailure,
        EvidenceKind::RiskExpansion,
        EvidenceKind::LiquidityConfirmation,
    ];

    let low_count = records
        .iter()
        .filter(|r| r.event.decision.assessment.risk == RiskLevel::Low)
        .count();
    let medium_count = records
        .iter()
        .filter(|r| r.event.decision.assessment.risk == RiskLevel::Medium)
        .count();
    let high_count = records
        .iter()
        .filter(|r| r.event.decision.assessment.risk == RiskLevel::High)
        .count();

    let high_risk_records: Vec<&ExecutionResearchRecord> = records
        .iter()
        .filter(|r| r.event.decision.assessment.risk == RiskLevel::High)
        .collect();

    // Evidence composition for High risk records.
    let mut evidence_counts: std::collections::HashMap<EvidenceKind, usize> = std::collections::HashMap::new();
    for record in &high_risk_records {
        for evidence in &record.event.evidences {
            *evidence_counts.entry(evidence.kind).or_insert(0) += 1;
        }
    }
    let total_high_risk = high_risk_records.len().max(1);
    let mut high_risk_evidence_composition: Vec<EvidenceComposition> = evidence_counts
        .into_iter()
        .map(|(kind, count)| EvidenceComposition {
            evidence_kind: kind,
            count,
            pct_of_group: count as f64 / total_high_risk as f64 * 100.0,
        })
        .collect();
    high_risk_evidence_composition.sort_by(|a, b| b.count.cmp(&a.count));

    // Decision context for High risk records.
    let direction_values: Vec<f64> = high_risk_records
        .iter()
        .map(|r| r.event.decision.assessment.dominant_direction)
        .collect();
    let confidence_values: Vec<f64> = high_risk_records
        .iter()
        .map(|r| r.event.decision.assessment.confidence)
        .collect();
    let consensus_values: Vec<f64> = high_risk_records
        .iter()
        .map(|r| r.event.decision.assessment.consensus)
        .collect();
    let coverage_values: Vec<f64> = high_risk_records
        .iter()
        .map(|r| r.event.decision.assessment.coverage)
        .collect();

    let mut decision_breakdown: std::collections::HashMap<ExecutionState, usize> = std::collections::HashMap::new();
    for record in &high_risk_records {
        *decision_breakdown.entry(record.event.decision.state).or_insert(0) += 1;
    }

    let high_risk_decision_context = DecisionContext {
        count: high_risk_records.len(),
        direction_summary: NumericSummary::compute(&direction_values).unwrap_or_default(),
        confidence_summary: NumericSummary::compute(&confidence_values).unwrap_or_default(),
        consensus_summary: NumericSummary::compute(&consensus_values).unwrap_or_default(),
        coverage_summary: NumericSummary::compute(&coverage_values).unwrap_or_default(),
        decision_breakdown,
    };

    // Outcomes by risk level.
    let low_risk_refs: Vec<&ExecutionResearchRecord> = records
        .iter()
        .filter(|r| r.event.decision.assessment.risk == RiskLevel::Low)
        .collect();
    let medium_risk_refs: Vec<&ExecutionResearchRecord> = records
        .iter()
        .filter(|r| r.event.decision.assessment.risk == RiskLevel::Medium)
        .collect();
    let high_risk_wait_refs: Vec<&ExecutionResearchRecord> = high_risk_records
        .iter()
        .filter(|r| r.event.decision.state == ExecutionState::Maintain)
        .copied()
        .collect();
    let high_risk_reduce_refs: Vec<&ExecutionResearchRecord> = high_risk_records
        .iter()
        .filter(|r| r.event.decision.state == ExecutionState::Reduce)
        .copied()
        .collect();

    let high_risk_reduce_outcome = if high_risk_reduce_refs.is_empty() {
        None
    } else {
        Some(OutcomeSummary::compute(&high_risk_reduce_refs))
    };

    // Blocked Reduce candidates: RiskHigh + bearish direction + Wait decision.
    let blocked_candidates_refs: Vec<&ExecutionResearchRecord> = high_risk_records
        .iter()
        .filter(|r| {
            r.event.decision.state == ExecutionState::Maintain
                && r.event.decision.assessment.dominant_direction < r.event.policy.reduce_threshold
        })
        .copied()
        .collect();

    let blocked_reduce_candidates: Vec<RiskHighCandidateRecord> = blocked_candidates_refs
        .iter()
        .map(|r| {
            let evidence_kinds: Vec<EvidenceKind> = r
                .event
                .evidences
                .iter()
                .map(|e| e.kind)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            RiskHighCandidateRecord {
                execution_id: r.event.execution_id.clone(),
                symbol: r.event.decision.symbol.clone(),
                date: r.event.request.date,
                dominant_direction: r.event.decision.assessment.dominant_direction,
                confidence: r.event.decision.assessment.confidence,
                consensus: r.event.decision.assessment.consensus,
                coverage: r.event.decision.assessment.coverage,
                risk: r.event.decision.assessment.risk,
                decision_state: r.event.decision.state,
                strategy_state: r.event.request.strategy_state.state.clone(),
                evidences: evidence_kinds,
                outcome: r.outcome.clone(),
            }
        })
        .collect();

    // Semantic mapping proposal.
    let semantic_mapping = RiskSemanticMapping {
        entry_risk: vec![
            EvidenceKind::MomentumExpansion,
            EvidenceKind::MarketAcceptance,
            EvidenceKind::LeadershipRotation,
        ],
        holding_risk: risk_kinds.to_vec(),
        ambiguous: vec![
            EvidenceKind::Breadth,
            EvidenceKind::Confirmation,
            EvidenceKind::Recovery,
            EvidenceKind::Stretch,
            EvidenceKind::SignalStrength,
            EvidenceKind::StrategyState,
            EvidenceKind::TrendParticipation,
            EvidenceKind::RiskCompression,
        ],
    };

    let policy = records
        .first()
        .map(|r| r.event.policy.clone())
        .unwrap_or_default();

    RiskSemanticsReview {
        total_records: records.len(),
        risk_distribution: RiskDistribution {
            low: low_count,
            medium: medium_count,
            high: high_count,
            total: records.len(),
        },
        high_risk_evidence_composition,
        high_risk_decision_context,
        high_risk_outcome: OutcomeSummary::compute(&high_risk_records),
        medium_risk_outcome: OutcomeSummary::compute(&medium_risk_refs),
        low_risk_outcome: OutcomeSummary::compute(&low_risk_refs),
        high_risk_wait_outcome: OutcomeSummary::compute(&high_risk_wait_refs),
        high_risk_reduce_outcome,
        blocked_reduce_candidates_outcome: OutcomeSummary::compute(&blocked_candidates_refs),
        blocked_reduce_candidates,
        semantic_mapping,
        risk_threshold_low: policy.risk_threshold_low,
        risk_threshold_high: policy.risk_threshold_high,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use execution_engine::v2::assessment::{ExecutionAssessment, RiskLevel};
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::evidence::{Evidence, EvidencePayload, EvidenceSource};
    use execution_engine::v2::request::{
        ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
    };
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn make_record(
        risk: RiskLevel,
        direction: f64,
        decision: ExecutionState,
        evidences: Vec<EvidenceKind>,
    ) -> ExecutionResearchRecord {
        let assessment = ExecutionAssessment {
            confidence: 0.7,
            consensus: 0.6,
            coverage: 0.75,
            risk,
            dominant_direction: direction,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };
        let decision_obj = ExecutionDecision {
            symbol: "000001".into(),
            state: decision,
            confidence: 0.7,
            risk,
            evidences: evidences
                .iter()
                .map(|kind| Evidence {
                    kind: *kind,
                    confidence: 0.8,
                    direction: 0.0,
                    source: EvidenceSource::IntradayObservation,
                    payload: EvidencePayload::Empty,
                })
                .collect(),
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
            evidences
                .iter()
                .map(|kind| Evidence {
                    kind: *kind,
                    confidence: 0.8,
                    direction: 0.0,
                    source: EvidenceSource::IntradayObservation,
                    payload: EvidencePayload::Empty,
                })
                .collect(),
            assessment,
            decision_obj,
        );
        ExecutionResearchRecord {
            event,
            outcome: ExecutionOutcome {
                t20_return: Some(-0.02),
                ..Default::default()
            },
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn risk_semantics_counts_high_risk_records() {
        let records = vec![
            make_record(RiskLevel::High, -0.5, ExecutionState::Maintain, vec![EvidenceKind::RiskExpansion]),
            make_record(RiskLevel::Low, 0.5, ExecutionState::Increase, vec![]),
        ];
        let review = compute_risk_semantics_review(&records);
        assert_eq!(review.risk_distribution.high, 1);
        assert_eq!(review.risk_distribution.low, 1);
        assert_eq!(review.blocked_reduce_candidates.len(), 1);
    }

    #[test]
    fn risk_semantics_composition_counts_risk_evidence() {
        let records = vec![
            make_record(
                RiskLevel::High,
                -0.5,
                ExecutionState::Maintain,
                vec![EvidenceKind::RiskExpansion, EvidenceKind::Distribution],
            ),
        ];
        let review = compute_risk_semantics_review(&records);
        let composition = review.high_risk_evidence_composition;
        assert_eq!(composition.len(), 2);
        assert!(composition.iter().any(|c| c.evidence_kind == EvidenceKind::RiskExpansion && c.count == 1));
    }
}

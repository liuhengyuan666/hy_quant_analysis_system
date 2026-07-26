use execution_engine::v2::evidence::EvidenceKind;
use execution_engine::ExecutionState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ExecutionResearchRecord;

/// A single bucket in the Decision Margin histogram.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectionBucket {
    pub bin_start: f64,
    pub bin_end: f64,
    pub total: usize,
    pub buy_now: usize,
    pub wait: usize,
    pub reduce: usize,
}

/// Per-EvidenceKind review of how Assessment.dominant_direction maps to Decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceDecisionProfile {
    pub evidence_kind: EvidenceKind,
    pub record_count: usize,
    /// Histogram of dominant_direction across records that have this evidence.
    pub direction_histogram: Vec<DirectionBucket>,
    /// Records where the decision matched the direction sign ( bullish → BuyNow, bearish → Reduce/Wait).
    pub buy_now_when_direction_positive: usize,
    pub reduce_when_direction_negative: usize,
    pub wait_when_direction_negative: usize,
    pub wait_when_direction_positive: usize,
    /// Records where direction was negative enough to cross reduce_threshold but decision was not Reduce.
    /// Threshold is the policy reduce_threshold from the first record that has this evidence.
    pub missed_reduce_count: usize,
    pub reduce_threshold: f64,
}

impl EvidenceDecisionProfile {
    pub fn reduce_recall(&self) -> f64 {
        let eligible = self.reduce_when_direction_negative + self.missed_reduce_count;
        if eligible == 0 {
            0.0
        } else {
            self.reduce_when_direction_negative as f64 / eligible as f64
        }
    }
}

/// Decision Margin Review output: how Assessment direction maps to final Decision,
/// with per-EvidenceKind breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionMarginReview {
    pub record_count: usize,
    pub profiles: Vec<EvidenceDecisionProfile>,
}

impl DecisionMarginReview {
    pub fn get(&self, kind: EvidenceKind) -> Option<&EvidenceDecisionProfile> {
        self.profiles.iter().find(|p| p.evidence_kind == kind)
    }
}

/// Computes the Decision Margin Review for a set of records.
///
/// For each record that contains a given EvidenceKind, bins the
/// `assessment.dominant_direction` and records what decision was made. This lets
/// us see whether decisions are concentrated near the policy thresholds (indicating
/// threshold is the bottleneck) or far from thresholds (indicating something else).
pub fn compute_decision_margin_review(records: &[ExecutionResearchRecord]) -> DecisionMarginReview {
    let bin_count = 20;
    let bin_width = 2.0 / bin_count as f64; // [-1.0, 1.0]

    let mut profiles: HashMap<EvidenceKind, EvidenceDecisionProfile> = HashMap::new();

    for record in records {
        let event = &record.event;
        let decision = &event.decision;
        let assessment = &decision.assessment;
        let direction = assessment.dominant_direction;

        let evidence_kinds: Vec<EvidenceKind> = event
            .evidences
            .iter()
            .map(|e| e.kind)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for kind in evidence_kinds {
            let profile = profiles.entry(kind).or_insert_with(|| EvidenceDecisionProfile {
                evidence_kind: kind,
                record_count: 0,
                direction_histogram: vec![DirectionBucket::default(); bin_count],
                buy_now_when_direction_positive: 0,
                reduce_when_direction_negative: 0,
                wait_when_direction_negative: 0,
                wait_when_direction_positive: 0,
                missed_reduce_count: 0,
                reduce_threshold: event.policy.reduce_threshold,
            });

            profile.record_count += 1;

            // Initialize bin boundaries once per profile if needed.
            for (i, bucket) in profile.direction_histogram.iter_mut().enumerate() {
                bucket.bin_start = -1.0 + i as f64 * bin_width;
                bucket.bin_end = bucket.bin_start + bin_width;
            }

            let bin_idx = ((direction + 1.0) / bin_width)
                .floor()
                .clamp(0.0, (bin_count - 1) as f64) as usize;
            let bucket = &mut profile.direction_histogram[bin_idx];
            bucket.total += 1;
            match decision.state {
                ExecutionState::Increase => bucket.buy_now += 1,
                ExecutionState::Maintain => bucket.wait += 1,
                ExecutionState::Reduce => bucket.reduce += 1,
                _ => {}
            }

            // Direction sign classification.
            if direction > 0.0 {
                if decision.state == ExecutionState::Increase {
                    profile.buy_now_when_direction_positive += 1;
                } else {
                    profile.wait_when_direction_positive += 1;
                }
            } else if direction < 0.0 {
                if decision.state == ExecutionState::Reduce {
                    profile.reduce_when_direction_negative += 1;
                } else if decision.state == ExecutionState::Maintain {
                    profile.wait_when_direction_negative += 1;
                }
            }

            // Missed Reduce: direction is below reduce_threshold but decision is not Reduce.
            if direction < event.policy.reduce_threshold && decision.state != ExecutionState::Reduce {
                profile.missed_reduce_count += 1;
            }
        }
    }

    let mut profiles_vec: Vec<_> = profiles.into_values().collect();
    profiles_vec.sort_by(|a, b| b.record_count.cmp(&a.record_count));

    DecisionMarginReview {
        record_count: records.len(),
        profiles: profiles_vec,
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

    fn make_record_with_risk(direction: f64, decision_state: ExecutionState) -> ExecutionResearchRecord {
        let evidences = vec![Evidence {
            kind: EvidenceKind::RiskExpansion,
            confidence: 0.8,
            direction: -1.0,
            source: EvidenceSource::IntradayObservation,
            payload: EvidencePayload::Empty,
        }];
        let assessment = ExecutionAssessment {
            confidence: 0.7,
            consensus: 0.6,
            coverage: 0.75,
            risk: RiskLevel::Medium,
            dominant_direction: direction,
            supporting_evidence: vec![],
            conflicting_evidence: evidences.clone(),
            neutral_evidence: vec![],
        };
        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: decision_state,
            confidence: 0.7,
            risk: RiskLevel::Medium,
            evidences: evidences.clone(),
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };
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
            policy: ExecutionPolicy::default(),
        };
        let event = ExecutionEvent::new(
            request,
            Default::default(),
            vec![],
            evidences,
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
    fn decision_margin_counts_missed_reduce() {
        let records = vec![
            make_record_with_risk(-0.5, ExecutionState::Maintain), // below threshold, but Wait
            make_record_with_risk(-0.5, ExecutionState::Reduce), // below threshold, Reduce
            make_record_with_risk(0.5, ExecutionState::Increase),
        ];
        let review = compute_decision_margin_review(&records);
        let profile = review.get(EvidenceKind::RiskExpansion).unwrap();
        assert_eq!(profile.record_count, 3);
        assert_eq!(profile.reduce_when_direction_negative, 1);
        assert_eq!(profile.missed_reduce_count, 1);
        assert_eq!(profile.reduce_recall(), 0.5);
    }
}

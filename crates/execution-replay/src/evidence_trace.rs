use chrono::{DateTime, NaiveDate, Utc};
use execution_engine::v2::evidence::EvidenceKind;
use execution_engine::v2::observation::ObservationKind;
use execution_engine::ExecutionState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ExecutionResearchRecord;

/// Maps an intraday ObservationKind to the EvidenceKind produced by the
/// DefaultEvidenceBuilder. This is the bridge between the Observation and
/// Evidence layers in the funnel.
fn observation_to_evidence_kind(obs: ObservationKind) -> EvidenceKind {
    match obs {
        ObservationKind::TrendPersistence => EvidenceKind::TrendParticipation,
        ObservationKind::CloseStrength => EvidenceKind::MarketAcceptance,
        ObservationKind::BuyingPressure => EvidenceKind::MomentumExpansion,
        ObservationKind::BreakoutAttempt => EvidenceKind::MomentumExpansion,
        ObservationKind::FailedBreakout => EvidenceKind::MomentumFailure,
        ObservationKind::Distribution => EvidenceKind::Distribution,
        ObservationKind::LiquidityDryUp => EvidenceKind::LiquidityConfirmation,
        ObservationKind::VolatilityExpansion => EvidenceKind::RiskExpansion,
    }
}

/// Per-EvidenceKind trace row: counts at each pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceTraceRow {
    pub evidence_kind: EvidenceKind,
    /// Count of observations that could have produced this evidence kind.
    /// Non-observation-derived evidences (e.g., StrategyState) will have 0.
    pub observation_count: usize,
    /// Count of evidence items of this kind produced by the EvidenceBuilder.
    pub evidence_count: usize,
    /// Count of records where this evidence appears in the supporting bucket.
    pub supporting_count: usize,
    /// Count of records where this evidence appears in the conflicting bucket.
    pub conflicting_count: usize,
    /// Count of records where this evidence appears in the neutral bucket.
    pub neutral_count: usize,
    /// Count of records with this evidence that resulted in each decision state.
    pub decision_counts: HashMap<ExecutionState, usize>,
}

impl EvidenceTraceRow {
    /// Number of records where this evidence reached the Assessment layer.
    pub fn in_assessment_count(&self) -> usize {
        self.supporting_count + self.conflicting_count + self.neutral_count
    }

    /// Retention from Observation layer to Evidence layer.
    pub fn observation_to_evidence_retention(&self) -> f64 {
        if self.observation_count == 0 {
            0.0
        } else {
            self.evidence_count as f64 / self.observation_count as f64
        }
    }

    /// Retention from Evidence layer to Assessment layer.
    pub fn evidence_to_assessment_retention(&self) -> f64 {
        if self.evidence_count == 0 {
            0.0
        } else {
            self.in_assessment_count() as f64 / self.evidence_count as f64
        }
    }

    /// Retention from Assessment layer to a specific decision state.
    pub fn assessment_to_decision_retention(&self, state: ExecutionState) -> f64 {
        let in_assessment = self.in_assessment_count();
        if in_assessment == 0 {
            0.0
        } else {
            self.decision_counts.get(&state).copied().unwrap_or(0) as f64 / in_assessment as f64
        }
    }

    /// Decision share: when this evidence exists, how often does the record
    /// result in the given decision state?
    pub fn decision_share(&self, state: ExecutionState) -> f64 {
        if self.evidence_count == 0 {
            0.0
        } else {
            self.decision_counts.get(&state).copied().unwrap_or(0) as f64 / self.evidence_count as f64
        }
    }
}

/// Metadata for an EvidenceTrace computation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceTraceMeta {
    pub record_count: usize,
    pub scope: Option<String>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub generated_at: DateTime<Utc>,
}

/// The complete per-stage funnel for every EvidenceKind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceTrace {
    pub meta: EvidenceTraceMeta,
    pub rows: Vec<EvidenceTraceRow>,
}

impl EvidenceTrace {
    pub fn get(&self, kind: EvidenceKind) -> Option<&EvidenceTraceRow> {
        self.rows.iter().find(|r| r.evidence_kind == kind)
    }
}

/// Computes a per-stage Evidence funnel across a set of records.
///
/// For each EvidenceKind, counts how many observations existed, how many
/// evidences were produced, how many reached Assessment (supporting/conflicting/
/// neutral), and how many records with this evidence resulted in each decision.
///
/// This lets us answer: "Where does Reduce evidence die?" without modifying any
/// pipeline stage.
pub fn compute_evidence_trace(records: &[ExecutionResearchRecord]) -> EvidenceTrace {
    let mut rows: HashMap<EvidenceKind, EvidenceTraceRow> = HashMap::new();

    for record in records {
        let event = &record.event;
        let decision = &event.decision;
        let assessment = &decision.assessment;

        // Count observations per kind.
        let mut observations_per_kind: HashMap<EvidenceKind, usize> = HashMap::new();
        for obs in &event.observations {
            let kind = observation_to_evidence_kind(obs.kind);
            *observations_per_kind.entry(kind).or_insert(0) += 1;
        }

        // Count evidences per kind and track which are present in this record.
        let mut evidence_per_kind: HashMap<EvidenceKind, usize> = HashMap::new();
        for e in &event.evidences {
            *evidence_per_kind.entry(e.kind).or_insert(0) += 1;
        }

        // Determine which evidence kinds are present in assessment buckets.
        let mut supporting_kinds: HashMap<EvidenceKind, usize> = HashMap::new();
        for e in &assessment.supporting_evidence {
            *supporting_kinds.entry(e.kind).or_insert(0) += 1;
        }
        let mut conflicting_kinds: HashMap<EvidenceKind, usize> = HashMap::new();
        for e in &assessment.conflicting_evidence {
            *conflicting_kinds.entry(e.kind).or_insert(0) += 1;
        }
        let mut neutral_kinds: HashMap<EvidenceKind, usize> = HashMap::new();
        for e in &assessment.neutral_evidence {
            *neutral_kinds.entry(e.kind).or_insert(0) += 1;
        }

        // Aggregate all evidence kinds touched in this record.
        let mut all_kinds: Vec<EvidenceKind> = evidence_per_kind.keys().copied().collect();
        for kind in observations_per_kind.keys().copied() {
            if !all_kinds.contains(&kind) {
                all_kinds.push(kind);
            }
        }

        for kind in all_kinds {
            let row = rows.entry(kind).or_insert_with(|| EvidenceTraceRow {
                evidence_kind: kind,
                observation_count: 0,
                evidence_count: 0,
                supporting_count: 0,
                conflicting_count: 0,
                neutral_count: 0,
                decision_counts: HashMap::new(),
            });

            row.observation_count += observations_per_kind.get(&kind).copied().unwrap_or(0);
            row.evidence_count += evidence_per_kind.get(&kind).copied().unwrap_or(0);
            row.supporting_count += supporting_kinds.get(&kind).copied().unwrap_or(0);
            row.conflicting_count += conflicting_kinds.get(&kind).copied().unwrap_or(0);
            row.neutral_count += neutral_kinds.get(&kind).copied().unwrap_or(0);

            // If the evidence kind is present anywhere in the record, count the decision.
            if evidence_per_kind.contains_key(&kind) || observations_per_kind.contains_key(&kind) {
                *row.decision_counts.entry(decision.state).or_insert(0) += 1;
            }
        }
    }

    let mut rows_vec: Vec<EvidenceTraceRow> = rows.into_values().collect();
    rows_vec.sort_by(|a, b| b.evidence_count.cmp(&a.evidence_count));

    EvidenceTrace {
        meta: EvidenceTraceMeta {
            record_count: records.len(),
            scope: None,
            from_date: None,
            to_date: None,
            generated_at: Utc::now(),
        },
        rows: rows_vec,
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
    use execution_engine::v2::observation::{IntradayObservation, ObservationPayload};
    use execution_engine::v2::request::{
        ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
    };
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn market_evidence(kind: EvidenceKind, direction: f64) -> Evidence {
        Evidence {
            kind,
            confidence: 0.7,
            direction,
            source: EvidenceSource::IntradayObservation,
            payload: EvidencePayload::Empty,
        }
    }

    fn risk_expansion_observation() -> IntradayObservation {
        IntradayObservation {
            kind: ObservationKind::VolatilityExpansion,
            confidence: 0.8,
            direction: -1.0,
            payload: ObservationPayload::VolatilityExpansion { amplitude_pct: 0.07 },
        }
    }

    fn distribution_observation() -> IntradayObservation {
        IntradayObservation {
            kind: ObservationKind::Distribution,
            confidence: 0.8,
            direction: -1.0,
            payload: ObservationPayload::Distribution {
                close_position: 0.1,
                volume_ratio: 2.0,
            },
        }
    }

    fn make_record(
        observations: Vec<IntradayObservation>,
        evidences: Vec<Evidence>,
        decision_state: ExecutionState,
    ) -> ExecutionResearchRecord {
        let assessment = ExecutionAssessment {
            confidence: 0.7,
            consensus: 0.6,
            coverage: 0.75,
            risk: RiskLevel::Medium,
            dominant_direction: 0.5,
            supporting_evidence: evidences.clone(),
            conflicting_evidence: vec![],
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
            observations,
            evidences.clone(),
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
    fn trace_counts_observation_to_evidence_mapping() {
        let records = vec![make_record(
            vec![risk_expansion_observation()],
            vec![market_evidence(EvidenceKind::RiskExpansion, -1.0)],
            ExecutionState::Maintain,
        )];
        let trace = compute_evidence_trace(&records);
        let row = trace.get(EvidenceKind::RiskExpansion).unwrap();
        assert_eq!(row.observation_count, 1);
        assert_eq!(row.evidence_count, 1);
        assert_eq!(row.supporting_count, 1);
        assert_eq!(row.decision_counts.get(&ExecutionState::Maintain), Some(&1));
    }

    #[test]
    fn trace_counts_multiple_risk_evidences() {
        let records = vec![
            make_record(
                vec![risk_expansion_observation(), distribution_observation()],
                vec![
                    market_evidence(EvidenceKind::RiskExpansion, -1.0),
                    market_evidence(EvidenceKind::Distribution, -1.0),
                ],
                ExecutionState::Maintain,
            ),
            make_record(
                vec![risk_expansion_observation()],
                vec![market_evidence(EvidenceKind::RiskExpansion, -1.0)],
                ExecutionState::Maintain,
            ),
        ];
        let trace = compute_evidence_trace(&records);
        let risk = trace.get(EvidenceKind::RiskExpansion).unwrap();
        assert_eq!(risk.observation_count, 2);
        assert_eq!(risk.evidence_count, 2);
        let dist = trace.get(EvidenceKind::Distribution).unwrap();
        assert_eq!(dist.observation_count, 1);
        assert_eq!(dist.evidence_count, 1);
    }

    #[test]
    fn trace_records_decision_for_evidence_presence() {
        let records = vec![make_record(
            vec![risk_expansion_observation()],
            vec![market_evidence(EvidenceKind::RiskExpansion, -1.0)],
            ExecutionState::Reduce,
        )];
        let trace = compute_evidence_trace(&records);
        let row = trace.get(EvidenceKind::RiskExpansion).unwrap();
        assert_eq!(row.decision_counts.get(&ExecutionState::Reduce), Some(&1));
        assert_eq!(row.decision_share(ExecutionState::Reduce), 1.0);
    }
}

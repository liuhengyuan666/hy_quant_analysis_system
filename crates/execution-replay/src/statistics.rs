use chrono::{DateTime, NaiveDate, Utc};
use execution_engine::v2::evidence::{EvidenceKind, EvidencePayload, EvidenceSource};
use execution_engine::ExecutionState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ExecutionEvaluation;
use crate::ExecutionResearchRecord;

/// Metadata describing the provenance of a statistics computation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStatisticsMeta {
    pub record_count: usize,
    pub scope: Option<String>,
    pub from_date: Option<NaiveDate>,
    pub to_date: Option<NaiveDate>,
    pub generated_at: DateTime<Utc>,
    pub execution_engine_version: String,
    pub policy_hash: Option<String>,
}

/// Frequency count of each EvidenceKind across all records.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceFrequency {
    pub counts: HashMap<EvidenceKind, usize>,
}

impl EvidenceFrequency {
    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }

    pub fn ratio(&self, kind: EvidenceKind) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.counts.get(&kind).copied().unwrap_or(0) as f64 / total as f64
        }
    }
}

/// Co-occurrence counts of EvidenceKind pairs within the same record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidencePairMatrix {
    pub pairs: HashMap<String, usize>,
}

impl EvidencePairMatrix {
    fn key(a: EvidenceKind, b: EvidenceKind) -> String {
        let mut pair = [a, b];
        pair.sort_by(|x, y| format!("{x:?}").cmp(&format!("{y:?}")));
        format!("{:?} + {:?}", pair[0], pair[1])
    }

    pub fn get(&self, a: EvidenceKind, b: EvidenceKind) -> usize {
        self.pairs.get(&Self::key(a, b)).copied().unwrap_or(0)
    }
}

/// Distribution of final ExecutionDecision states.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionDistribution {
    pub counts: HashMap<ExecutionState, usize>,
}

impl DecisionDistribution {
    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }

    pub fn ratio(&self, state: ExecutionState) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.counts.get(&state).copied().unwrap_or(0) as f64 / total as f64
        }
    }
}

/// Distribution of Prior Evidence labels (e.g. NoTrade, DeRisk, LeftProbe).
///
/// This intentionally does not count raw StrategyState; it counts the Prior
/// Evidence that the Execution Platform actually observed and used.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorDistribution {
    pub counts: HashMap<String, usize>,
}

impl PriorDistribution {
    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }

    pub fn ratio(&self, label: &str) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.counts.get(label).copied().unwrap_or(0) as f64 / total as f64
        }
    }
}

/// Histograms over the assessment dimensions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssessmentHistograms {
    pub confidence: Vec<usize>,
    pub consensus: Vec<usize>,
    pub coverage: Vec<usize>,
    pub risk: HashMap<String, usize>,
    pub bin_count: usize,
}

impl AssessmentHistograms {
    pub fn new(bin_count: usize) -> Self {
        Self {
            confidence: vec![0; bin_count],
            consensus: vec![0; bin_count],
            coverage: vec![0; bin_count],
            risk: HashMap::new(),
            bin_count,
        }
    }

    fn bin_index(value: f64, bin_count: usize) -> usize {
        let clamped = value.clamp(0.0, 1.0);
        ((clamped * (bin_count as f64 - 1.0)) + 0.5) as usize
    }

    pub fn add_confidence(&mut self, value: f64) {
        let idx = Self::bin_index(value, self.bin_count);
        self.confidence[idx] += 1;
    }

    pub fn add_consensus(&mut self, value: f64) {
        let idx = Self::bin_index(value, self.bin_count);
        self.consensus[idx] += 1;
    }

    pub fn add_coverage(&mut self, value: f64) {
        let idx = Self::bin_index(value, self.bin_count);
        self.coverage[idx] += 1;
    }

    pub fn add_risk(&mut self, label: &str) {
        *self.risk.entry(label.to_string()).or_insert(0) += 1;
    }
}

/// Bucketing of outcomes for the Decision × Outcome matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutcomeBucket {
    Hit,
    Miss,
    TooEarly,
    TooLate,
    Unknown,
}

/// Cross-tabulation of Decision state against Outcome bucket.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutcomeMatrix {
    pub rows: HashMap<ExecutionState, HashMap<OutcomeBucket, usize>>,
}

impl OutcomeMatrix {
    pub fn get(&self, state: ExecutionState, bucket: OutcomeBucket) -> usize {
        self.rows
            .get(&state)
            .and_then(|m| m.get(&bucket))
            .copied()
            .unwrap_or(0)
    }

    pub fn row_total(&self, state: ExecutionState) -> usize {
        self.rows
            .get(&state)
            .map(|m| m.values().sum())
            .unwrap_or(0)
    }
}

/// The complete empirical-fact output of the Execution Statistics layer.
///
/// This is the frozen Phase 2A-2 contract. New statistics require ADR review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    pub meta: ExecutionStatisticsMeta,
    pub evidence_frequency: EvidenceFrequency,
    pub evidence_pairs: EvidencePairMatrix,
    pub decision_distribution: DecisionDistribution,
    pub prior_distribution: PriorDistribution,
    pub assessment_histograms: AssessmentHistograms,
    pub outcome_matrix: OutcomeMatrix,
}

impl ExecutionStatistics {
    /// Returns true if the statistics were computed from an empty record set.
    pub fn is_empty(&self) -> bool {
        self.meta.record_count == 0
    }
}

/// Computes the frozen set of Execution Statistics from a collection of records.
///
/// The computation is deterministic: same records, same policy, same output.
pub fn compute_execution_statistics(records: &[ExecutionResearchRecord]) -> ExecutionStatistics {
    let mut evidence_frequency = EvidenceFrequency::default();
    let mut evidence_pairs = EvidencePairMatrix::default();
    let mut decision_distribution = DecisionDistribution::default();
    let mut prior_distribution = PriorDistribution::default();
    let mut assessment_histograms = AssessmentHistograms::new(10);
    let mut outcome_matrix = OutcomeMatrix::default();

    let mut policy_hash: Option<String> = None;
    let mut engine_version: Option<String> = None;

    for record in records {
        let event = &record.event;
        let decision = &event.decision;

        // Track provenance metadata from the first record.
        if policy_hash.is_none() {
            policy_hash = Some(event.policy_hash.clone());
        }
        if engine_version.is_none() {
            engine_version = Some(event.versions.engine_version.clone());
        }

        // Decision distribution.
        *decision_distribution.counts.entry(decision.state).or_insert(0) += 1;

        // Evidence frequency and pair matrix.
        let all_evidences: Vec<_> = event.evidences.clone();

        let kinds: Vec<EvidenceKind> = all_evidences.iter().map(|e| e.kind).collect();
        for kind in &kinds {
            *evidence_frequency.counts.entry(*kind).or_insert(0) += 1;
        }
        for i in 0..kinds.len() {
            for j in (i + 1)..kinds.len() {
                let key = EvidencePairMatrix::key(kinds[i], kinds[j]);
                *evidence_pairs.pairs.entry(key).or_insert(0) += 1;
            }
        }

        // Prior distribution: Evidence from StrategyState source.
        for e in &all_evidences {
            if e.source == EvidenceSource::StrategyState {
                if let EvidencePayload::StrategyState { state_label, .. } = &e.payload {
                    *prior_distribution
                        .counts
                        .entry(state_label.clone())
                        .or_insert(0) += 1;
                }
            }
        }

        // Assessment histograms.
        assessment_histograms.add_confidence(decision.assessment.confidence);
        assessment_histograms.add_consensus(decision.assessment.consensus);
        assessment_histograms.add_coverage(decision.assessment.coverage);
        assessment_histograms.add_risk(&format!("{:?}", decision.assessment.risk));

        // Outcome matrix.
        let bucket = evaluation_bucket(record.evaluation);
        outcome_matrix
            .rows
            .entry(decision.state)
            .or_default()
            .entry(bucket)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }

    ExecutionStatistics {
        meta: ExecutionStatisticsMeta {
            record_count: records.len(),
            scope: None,
            from_date: None,
            to_date: None,
            generated_at: Utc::now(),
            execution_engine_version: engine_version.unwrap_or_else(|| "unknown".into()),
            policy_hash,
        },
        evidence_frequency,
        evidence_pairs,
        decision_distribution,
        prior_distribution,
        assessment_histograms,
        outcome_matrix,
    }
}

fn evaluation_bucket(eval: ExecutionEvaluation) -> OutcomeBucket {
    use ExecutionEvaluation::*;
    match eval {
        AwaitingOutcome => OutcomeBucket::Unknown,
        Hit | TimingAcceptable | RiskWellManaged => OutcomeBucket::Hit,
        TooEarly => OutcomeBucket::TooEarly,
        TooLate => OutcomeBucket::TooLate,
        _ => OutcomeBucket::Miss,
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

    fn make_test_record(
        decision_state: ExecutionState,
        evaluation: ExecutionEvaluation,
        evidences: Vec<Evidence>,
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
            vec![],
            evidences,
            assessment,
            decision,
        );
        ExecutionResearchRecord {
            event,
            outcome: Default::default(),
            evaluation,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    fn prior_evidence(label: &str) -> Evidence {
        Evidence {
            kind: EvidenceKind::StrategyState,
            confidence: 0.8,
            direction: 0.0,
            source: EvidenceSource::StrategyState,
            payload: EvidencePayload::StrategyState {
                state_label: label.into(),
                recommended_position_pct: 0.0,
            },
        }
    }

    fn market_evidence(kind: EvidenceKind) -> Evidence {
        Evidence {
            kind,
            confidence: 0.7,
            direction: 1.0,
            source: EvidenceSource::IntradayObservation,
            payload: EvidencePayload::Empty,
        }
    }

    #[test]
    fn statistics_computes_decision_distribution() {
        let records = vec![
            make_test_record(ExecutionState::Increase, ExecutionEvaluation::Hit, vec![]),
            make_test_record(ExecutionState::Maintain, ExecutionEvaluation::PolicyTooConservative, vec![]),
            make_test_record(ExecutionState::Maintain, ExecutionEvaluation::PolicyTooConservative, vec![]),
        ];
        let stats = compute_execution_statistics(&records);
        assert_eq!(stats.decision_distribution.counts.get(&ExecutionState::Increase), Some(&1));
        assert_eq!(stats.decision_distribution.counts.get(&ExecutionState::Maintain), Some(&2));
        assert_eq!(stats.meta.record_count, 3);
    }

    #[test]
    fn statistics_computes_prior_distribution_from_strategy_state_evidence() {
        let records = vec![
            make_test_record(ExecutionState::Maintain, ExecutionEvaluation::PolicyTooConservative, vec![prior_evidence("NoTrade")]),
            make_test_record(ExecutionState::Maintain, ExecutionEvaluation::PolicyTooConservative, vec![prior_evidence("DeRisk")]),
            make_test_record(ExecutionState::Maintain, ExecutionEvaluation::PolicyTooConservative, vec![prior_evidence("NoTrade")]),
        ];
        let stats = compute_execution_statistics(&records);
        assert_eq!(stats.prior_distribution.counts.get("NoTrade"), Some(&2));
        assert_eq!(stats.prior_distribution.counts.get("DeRisk"), Some(&1));
    }

    #[test]
    fn statistics_computes_evidence_pair_matrix() {
        let records = vec![
            make_test_record(
                ExecutionState::Increase,
                ExecutionEvaluation::Hit,
                vec![market_evidence(EvidenceKind::TrendParticipation), market_evidence(EvidenceKind::Confirmation)],
            ),
            make_test_record(
                ExecutionState::Increase,
                ExecutionEvaluation::Hit,
                vec![market_evidence(EvidenceKind::TrendParticipation), market_evidence(EvidenceKind::MomentumExpansion)],
            ),
        ];
        let stats = compute_execution_statistics(&records);
        assert_eq!(stats.evidence_pairs.get(EvidenceKind::TrendParticipation, EvidenceKind::Confirmation), 1);
        assert_eq!(stats.evidence_pairs.get(EvidenceKind::TrendParticipation, EvidenceKind::MomentumExpansion), 1);
    }

    #[test]
    fn statistics_outcome_matrix_classifies_evaluations() {
        let records = vec![
            make_test_record(ExecutionState::Increase, ExecutionEvaluation::Hit, vec![]),
            make_test_record(ExecutionState::Increase, ExecutionEvaluation::TooEarly, vec![]),
            make_test_record(ExecutionState::Reduce, ExecutionEvaluation::TrendLost, vec![]),
            make_test_record(ExecutionState::Maintain, ExecutionEvaluation::AwaitingOutcome, vec![]),
        ];
        let stats = compute_execution_statistics(&records);
        assert_eq!(stats.outcome_matrix.get(ExecutionState::Increase, OutcomeBucket::Hit), 1);
        assert_eq!(stats.outcome_matrix.get(ExecutionState::Increase, OutcomeBucket::TooEarly), 1);
        assert_eq!(stats.outcome_matrix.get(ExecutionState::Reduce, OutcomeBucket::Miss), 1);
        assert_eq!(stats.outcome_matrix.get(ExecutionState::Maintain, OutcomeBucket::Unknown), 1);
    }

    #[test]
    fn statistics_is_empty_for_no_records() {
        let stats = compute_execution_statistics(&[]);
        assert!(stats.is_empty());
    }
}

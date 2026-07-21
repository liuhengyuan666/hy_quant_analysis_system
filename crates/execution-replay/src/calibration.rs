use chrono::NaiveDate;
use execution_engine::v2::assessment::ExecutionAssessment;
use execution_engine::v2::decision::{DecisionEngine, DefaultDecisionEngine, ExecutionDecision};
use execution_engine::v2::request::ExecutionPolicy;
use execution_engine::ExecutionState;
use serde::{Deserialize, Serialize};

use crate::ExecutionResearchRecord;

/// A confidence policy variant for a calibration experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalibrationPolicyKind {
    /// Single confidence threshold used for both bullish and bearish decisions.
    Uniform { confidence_threshold: f64 },
    /// Different confidence thresholds for buy-side and reduce-side decisions.
    Directional {
        buy_confidence_threshold: f64,
        reduce_confidence_threshold: f64,
    },
}

/// Policy configuration for a single calibration experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationPolicy {
    pub name: String,
    pub description: String,
    pub kind: CalibrationPolicyKind,
}

impl CalibrationPolicy {
    /// Builds an `ExecutionPolicy` that reflects this calibration variant.
    ///
    /// For `Uniform` this is straightforward: the confidence threshold is set
    /// to the given value. For `Directional` we use the *minimum* of the two
    /// thresholds as the uniform `confidence_threshold` because the current
    /// `DefaultDecisionEngine` only accepts a single value. The asymmetric
    /// behavior is then implemented by the custom `decide` method below.
    pub fn to_execution_policy(&self, base: &ExecutionPolicy) -> ExecutionPolicy {
        let mut policy = base.clone();
        match self.kind {
            CalibrationPolicyKind::Uniform { confidence_threshold } => {
                policy.confidence_threshold = confidence_threshold;
            }
            CalibrationPolicyKind::Directional {
                buy_confidence_threshold,
                reduce_confidence_threshold,
            } => {
                policy.confidence_threshold =
                    buy_confidence_threshold.min(reduce_confidence_threshold);
            }
        }
        policy
    }

    /// Decides the execution state for an assessment under this calibration policy.
    ///
    /// For `Uniform` we delegate to the `DefaultDecisionEngine` so the behavior
    /// is identical to the real engine. For `Directional` we apply the buy
    /// confidence threshold to bullish decisions and the reduce confidence
    /// threshold to bearish decisions.
    pub fn decide(&self, symbol: &str, assessment: &ExecutionAssessment, base: &ExecutionPolicy) -> ExecutionDecision {
        match self.kind {
            CalibrationPolicyKind::Uniform { .. } => {
                let policy = self.to_execution_policy(base);
                DefaultDecisionEngine.decide(symbol, assessment, &policy)
            }
            CalibrationPolicyKind::Directional {
                buy_confidence_threshold,
                reduce_confidence_threshold,
            } => {
                let (state, reasons) =
                    Self::directional_decide(assessment, buy_confidence_threshold, reduce_confidence_threshold, base);
                ExecutionDecision {
                    symbol: symbol.into(),
                    state,
                    confidence: assessment.confidence,
                    risk: assessment.risk,
                    evidences: assessment
                        .supporting_evidence
                        .iter()
                        .chain(assessment.conflicting_evidence.iter())
                        .chain(assessment.neutral_evidence.iter())
                        .cloned()
                        .collect(),
                    assessment: assessment.clone(),
                    decision_reasons: reasons,
                }
            }
        }
    }

    fn directional_decide(
        assessment: &ExecutionAssessment,
        buy_confidence_threshold: f64,
        reduce_confidence_threshold: f64,
        base: &ExecutionPolicy,
    ) -> (ExecutionState, Vec<execution_engine::v2::decision::DecisionReason>) {
        use execution_engine::v2::assessment::RiskLevel;
        use execution_engine::v2::decision::DecisionReason;

        if assessment.risk == RiskLevel::Critical {
            return (ExecutionState::Maintain, vec![DecisionReason::CriticalRisk]);
        }
        if assessment.risk == RiskLevel::High {
            return (ExecutionState::Maintain, vec![DecisionReason::RiskTooHigh]);
        }

        let direction = assessment.dominant_direction;
        let required_confidence = if direction > 0.0 {
            buy_confidence_threshold
        } else if direction < 0.0 {
            reduce_confidence_threshold
        } else {
            buy_confidence_threshold.min(reduce_confidence_threshold)
        };

        if assessment.confidence < required_confidence {
            return (ExecutionState::Maintain, vec![DecisionReason::ConfidenceBelowThreshold]);
        }
        if assessment.consensus < base.consensus_threshold {
            return (ExecutionState::Maintain, vec![DecisionReason::ConsensusBelowThreshold]);
        }
        if assessment.dominant_direction > base.buy_threshold {
            return (ExecutionState::Increase, vec![DecisionReason::PositiveConsensus]);
        }
        if assessment.dominant_direction < base.reduce_threshold {
            return (ExecutionState::Reduce, vec![DecisionReason::NegativeConsensus]);
        }
        (ExecutionState::Maintain, vec![DecisionReason::WeakDirection])
    }
}

/// A single calibration experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationExperiment {
    pub id: String,
    pub policy: CalibrationPolicy,
}

/// Per-record calibration decision detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationDecisionRecord {
    pub execution_id: String,
    pub symbol: String,
    pub date: NaiveDate,
    pub baseline_state: ExecutionState,
    pub experiment_state: ExecutionState,
    pub dominant_direction: f64,
    pub confidence: f64,
    pub consensus: f64,
    pub risk: execution_engine::v2::assessment::RiskLevel,
    pub is_reduce_candidate: bool,
    pub t20_return: Option<f64>,
    pub t60_return: Option<f64>,
    pub t120_return: Option<f64>,
}

/// Aggregate metrics for a calibration experiment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub experiment_id: String,
    pub experiment_name: String,
    pub description: String,
    pub total_records: usize,
    pub reduce_candidates: usize,
    pub reduce_count: usize,
    pub buy_now_count: usize,
    pub wait_count: usize,
    pub avoided_loss_count: usize,
    pub missed_recovery_count: usize,
    pub missed_reduce_count: usize,
    pub correct_wait_count: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    pub avg_t20_return_after_reduce: Option<f64>,
    pub avg_t60_return_after_reduce: Option<f64>,
    pub avg_t120_return_after_reduce: Option<f64>,
    pub avg_t20_return_for_reduce_candidates: Option<f64>,
    pub decisions: Vec<CalibrationDecisionRecord>,
}

/// Calibration review output: a collection of experiment results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationReview {
    pub total_records: usize,
    pub baseline_reduce_candidates: usize,
    pub baseline_reduce_count: usize,
    pub results: Vec<CalibrationResult>,
    pub recommendation: String,
}

/// Computes a calibration review by running each experiment on the same records.
///
/// This function does not modify any engine or policy defaults. It only
/// re-applies the decision logic to the existing assessments using alternative
/// policy configurations.
pub fn compute_calibration_review(
    records: &[ExecutionResearchRecord],
    experiments: &[CalibrationExperiment],
) -> CalibrationReview {
    let baseline_reduce_candidates = records
        .iter()
        .filter(|r| {
            r.event.decision.assessment.dominant_direction
                < r.event.policy.reduce_threshold
        })
        .count();
    let baseline_reduce_count = records
        .iter()
        .filter(|r| r.event.decision.state == ExecutionState::Reduce)
        .count();

    let mut results = Vec::new();
    for experiment in experiments {
        let result = run_experiment(records, experiment);
        results.push(result);
    }

    let recommendation = build_recommendation(&results);

    CalibrationReview {
        total_records: records.len(),
        baseline_reduce_candidates,
        baseline_reduce_count,
        results,
        recommendation,
    }
}

fn run_experiment(
    records: &[ExecutionResearchRecord],
    experiment: &CalibrationExperiment,
) -> CalibrationResult {
    let mut result = CalibrationResult {
        experiment_id: experiment.id.clone(),
        experiment_name: experiment.policy.name.clone(),
        description: experiment.policy.description.clone(),
        total_records: records.len(),
        ..Default::default()
    };

    let mut reduce_returns: Vec<f64> = Vec::new();
    let mut candidate_returns: Vec<f64> = Vec::new();
    let mut _true_negatives = 0usize; // Wait and T+20 >= 0 (for reduce candidates)
    let mut false_negatives = 0usize; // Wait and T+20 < 0 (for reduce candidates)
    let mut true_positives = 0usize; // Reduce made and T+20 < 0
    let mut false_positives = 0usize; // Reduce made and T+20 >= 0

    for record in records {
        let assessment = &record.event.decision.assessment;
        let base = &record.event.policy;
        let is_reduce_candidate = assessment.dominant_direction < base.reduce_threshold;
        let new_decision = experiment.policy.decide(&record.event.decision.symbol, assessment, base);
        let t20 = record.outcome.t20_return;

        let decision_record = CalibrationDecisionRecord {
            execution_id: record.event.execution_id.clone(),
            symbol: record.event.decision.symbol.clone(),
            date: record.event.request.date,
            baseline_state: record.event.decision.state,
            experiment_state: new_decision.state,
            dominant_direction: assessment.dominant_direction,
            confidence: assessment.confidence,
            consensus: assessment.consensus,
            risk: assessment.risk,
            is_reduce_candidate,
            t20_return: record.outcome.t20_return,
            t60_return: record.outcome.t60_return,
            t120_return: record.outcome.t120_return,
        };
        result.decisions.push(decision_record);

        if is_reduce_candidate {
            result.reduce_candidates += 1;
            if let Some(ret) = t20 {
                candidate_returns.push(ret);
            }
        }

        match new_decision.state {
            ExecutionState::Reduce => {
                result.reduce_count += 1;
                if let Some(ret) = t20 {
                    reduce_returns.push(ret);
                    if ret < 0.0 {
                        result.avoided_loss_count += 1;
                        true_positives += 1;
                    } else {
                        result.missed_recovery_count += 1;
                        false_positives += 1;
                    }
                }
            }
            ExecutionState::Increase => {
                result.buy_now_count += 1;
            }
            ExecutionState::Maintain => {
                result.wait_count += 1;
                if is_reduce_candidate {
                    if let Some(ret) = t20 {
                        if ret < 0.0 {
                            result.missed_reduce_count += 1;
                            false_negatives += 1;
                        } else {
                            result.correct_wait_count += 1;
                            _true_negatives += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    result.avg_t20_return_after_reduce = average(&reduce_returns);
    result.avg_t60_return_after_reduce = average(
        &result
            .decisions
            .iter()
            .filter(|d| d.experiment_state == ExecutionState::Reduce)
            .filter_map(|d| d.t60_return)
            .collect::<Vec<_>>(),
    );
    result.avg_t120_return_after_reduce = average(
        &result
            .decisions
            .iter()
            .filter(|d| d.experiment_state == ExecutionState::Reduce)
            .filter_map(|d| d.t120_return)
            .collect::<Vec<_>>(),
    );
    result.avg_t20_return_for_reduce_candidates = average(&candidate_returns);

    result.precision = if (true_positives + false_positives) > 0 {
        Some(true_positives as f64 / (true_positives + false_positives) as f64)
    } else {
        None
    };
    result.recall = if (true_positives + false_negatives) > 0 {
        Some(true_positives as f64 / (true_positives + false_negatives) as f64)
    } else {
        None
    };
    result.f1 = if let (Some(p), Some(r)) = (result.precision, result.recall) {
        if p + r > 0.0 {
            Some(2.0 * p * r / (p + r))
        } else {
            None
        }
    } else {
        None
    };

    result
}

fn average(values: &Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn build_recommendation(results: &[CalibrationResult]) -> String {
    if results.is_empty() {
        return "No experiments were run.".into();
    }

    // Prefer experiments with F1 score, then precision, then recall.
    let best = results
        .iter()
        .max_by(|a, b| {
            a.f1
                .partial_cmp(&b.f1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.precision
                        .partial_cmp(&b.precision)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
                .then(
                    a.recall
                        .partial_cmp(&b.recall)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        })
        .unwrap();

    format!(
        "Recommended experiment: {} (F1={:.2}, Precision={:.2}, Recall={:.2}, Reduce count={}/{}).",
        best.experiment_name,
        best.f1.unwrap_or(0.0),
        best.precision.unwrap_or(0.0),
        best.recall.unwrap_or(0.0),
        best.reduce_count,
        best.reduce_candidates
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExecutionOutcome;
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

    fn make_record(
        direction: f64,
        confidence: f64,
        t20_return: f64,
    ) -> ExecutionResearchRecord {
        let assessment = ExecutionAssessment {
            confidence,
            consensus: 0.6,
            coverage: 1.0,
            risk: RiskLevel::Medium,
            dominant_direction: direction,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };
        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: ExecutionState::Maintain,
            confidence,
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
        ExecutionResearchRecord {
            event,
            outcome: ExecutionOutcome {
                t20_return: Some(t20_return),
                ..Default::default()
            },
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn calibration_releases_reduce_when_confidence_lowered() {
        let records = vec![
            make_record(-0.5, 0.55, -0.05), // bearish, confidence below 0.6 but above 0.5, negative outcome
        ];
        let experiments = vec![
            CalibrationExperiment {
                id: "C2".into(),
                policy: CalibrationPolicy {
                    name: "Uniform 0.50".into(),
                    description: "Test".into(),
                    kind: CalibrationPolicyKind::Uniform {
                        confidence_threshold: 0.5,
                    },
                },
            },
        ];
        let review = compute_calibration_review(&records, &experiments);
        let result = &review.results[0];
        assert_eq!(result.reduce_count, 1);
        assert_eq!(result.avoided_loss_count, 1);
        assert_eq!(result.missed_recovery_count, 0);
    }

    #[test]
    fn directional_policy_allows_lower_reduce_confidence() {
        let records = vec![
            make_record(-0.5, 0.45, -0.05), // bearish, confidence 0.45, negative outcome
        ];
        let experiments = vec![
            CalibrationExperiment {
                id: "asymmetric".into(),
                policy: CalibrationPolicy {
                    name: "Asymmetric 0.60/0.40".into(),
                    description: "Test".into(),
                    kind: CalibrationPolicyKind::Directional {
                        buy_confidence_threshold: 0.6,
                        reduce_confidence_threshold: 0.4,
                    },
                },
            },
        ];
        let review = compute_calibration_review(&records, &experiments);
        let result = &review.results[0];
        assert_eq!(result.reduce_count, 1);
    }
}

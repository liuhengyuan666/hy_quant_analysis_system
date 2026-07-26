use serde::{Deserialize, Serialize};

use crate::ExecutionResearchRecord;

/// A numeric percentile summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PercentileSummary {
    pub count: usize,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
}

impl PercentileSummary {
    fn compute(values: &[f64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let mut sorted: Vec<f64> = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let len = sorted.len();
        Some(Self {
            count: len,
            p10: percentile(&sorted, 0.10),
            p25: percentile(&sorted, 0.25),
            p50: percentile(&sorted, 0.50),
            p75: percentile(&sorted, 0.75),
            p90: percentile(&sorted, 0.90),
            p95: percentile(&sorted, 0.95),
            min: sorted[0],
            max: sorted[len - 1],
            mean: values.iter().sum::<f64>() / len as f64,
        })
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let len = sorted.len();
    if len == 0 {
        return 0.0;
    }
    if len == 1 {
        return sorted[0];
    }
    let idx = (p * (len - 1) as f64).floor() as usize;
    let frac = p * (len - 1) as f64 - idx as f64;
    let lower = sorted[idx.clamp(0, len - 1)];
    let upper = sorted[(idx + 1).clamp(0, len - 1)];
    lower + frac * (upper - lower)
}

/// Coverage of the Distribution observation condition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DistributionConditionCoverage {
    pub total_records: usize,
    pub records_with_negative_return: usize,
    pub records_with_negative_return_and_low_close: usize,
    pub records_with_negative_return_and_high_volume: usize,
    pub records_satisfying_all_conditions: usize,
    pub records_with_distribution_observation: usize,
    pub coverage_pct: f64,
}

/// Distribution Coverage Review output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionCoverageReview {
    pub record_count: usize,
    pub close_position: PercentileSummary,
    pub volume_ratio: PercentileSummary,
    pub today_return: PercentileSummary,
    pub condition_coverage: DistributionConditionCoverage,
}

/// Computes the Distribution Coverage Review.
///
/// Analyzes the intraday features of all records to understand whether the
/// current Distribution observation condition (`close_position < 0.2 &&
/// volume_ratio > 1.5 && today_return < 0.0`) is too strict or too loose.
///
/// This function does NOT modify the ObservationEngine; it only reports.
pub fn compute_distribution_coverage_review(
    records: &[ExecutionResearchRecord],
) -> DistributionCoverageReview {
    let close_positions: Vec<f64> = records
        .iter()
        .map(|r| r.event.features.close_position)
        .collect();
    let volume_ratios: Vec<f64> = records
        .iter()
        .map(|r| r.event.features.volume_ratio)
        .collect();
    let today_returns: Vec<f64> = records
        .iter()
        .map(|r| r.event.features.today_return)
        .collect();

    let mut coverage = DistributionConditionCoverage::default();
    coverage.total_records = records.len();

    let distribution_observations: usize = records
        .iter()
        .map(|r| {
            r.event
                .observations
                .iter()
                .filter(|o| matches!(o.kind, execution_engine::v2::observation::ObservationKind::Distribution))
                .count()
        })
        .sum();

    for record in records {
        let f = &record.event.features;
        let negative_return = f.today_return < 0.0;
        let low_close = f.close_position < 0.2;
        let high_volume = f.volume_ratio > 1.5;

        if negative_return {
            coverage.records_with_negative_return += 1;
        }
        if negative_return && low_close {
            coverage.records_with_negative_return_and_low_close += 1;
        }
        if negative_return && high_volume {
            coverage.records_with_negative_return_and_high_volume += 1;
        }
        if negative_return && low_close && high_volume {
            coverage.records_satisfying_all_conditions += 1;
        }
    }
    coverage.records_with_distribution_observation = distribution_observations;
    coverage.coverage_pct = if coverage.records_satisfying_all_conditions == 0 {
        0.0
    } else {
        coverage.records_with_distribution_observation as f64
            / coverage.records_satisfying_all_conditions as f64
    };

    DistributionCoverageReview {
        record_count: records.len(),
        close_position: PercentileSummary::compute(&close_positions).unwrap_or_default(),
        volume_ratio: PercentileSummary::compute(&volume_ratios).unwrap_or_default(),
        today_return: PercentileSummary::compute(&today_returns).unwrap_or_default(),
        condition_coverage: coverage,
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
    use execution_engine::v2::feature::IntradayFeatures;
    use execution_engine::v2::request::{
        ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
    };
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn make_record_with_features(features: IntradayFeatures) -> ExecutionResearchRecord {
        let assessment = ExecutionAssessment {
            confidence: 0.7,
            consensus: 0.6,
            coverage: 0.75,
            risk: RiskLevel::Medium,
            dominant_direction: 0.0,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };
        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: execution_engine::ExecutionState::Maintain,
            confidence: 0.7,
            risk: RiskLevel::Medium,
            evidences: vec![],
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
        let event = ExecutionEvent::new(request, features, vec![], vec![], assessment, decision);
        ExecutionResearchRecord {
            event,
            outcome: Default::default(),
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn distribution_coverage_counts_conditions() {
        let records = vec![
            make_record_with_features(IntradayFeatures {
                symbol: "000001".into(),
                today_return: -0.03,
                open_return: 0.0,
                gap_pct: 0.0,
                close_position: 0.1,
                amplitude_pct: 0.04,
                upper_shadow_pct: 0.0,
                lower_shadow_pct: 0.0,
                volume_ratio: 2.0,
                body_ratio: 0.2,
                gap_fill_ratio: 0.0,
            }),
            make_record_with_features(IntradayFeatures {
                symbol: "000001".into(),
                today_return: 0.01,
                open_return: 0.0,
                gap_pct: 0.0,
                close_position: 0.8,
                amplitude_pct: 0.02,
                upper_shadow_pct: 0.0,
                lower_shadow_pct: 0.0,
                volume_ratio: 1.0,
                body_ratio: 0.8,
                gap_fill_ratio: 0.0,
            }),
        ];
        let review = compute_distribution_coverage_review(&records);
        assert_eq!(review.condition_coverage.total_records, 2);
        assert_eq!(review.condition_coverage.records_with_negative_return, 1);
        assert_eq!(review.condition_coverage.records_satisfying_all_conditions, 1);
    }
}

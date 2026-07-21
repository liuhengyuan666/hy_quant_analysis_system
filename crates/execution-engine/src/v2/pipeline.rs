use crate::v2::assessment::{AssessmentEngine, EqualWeightAssessmentEngine};
use crate::v2::decision::{DecisionEngine, DefaultDecisionEngine};
use crate::v2::event::ExecutionEvent;
use crate::v2::evidence::{DefaultEvidenceBuilder, EvidenceBuilder};
use crate::v2::feature::{DefaultFeatureExtractor, FeatureExtractor, FeatureExtractorInputs};
use crate::v2::observation::{DefaultObservationEngine, ObservationEngine};
use crate::v2::request::ExecutionRequest;

/// Orchestrates the full Execution Pipeline and produces an `ExecutionEvent`.
///
/// The pipeline is the only place where all stages are wired together. Each
/// stage is implemented via a trait, so individual algorithms can be swapped
/// without changing the pipeline shape.
pub trait ExecutionPipeline {
    fn execute(&self, request: ExecutionRequest) -> ExecutionEvent;
}

/// Default end-to-end pipeline using the MVP engines.
#[derive(Debug, Clone, Default)]
pub struct DefaultExecutionPipeline;

impl ExecutionPipeline for DefaultExecutionPipeline {
    fn execute(&self, request: ExecutionRequest) -> ExecutionEvent {
        let feature_extractor = DefaultFeatureExtractor;
        let observation_engine = DefaultObservationEngine;
        let evidence_builder = DefaultEvidenceBuilder;
        let assessment_engine = EqualWeightAssessmentEngine;
        let decision_engine = DefaultDecisionEngine;

        let features = feature_extractor.extract(&FeatureExtractorInputs {
            quote: request.quote.clone(),
            volume_ma20: request.volume_ma20,
        });

        let observations = observation_engine.observe(&features);

        let evidences = evidence_builder.build(
            &observations,
            &request.market_view,
            &request.signal,
            &request.strategy_state,
        );

        let assessment = assessment_engine.assess(&evidences, &request.policy);

        let decision = decision_engine.decide(&request.symbol, &assessment, &request.policy);

        ExecutionEvent::new(
            request,
            features,
            observations,
            evidences,
            assessment,
            decision,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_domain::{SignalLabel, StrategyState, StrategyKind};
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    use crate::v2::request::{ExecutionMarketView, ExecutionPolicy, QuoteSnapshot};

    fn make_request() -> ExecutionRequest {
        ExecutionRequest {
            symbol: "000001".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            signal: core_domain::SignalSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                symbol: "000001".into(),
                final_score: 85.0,
                signal_label: SignalLabel::StrongBuy,
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
                    final_score: 85.0,
                    label: SignalLabel::StrongBuy,
                    summary: "test".into(),
                },
            },
            strategy_state: core_domain::StrategyStateSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                scope: "CN".into(),
                state: StrategyState::FullTrend,
                state_score: 75.0,
                transition_reason: "test".into(),
                recommended_position_pct: 100.0,
            },
            quote: QuoteSnapshot {
                symbol: "000001".into(),
                ts: Utc::now(),
                open: 10.0,
                high: 11.0,
                low: 9.5,
                close: 10.8,
                volume: 1_500_000.0,
                prev_close: 10.0,
            },
            volume_ma20: 500_000.0,
            market_view: ExecutionMarketView {
                research_version: "1".into(),
                market_regime_label: "Bullish".into(),
                confirmation: ConfirmationSummary {
                    trend: ConfirmationDimension {
                        score: 70.0,
                        label: "Strong".into(),
                    },
                    participation: ConfirmationDimension {
                        score: 60.0,
                        label: "Moderate".into(),
                    },
                    risk: ConfirmationDimension {
                        score: 35.0,
                        label: "Low".into(),
                    },
                    overall: "Strong".into(),
                },
                breadth: BreadthSummary {
                    breadth_pct: 60.0,
                    sma5: None,
                    delta_5d: Some(0.0),
                    condition: "strong".into(),
                },
                recovery: RecoverySummary {
                    score: 60.0,
                    drivers: vec![],
                },
                rotation_state: "broad".into(),
                leadership_stability: 0.7,
            },
            policy: ExecutionPolicy::default(),
        }
    }

    #[test]
    fn full_pipeline_produces_event() {
        let request = make_request();
        let event = DefaultExecutionPipeline.execute(request);

        assert!(!event.execution_id.is_empty());
        assert!(!event.observations.is_empty());
        assert!(!event.evidences.is_empty());
        assert!(event.decision.confidence > 0.0);
    }

    #[test]
    fn full_pipeline_event_has_symbol() {
        let request = make_request();
        let event = DefaultExecutionPipeline.execute(request);

        assert_eq!(event.symbol(), "000001");
    }
}

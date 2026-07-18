use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use execution_engine::v2::event::ExecutionEvent;

pub mod decision_gate;
pub mod decision_gate_formatter;
pub mod decision_margin;
pub mod decision_margin_formatter;
pub mod distribution_coverage;
pub mod distribution_coverage_formatter;
pub mod evaluation;
pub mod evidence_trace;
pub mod evidence_trace_formatter;
pub mod formatter;
pub mod outcome;
pub mod risk_semantics;
pub mod risk_semantics_formatter;
pub mod runner;
pub mod statistics;
pub mod statistics_formatter;
pub mod validation_suite;

pub use decision_margin::{
    compute_decision_margin_review, DecisionMarginReview, DirectionBucket, EvidenceDecisionProfile,
};
pub use distribution_coverage::{
    compute_distribution_coverage_review, DistributionConditionCoverage, DistributionCoverageReview,
    PercentileSummary,
};
pub use decision_gate::{
    compute_decision_gate_analysis, DecisionGateAnalysis, DecisionGateRecord, GateFailureReason,
};
pub use decision_gate_formatter::DecisionGateFormatter;
pub use risk_semantics::{
    compute_risk_semantics_review, RiskSemanticsReview, RiskSemanticMapping,
};
pub use risk_semantics_formatter::RiskSemanticsFormatter;
pub use decision_margin_formatter::DecisionMarginFormatter;
pub use distribution_coverage_formatter::DistributionCoverageFormatter;
pub use evaluation::RuleBasedEvaluationEngine;
pub use evidence_trace::{compute_evidence_trace, EvidenceTrace, EvidenceTraceMeta, EvidenceTraceRow};
pub use evidence_trace_formatter::EvidenceTraceFormatter;
pub use formatter::ValidationFormatter;
pub use outcome::MarketStoreOutcomeResolver;
pub use runner::{
    MockOutcomeResolver, ValidationReportFormatter, ValidationResult, ValidationRunner,
    ValidationSummary,
};
pub use statistics::{
    compute_execution_statistics, AssessmentHistograms, DecisionDistribution,
    EvidenceFrequency, EvidencePairMatrix, ExecutionStatistics, ExecutionStatisticsMeta,
    OutcomeBucket, OutcomeMatrix, PriorDistribution,
};
pub use statistics_formatter::ExecutionStatisticsFormatter;
pub use validation_suite::{ValidationCandidate, ValidationCase, ValidationSuite};

/// Convenience helper to resolve and evaluate a single `ExecutionEvent`.
///
/// This is the canonical Research Layer entry point: it combines the objective
/// outcome from `ReplayOutcomeResolver` with the deterministic label from
/// `EvaluationEngine`, packaging them into an `ExecutionResearchRecord` ready
/// for the Research Asset workspace.
pub fn replay_single<R, E>(
    resolver: &R,
    evaluator: &E,
    event: &ExecutionEvent,
    as_of: NaiveDate,
) -> anyhow::Result<ExecutionResearchRecord>
where
    R: ReplayOutcomeResolver,
    E: EvaluationEngine,
{
    let outcome = resolver.resolve(event, as_of)?;
    let evaluation = evaluator.evaluate(event, &outcome);
    Ok(ExecutionResearchRecord {
        event: event.clone(),
        outcome,
        evaluation,
        evaluation_version: "v1.0.0-rule-based".into(),
        evaluated_at: Utc::now(),
    })
}
/// The objective, forward-looking outcome of a single execution decision.
///
/// `ExecutionOutcome` contains only computable facts: returns, MFE, MAE, drawdown,
/// holding period, benchmark comparison, and stop/take-profit triggers. It must not
/// contain any judgment about whether the decision was good or bad.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub t20_return: Option<f64>,
    pub t60_return: Option<f64>,
    pub t120_return: Option<f64>,
    pub mfe: Option<f64>,
    pub mae: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub holding_days: Option<u32>,
    pub benchmark_return: Option<f64>,
    pub alpha: Option<f64>,
    pub stop_loss_hit: Option<bool>,
    pub take_profit_hit: Option<bool>,
}

/// Research label for a single execution decision.
///
/// `ExecutionEvaluation` answers: "Why did this decision succeed or fail?" It is
/// the supervision signal for the Research Asset and, eventually, Policy Calibration.
/// The taxonomy is intentionally closed at runtime; new labels require ADR review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum ExecutionEvaluation {
    /// Outcome is not yet known.
    AwaitingOutcome,

    /// ---------- Successful outcomes ----------
    /// Decision direction was correct and timing was acceptable.
    Hit,
    /// Decision direction was correct; timing was not perfect but acceptable.
    TimingAcceptable,
    /// Risk was well-managed relative to the realized return.
    RiskWellManaged,

    /// ---------- Timing failures ----------
    /// Decision direction was eventually correct, but entry was too early.
    TooEarly,
    /// Decision direction was eventually correct, but entry was too late.
    TooLate,

    /// ---------- Direction failures ----------
    /// The trend that supported the decision dissipated quickly.
    TrendLost,
    /// Price briefly moved in the decision direction but then reversed.
    FalseBreakout,
    /// The decision missed a reversal that occurred soon after.
    ReversalMissed,

    /// ---------- Policy failures ----------
    /// Policy thresholds caused an overly aggressive decision.
    PolicyTooAggressive,
    /// Policy thresholds caused an overly conservative decision.
    PolicyTooConservative,
    /// Policy ignored a risk signal that later materialized.
    PolicyIgnoredRisk,

    /// ---------- Signal failures ----------
    /// Signal suggested action where none was warranted.
    SignalFalsePositive,
    /// Signal failed to suggest action when it should have.
    SignalFalseNegative,
    /// Signal strength decayed rapidly after the decision.
    SignalDecay,

    /// ---------- Market regime failures ----------
    /// Market regime changed shortly after the decision.
    MarketRegimeChanged,
    /// Liquidity collapsed, preventing the expected price path.
    LiquidityCollapse,
    /// Market breadth deteriorated, undermining the decision premise.
    BreadthDeterioration,

    /// ---------- Execution / microstructure failures ----------
    /// Gap or slippage prevented the intended entry/exit price.
    GapSlippage,
    /// Volume was insufficient to support the decision assumption.
    VolumeInsufficient,

    /// ---------- Catch-all ----------
    /// Evaluation could not be determined from the available information.
    EvaluationFailure,
}

/// Combination of an execution event, its realized outcome, and its research label.
///
/// This is the primary record persisted to the V8 Research Asset workspace. It is
/// the supervision sample used by later Policy Calibration and Pattern Discovery.
///
/// `outcome` is immutable: once computed from historical bars, it should never
/// change. `evaluation` is re-runnable: a new `EvaluationEngine` version can
/// produce a new label from the same `(event, outcome)` pair. Therefore the record
/// carries `evaluation_version` so that Research Asset consumers know which
/// evaluation rules were used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResearchRecord {
    pub event: ExecutionEvent,
    pub outcome: ExecutionOutcome,
    pub evaluation: ExecutionEvaluation,
    pub evaluation_version: String,
    pub evaluated_at: DateTime<Utc>,
}

/// Legacy alias for `ExecutionResearchRecord`.
///
/// Kept for compatibility with earlier naming; prefer `ExecutionResearchRecord` in
/// new code.
pub type ExecutionReplayRecord = ExecutionResearchRecord;

/// Resolves the forward-looking objective outcome for an execution event.
///
/// Implementations read historical bars from `market-store` and compute returns,
/// MFE, MAE, drawdown, and benchmark comparisons. The trait is defined in this
/// crate so that `execution-engine` remains free of persistence concerns.
pub trait ReplayOutcomeResolver {
    fn resolve(&self, event: &ExecutionEvent, as_of: NaiveDate) -> anyhow::Result<ExecutionOutcome>;
}

/// Evaluates a realized outcome against the original decision.
///
/// Implementations apply deterministic rules to map `(ExecutionEvent,
/// ExecutionOutcome)` to an `ExecutionEvaluation`. They do not access future market
/// data and do not call LLMs.
pub trait EvaluationEngine {
    fn evaluate(&self, event: &ExecutionEvent, outcome: &ExecutionOutcome) -> ExecutionEvaluation;
}

/// Loads and stores research records to the Research Asset workspace.
pub trait ReplayStore {
    fn save(&self, record: &ExecutionResearchRecord) -> anyhow::Result<()>;
    fn load(&self, execution_id: &str) -> anyhow::Result<Option<ExecutionResearchRecord>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_taxonomy_covers_key_categories() {
        // This test documents the taxonomy. If a label is removed, it is a breaking
        // contract change and requires ADR review.
        let labels = vec![
            ExecutionEvaluation::AwaitingOutcome,
            ExecutionEvaluation::Hit,
            ExecutionEvaluation::TimingAcceptable,
            ExecutionEvaluation::RiskWellManaged,
            ExecutionEvaluation::TooEarly,
            ExecutionEvaluation::TooLate,
            ExecutionEvaluation::TrendLost,
            ExecutionEvaluation::FalseBreakout,
            ExecutionEvaluation::ReversalMissed,
            ExecutionEvaluation::PolicyTooAggressive,
            ExecutionEvaluation::PolicyTooConservative,
            ExecutionEvaluation::PolicyIgnoredRisk,
            ExecutionEvaluation::SignalFalsePositive,
            ExecutionEvaluation::SignalFalseNegative,
            ExecutionEvaluation::SignalDecay,
            ExecutionEvaluation::MarketRegimeChanged,
            ExecutionEvaluation::LiquidityCollapse,
            ExecutionEvaluation::BreadthDeterioration,
            ExecutionEvaluation::GapSlippage,
            ExecutionEvaluation::VolumeInsufficient,
            ExecutionEvaluation::EvaluationFailure,
        ];
        assert_eq!(labels.len(), 21);
    }

    #[test]
    fn evaluation_trait_exists() {
        struct AwaitingEvaluator;
        impl EvaluationEngine for AwaitingEvaluator {
            fn evaluate(&self, _event: &ExecutionEvent, _outcome: &ExecutionOutcome) -> ExecutionEvaluation {
                ExecutionEvaluation::AwaitingOutcome
            }
        }
        let _evaluator = AwaitingEvaluator;
        // The trait is instantiated. Full integration tests require a real
        // ExecutionEvent fixture and live in the caller (app-service or execution-engine).
    }

    #[test]
    fn outcome_round_trips_through_json() {
        let outcome = ExecutionOutcome {
            t20_return: Some(0.05),
            mfe: Some(0.08),
            mae: Some(-0.02),
            ..Default::default()
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let restored: ExecutionOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.t20_return, Some(0.05));
        assert_eq!(restored.mfe, Some(0.08));
        assert_eq!(restored.mae, Some(-0.02));
    }

    #[test]
    fn evaluation_round_trips_through_json() {
        let labels = vec![
            ExecutionEvaluation::Hit,
            ExecutionEvaluation::FalseBreakout,
            ExecutionEvaluation::PolicyTooConservative,
            ExecutionEvaluation::SignalDecay,
            ExecutionEvaluation::LiquidityCollapse,
        ];
        for label in labels {
            let json = serde_json::to_string(&label).unwrap();
            let restored: ExecutionEvaluation = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, label);
        }
    }

    #[test]
    fn replay_single_produces_record() {
        struct StubResolver;
        impl ReplayOutcomeResolver for StubResolver {
            fn resolve(&self, _event: &ExecutionEvent, _as_of: NaiveDate) -> anyhow::Result<ExecutionOutcome> {
                Ok(ExecutionOutcome {
                    t20_return: Some(0.02),
                    ..Default::default()
                })
            }
        }

        struct StubEvaluator;
        impl EvaluationEngine for StubEvaluator {
            fn evaluate(&self, _event: &ExecutionEvent, _outcome: &ExecutionOutcome) -> ExecutionEvaluation {
                ExecutionEvaluation::Hit
            }
        }

        // Minimal ExecutionEvent deserialized for the test.
        let event: ExecutionEvent = serde_json::from_str("{\"execution_id\":\"EE-test\",\"timestamp\":\"2026-07-17T00:00:00Z\",\"versions\":{\"schema_version\":\"v2.0\",\"engine_version\":\"v2.0.0-mvp\",\"policy_version\":\"abc\",\"research_version\":\"1\"},\"policy\":{\"min_signal_score\":0.0,\"allow_chase\":false,\"max_gap_pct\":0.0,\"confidence_threshold\":0.0,\"allow_left_probe\":false,\"risk_budget\":0.0,\"min_volume_ratio\":0.0,\"max_distance_ma_pct\":0.0,\"assessment_mode\":\"EQUAL_WEIGHT\",\"risk_threshold_low\":0.0,\"risk_threshold_high\":0.0,\"consensus_threshold\":0.0,\"buy_threshold\":0.0,\"reduce_threshold\":0.0},\"policy_hash\":\"abc\",\"request\":{\"symbol\":\"000001\",\"date\":\"2026-07-17\",\"signal\":{\"date\":\"2026-07-17\",\"symbol\":\"000001\",\"final_score\":0.0,\"signal_label\":\"Buy\",\"analysis_scope\":\"CN\",\"regime_basis_scope\":\"CN\",\"reason\":{\"best_strategy\":\"MomentumRight\",\"strategy_score\":0.0,\"strategy_contribution\":0.0,\"alignment\":0,\"aligned_strategies\":[],\"alignment_contribution\":0.0,\"regime\":{\"trend_score\":0.0,\"risk_score\":0.0,\"combined_score\":0.0,\"contribution\":0.0},\"rotation\":{\"momentum_score\":0.0,\"rank\":null,\"combined_score\":0.0,\"contribution\":0.0},\"final_score\":0.0,\"label\":\"Buy\",\"summary\":\"test\"}},\"strategy_state\":{\"date\":\"2026-07-17\",\"scope\":\"CN\",\"state\":\"FULL_TREND\",\"state_score\":0.0,\"transition_reason\":\"test\",\"recommended_position_pct\":0.0},\"quote\":{\"symbol\":\"000001\",\"ts\":\"2026-07-17T00:00:00Z\",\"open\":10.0,\"high\":11.0,\"low\":9.0,\"close\":10.5,\"volume\":1000.0,\"prev_close\":10.0},\"volume_ma20\":1000.0,\"market_view\":{\"research_version\":\"1\",\"market_regime_label\":\"Bullish\",\"confirmation\":{\"trend\":{\"score\":0.0,\"label\":\"\"},\"participation\":{\"score\":0.0,\"label\":\"\"},\"risk\":{\"score\":0.0,\"label\":\"\"},\"overall\":\"\"},\"breadth\":{\"breadth_pct\":0.0,\"sma5\":null,\"delta_5d\":null,\"condition\":\"\"},\"recovery\":{\"score\":0.0,\"drivers\":[]},\"rotation_state\":\"\",\"leadership_stability\":0.0},\"policy\":{\"min_signal_score\":0.0,\"allow_chase\":false,\"max_gap_pct\":0.0,\"confidence_threshold\":0.0,\"allow_left_probe\":false,\"risk_budget\":0.0,\"min_volume_ratio\":0.0,\"max_distance_ma_pct\":0.0,\"assessment_mode\":\"EQUAL_WEIGHT\",\"risk_threshold_low\":0.0,\"risk_threshold_high\":0.0,\"consensus_threshold\":0.0,\"buy_threshold\":0.0,\"reduce_threshold\":0.0}},\"features\":{\"symbol\":\"000001\",\"today_return\":0.0,\"open_return\":0.0,\"gap_pct\":0.0,\"close_position\":0.0,\"amplitude_pct\":0.0,\"upper_shadow_pct\":0.0,\"lower_shadow_pct\":0.0,\"volume_ratio\":0.0,\"body_ratio\":0.0,\"gap_fill_ratio\":0.0},\"observations\":[],\"evidences\":[],\"assessment\":{\"confidence\":0.0,\"consensus\":0.0,\"coverage\":0.0,\"risk\":\"LOW\",\"dominant_direction\":0.0,\"supporting_evidence\":[],\"conflicting_evidence\":[],\"neutral_evidence\":[]},\"decision\":{\"symbol\":\"000001\",\"state\":\"WAIT\",\"confidence\":0.0,\"risk\":\"LOW\",\"evidences\":[],\"assessment\":{\"confidence\":0.0,\"consensus\":0.0,\"coverage\":0.0,\"risk\":\"LOW\",\"dominant_direction\":0.0,\"supporting_evidence\":[],\"conflicting_evidence\":[],\"neutral_evidence\":[]},\"decision_reasons\":[]}}").unwrap();

        let as_of = NaiveDate::from_ymd_opt(2026, 7, 18).unwrap();
        let record = replay_single(&StubResolver, &StubEvaluator, &event, as_of).unwrap();

        assert_eq!(record.event.execution_id, "EE-test");
        assert_eq!(record.evaluation, ExecutionEvaluation::Hit);
        assert_eq!(record.outcome.t20_return, Some(0.02));
    }
}

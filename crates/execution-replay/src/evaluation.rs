use execution_engine::v2::decision::ExecutionDecision;
use execution_engine::v2::event::ExecutionEvent;
use execution_engine::types::ExecutionState;

use crate::{EvaluationEngine, ExecutionEvaluation, ExecutionOutcome};

/// Rule-based evaluation engine.
///
/// Maps `(ExecutionEvent, ExecutionOutcome)` to a fixed `ExecutionEvaluation`
/// taxonomy. The rules are intentionally simple and deterministic; they serve as
/// the initial supervision signal for the Research Asset. More sophisticated
/// evaluation (e.g., regime-aware attribution) can be layered on later without
/// changing the contract.
#[derive(Debug, Clone, Default)]
pub struct RuleBasedEvaluationEngine;

impl EvaluationEngine for RuleBasedEvaluationEngine {
    fn evaluate(&self, event: &ExecutionEvent, outcome: &ExecutionOutcome) -> ExecutionEvaluation {
        if outcome.t20_return.is_none() {
            return ExecutionEvaluation::AwaitingOutcome;
        }

        let decision = &event.decision;
        let t20 = outcome.t20_return.unwrap_or(0.0);
        let mfe = outcome.mfe.unwrap_or(0.0);
        let mae = outcome.mae.unwrap_or(0.0);
        let max_dd = outcome.max_drawdown.unwrap_or(0.0);

        match decision.state {
            ExecutionState::Increase => {
                evaluate_long(t20, mfe, mae, max_dd, decision)
            }
            ExecutionState::Reduce => {
                // For reduce decisions, success is negative return.
                evaluate_short(t20, mfe, mae, max_dd, decision)
            }
            _ => ExecutionEvaluation::EvaluationFailure,
        }
    }
}

fn evaluate_long(t20: f64, mfe: f64, mae: f64, max_dd: f64, decision: &ExecutionDecision) -> ExecutionEvaluation {
    if t20 > 0.01 {
        if mfe >= 0.03 && mae >= -0.02 {
            return ExecutionEvaluation::RiskWellManaged;
        }
        return ExecutionEvaluation::Hit;
    }

    if t20 < -0.02 {
        if mfe >= 0.03 {
            return ExecutionEvaluation::FalseBreakout;
        }
        if mae <= -0.05 {
            return ExecutionEvaluation::LiquidityCollapse;
        }
        if max_dd <= -0.05 {
            return ExecutionEvaluation::TrendLost;
        }
        return ExecutionEvaluation::SignalFalsePositive;
    }

    if t20 > 0.0 && mfe < 0.01 {
        return ExecutionEvaluation::TooLate;
    }

    if t20 < 0.0 && mfe > 0.05 {
        return ExecutionEvaluation::TooEarly;
    }

    if decision.confidence < 0.4 {
        return ExecutionEvaluation::PolicyTooConservative;
    }

    ExecutionEvaluation::EvaluationFailure
}

fn evaluate_short(t20: f64, mfe: f64, mae: f64, max_dd: f64, decision: &ExecutionDecision) -> ExecutionEvaluation {
    // For short/reduce, positive return is failure, negative return is success.
    if t20 < -0.01 {
        if mae.abs() >= 0.03 && mfe <= 0.02 {
            return ExecutionEvaluation::RiskWellManaged;
        }
        return ExecutionEvaluation::Hit;
    }

    if t20 > 0.02 {
        if mae.abs() >= 0.03 {
            return ExecutionEvaluation::FalseBreakout;
        }
        if mfe >= 0.05 {
            return ExecutionEvaluation::LiquidityCollapse;
        }
        if max_dd >= 0.05 {
            return ExecutionEvaluation::TrendLost;
        }
        return ExecutionEvaluation::SignalFalsePositive;
    }

    if t20 < 0.0 && mae.abs() < 0.01 {
        return ExecutionEvaluation::TooLate;
    }

    if t20 > 0.0 && mae.abs() > 0.05 {
        return ExecutionEvaluation::TooEarly;
    }

    if decision.confidence < 0.4 {
        return ExecutionEvaluation::PolicyTooConservative;
    }

    ExecutionEvaluation::EvaluationFailure
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use execution_engine::v2::assessment::{ExecutionAssessment, RiskLevel};
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::feature::IntradayFeatures;
    use execution_engine::v2::request::{ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot};
    use execution_engine::types::ExecutionState;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use research_context::{BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary};

    fn make_event(state: ExecutionState, confidence: f64) -> ExecutionEvent {
        let request = ExecutionRequest {
            symbol: "000001".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            signal: core_domain::SignalSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
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
                    regime: core_domain::RegimeReason { trend_score: 0.0, risk_score: 0.0, combined_score: 0.0, contribution: 0.0 },
                    rotation: core_domain::RotationReason { momentum_score: 0.0, rank: None, combined_score: 0.0, contribution: 0.0 },
                    final_score: 85.0,
                    label: SignalLabel::StrongBuy,
                    summary: "test".into(),
                },
            },
            strategy_state: core_domain::StrategyStateSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                scope: "CN".into(),
                state: StrategyState::FullTrend,
                state_score: 75.0,
                transition_reason: "test".into(),
                recommended_position_pct: 100.0,
            },
            quote: QuoteSnapshot {
                symbol: "000001".into(),
                ts: Utc::now(),
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1_000_000.0,
                prev_close: 99.0,
            },
            volume_ma20: 500_000.0,
            market_view: ExecutionMarketView {
                research_version: "1".into(),
                market_regime_label: "Bullish".into(),
                confirmation: ConfirmationSummary {
                    trend: ConfirmationDimension { score: 70.0, label: "Strong".into() },
                    participation: ConfirmationDimension { score: 60.0, label: "Moderate".into() },
                    risk: ConfirmationDimension { score: 35.0, label: "Low".into() },
                    overall: "Strong".into(),
                },
                breadth: BreadthSummary { breadth_pct: 60.0, sma5: None, delta_5d: Some(0.0), condition: "strong".into() },
                recovery: RecoverySummary { score: 60.0, drivers: vec![] },
                rotation_state: "broad".into(),
                leadership_stability: 0.7,
            },
            policy: ExecutionPolicy::default(),
        };

        let assessment = ExecutionAssessment {
            confidence,
            consensus: 1.0,
            coverage: 1.0,
            risk: RiskLevel::Low,
            dominant_direction: 1.0,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };

        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state,
            confidence,
            risk: RiskLevel::Low,
            evidences: vec![],
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };

        ExecutionEvent::new(
            request,
            IntradayFeatures::default(),
            vec![],
            vec![],
            assessment,
            decision,
        )
    }

    #[test]
    fn long_hit_when_positive_return() {
        let event = make_event(ExecutionState::Increase, 0.8);
        let outcome = ExecutionOutcome { t20_return: Some(0.03), ..Default::default() };
        let eval = RuleBasedEvaluationEngine.evaluate(&event, &outcome);
        assert_eq!(eval, ExecutionEvaluation::Hit);
    }

    #[test]
    fn long_false_breakout_when_mfe_positive_but_final_negative() {
        let event = make_event(ExecutionState::Increase, 0.8);
        let outcome = ExecutionOutcome {
            t20_return: Some(-0.03),
            mfe: Some(0.04),
            mae: Some(-0.03),
            ..Default::default()
        };
        let eval = RuleBasedEvaluationEngine.evaluate(&event, &outcome);
        assert_eq!(eval, ExecutionEvaluation::FalseBreakout);
    }

    #[test]
    fn awaiting_outcome_when_no_t20_return() {
        let event = make_event(ExecutionState::Increase, 0.8);
        let outcome = ExecutionOutcome::default();
        let eval = RuleBasedEvaluationEngine.evaluate(&event, &outcome);
        assert_eq!(eval, ExecutionEvaluation::AwaitingOutcome);
    }

    #[test]
    fn short_hit_when_negative_return() {
        let event = make_event(ExecutionState::Reduce, 0.8);
        let outcome = ExecutionOutcome { t20_return: Some(-0.03), ..Default::default() };
        let eval = RuleBasedEvaluationEngine.evaluate(&event, &outcome);
        assert_eq!(eval, ExecutionEvaluation::Hit);
    }
}

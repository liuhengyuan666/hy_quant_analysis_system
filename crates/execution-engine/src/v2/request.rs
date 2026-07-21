use chrono::{DateTime, NaiveDate, Utc};
use core_domain::{SignalSnapshot, StrategyStateSnapshot};
use research_context::{BreadthSummary, ConfirmationSummary, RecoverySummary, ResearchContext};
use serde::{Deserialize, Serialize};

/// Raw real-time quote for a single symbol.
///
/// This is the input to the FeatureExtractor. It intentionally carries only
/// provider-level fields, not derived features like `today_return` or
/// `volume_ratio`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteSnapshot {
    pub symbol: String,
    pub ts: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub prev_close: f64,
}

/// Projection of `ResearchContext` into the subset that the Execution Platform
/// needs.
///
/// This is not a copy of `ResearchContext`; it is a stable, narrower view that
/// isolates Execution from future ResearchContext evolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMarketView {
    pub research_version: String,
    pub market_regime_label: String,
    pub confirmation: ConfirmationSummary,
    pub breadth: BreadthSummary,
    pub recovery: RecoverySummary,
    pub rotation_state: String,
    pub leadership_stability: f64,
}

impl ExecutionMarketView {
    pub fn from_research_context(ctx: &ResearchContext) -> Self {
        let market_regime_label = ctx
            .market_state
            .label
            .trim()
            .is_empty()
            .then(|| "Unknown".to_string())
            .unwrap_or_else(|| ctx.market_state.label.clone());
        Self {
            research_version: format!("{}", ctx.version),
            market_regime_label,
            confirmation: ctx.confirmation.clone(),
            breadth: ctx.breadth.clone(),
            recovery: ctx.recovery.clone(),
            rotation_state: ctx.rotation.rotation_state.clone(),
            leadership_stability: ctx.rotation.leadership_stability,
        }
    }
}

/// Tunable parameters for the Execution Platform.
///
/// All thresholds, switches, risk budgets, and assessment modes live here. The
/// Engine itself must not contain hard-coded magic numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub min_signal_score: f64,
    pub allow_chase: bool,
    pub max_gap_pct: f64,
    pub confidence_threshold: f64,
    pub allow_left_probe: bool,
    pub risk_budget: f64,
    pub min_volume_ratio: f64,
    pub max_distance_ma_pct: f64,
    pub assessment_mode: AssessmentMode,
    pub risk_threshold_low: f64,
    pub risk_threshold_high: f64,
    pub consensus_threshold: f64,
    pub buy_threshold: f64,
    pub reduce_threshold: f64,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            min_signal_score: 75.0,
            allow_chase: true,
            max_gap_pct: 0.07,
            confidence_threshold: 0.6,
            allow_left_probe: true,
            risk_budget: 1.0,
            min_volume_ratio: 1.0,
            max_distance_ma_pct: 0.05,
            assessment_mode: AssessmentMode::EqualWeight,
            risk_threshold_low: 0.25,
            risk_threshold_high: 0.55,
            consensus_threshold: 0.5,
            buy_threshold: 0.5,
            reduce_threshold: -0.3,
        }
    }
}

/// Algorithm used by the AssessmentEngine to fuse Evidence.
///
/// This is a stable configuration point, not an enum for extension. New modes
/// may be added as the platform evolves, but the `ExecutionAssessment` contract
/// remains unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssessmentMode {
    EqualWeight,
    // Future: Bayesian, RuleBased, Calibrated, MlRanked
}

/// A single execution evaluation request.
///
/// This is the public input contract. It contains only external inputs:
/// signal, state, quote, pre-computed indicator values, market view, and policy.
/// Pipeline intermediates (features, observations, evidence, assessment) are
/// produced internally by the Engine and must not appear here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub symbol: String,
    pub date: NaiveDate,
    pub signal: SignalSnapshot,
    pub strategy_state: StrategyStateSnapshot,
    pub quote: QuoteSnapshot,
    pub volume_ma20: f64,
    pub market_view: ExecutionMarketView,
    pub policy: ExecutionPolicy,
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use core_domain::AnalysisScope;
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, DivergenceSummary,
        MarketStateSummary, RecoverySummary, ResearchContext, RotationSummary, SignalSummary,
        TrustLevel, TrustSummary,
    };

    use super::*;

    fn make_research_context_with_regime(label: &str) -> ResearchContext {
        ResearchContext {
            version: 1,
            scope: AnalysisScope::Cn,
            date: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            market_state: MarketStateSummary {
                label: label.to_string(),
                trend_score: 50.0,
                liquidity_score: 50.0,
                risk_score: 50.0,
                confidence: 0.6,
            },
            breadth: BreadthSummary {
                breadth_pct: 50.0,
                sma5: None,
                delta_5d: None,
                condition: "moderate".into(),
            },
            rotation: RotationSummary {
                top: vec![],
                bottom: vec![],
                rotation_state: "mixed".into(),
                leadership_stability: 0.5,
                leadership_transition: "stable".into(),
                rotation_acceleration: None,
                theme_dispersion: None,
            },
            signal: SignalSummary {
                signals: vec![],
                bullish_count: 0,
                strong_buy_count: 0,
                average_score: 0.0,
            },
            divergence: DivergenceSummary {
                divergence_duration: 0,
                samples: vec![],
            },
            trust: TrustSummary {
                level: TrustLevel::Unassessed,
                headline: "test".into(),
                is_data_complete: true,
            },
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
            recovery: RecoverySummary {
                score: 50.0,
                drivers: vec![],
            },
            consensus: None,
        }
    }

    #[test]
    fn market_view_preserves_bullish_regime_from_research_context() {
        let ctx = make_research_context_with_regime("Bullish");
        let view = ExecutionMarketView::from_research_context(&ctx);
        assert_eq!(view.market_regime_label, "Bullish");
    }

    #[test]
    fn market_view_preserves_bearish_regime_from_research_context() {
        let ctx = make_research_context_with_regime("Bearish");
        let view = ExecutionMarketView::from_research_context(&ctx);
        assert_eq!(view.market_regime_label, "Bearish");
    }

    #[test]
    fn market_view_falls_back_to_unknown_when_regime_missing() {
        let ctx = make_research_context_with_regime("");
        let view = ExecutionMarketView::from_research_context(&ctx);
        assert_eq!(view.market_regime_label, "Unknown");
    }
}

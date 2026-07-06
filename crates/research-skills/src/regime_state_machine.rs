use llm_context::ResearchContext;
use serde::{Deserialize, Serialize};

/// Market regime states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegimeState {
    RiskOn,
    Neutral,
    RiskOff,
    DeRisk,
}

/// Transition result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionResult {
    pub from: RegimeState,
    pub to: RegimeState,
    pub triggered: bool,
    pub condition_met: String,
    pub confidence: f64,
}

/// State machine for market regime transitions
pub struct RegimeStateMachine;

impl RegimeStateMachine {
    /// Detect if a transition should occur
    pub fn detect_transition(
        current: RegimeState,
        context: &ResearchContext,
    ) -> Option<TransitionResult> {
        let breadth_pct = context.breadth.breadth_pct;
        let _breadth_delta = context.breadth.breadth_delta;
        let liquidity_critical = matches!(
            context.liquidity.pressure,
            llm_context::LiquidityPressure::Critical
        );
        let liquidity_high = matches!(
            context.liquidity.pressure,
            llm_context::LiquidityPressure::High
                | llm_context::LiquidityPressure::Critical
        );
        let macro_stale = context.regime.macro_stale_days > 3;

        match current {
            RegimeState::RiskOn => {
                // risk_on → de_risk: breadth collapse + liquidity stress
                if breadth_pct < 30.0 && liquidity_high {
                    return Some(TransitionResult {
                        from: current,
                        to: RegimeState::DeRisk,
                        triggered: true,
                        condition_met: "breadth_pct < 30 && liquidity_high".to_string(),
                        confidence: 0.85,
                    });
                }
                // risk_on → neutral: breadth weakening or stale macro
                if breadth_pct < 50.0 || macro_stale {
                    return Some(TransitionResult {
                        from: current,
                        to: RegimeState::Neutral,
                        triggered: true,
                        condition_met: "breadth_pct < 50 || macro_stale".to_string(),
                        confidence: 0.7,
                    });
                }
            }
            RegimeState::Neutral => {
                // neutral → risk_off: severe conditions
                if breadth_pct < 30.0 || liquidity_critical {
                    return Some(TransitionResult {
                        from: current,
                        to: RegimeState::RiskOff,
                        triggered: true,
                        condition_met: "breadth_pct < 30 || liquidity_critical".to_string(),
                        confidence: 0.8,
                    });
                }
                // neutral → risk_on: strong recovery (only when liquidity is explicitly low)
                if breadth_pct > 60.0
                    && matches!(
                        context.liquidity.pressure,
                        llm_context::LiquidityPressure::Low
                    )
                {
                    return Some(TransitionResult {
                        from: current,
                        to: RegimeState::RiskOn,
                        triggered: true,
                        condition_met: "breadth_pct > 60 && liquidity_low".to_string(),
                        confidence: 0.75,
                    });
                }
            }
            RegimeState::RiskOff => {
                // risk_off → neutral: recovery signals
                if breadth_pct > 40.0 && !liquidity_critical {
                    return Some(TransitionResult {
                        from: current,
                        to: RegimeState::Neutral,
                        triggered: true,
                        condition_met: "breadth_pct > 40 && !liquidity_critical".to_string(),
                        confidence: 0.7,
                    });
                }
            }
            RegimeState::DeRisk => {
                // de_risk → risk_on: full recovery
                if breadth_pct > 60.0 && context.regime.macro_stale_days <= 2 {
                    return Some(TransitionResult {
                        from: current,
                        to: RegimeState::RiskOn,
                        triggered: true,
                        condition_met: "breadth_pct > 60 && macro_stale_days <= 2".to_string(),
                        confidence: 0.8,
                    });
                }
                // de_risk → risk_off: further deterioration
                if liquidity_critical {
                    return Some(TransitionResult {
                        from: current,
                        to: RegimeState::RiskOff,
                        triggered: true,
                        condition_met: "liquidity_critical".to_string(),
                        confidence: 0.85,
                    });
                }
            }
        }

        None // No transition
    }

    /// Get current regime state from context
    pub fn current_state(context: &ResearchContext) -> RegimeState {
        match context.market.current_state.as_str() {
            "risk_on" => RegimeState::RiskOn,
            "risk_off" => RegimeState::RiskOff,
            "de_risk" => RegimeState::DeRisk,
            _ => RegimeState::Neutral,
        }
    }

    /// Calculate confidence based on data quality
    pub fn calculate_confidence(context: &ResearchContext) -> f64 {
        let mut confidence: f64 = 0.7; // Base confidence

        // Reduce if macro data is stale
        if context.regime.macro_stale_days > 3 {
            confidence -= 0.1;
        }
        if context.regime.macro_stale_days > 5 {
            confidence -= 0.1;
        }

        // Reduce if breadth data is missing
        if context.breadth.breadth_pct == 0.0 {
            confidence -= 0.1;
        }

        // Reduce if liquidity data is missing
        if context.liquidity.dollar_strength.is_none() {
            confidence -= 0.05;
        }

        confidence.max(0.3).min(1.0)
    }
}

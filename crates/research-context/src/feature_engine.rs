use serde::{Deserialize, Serialize};

/// Semantic feature extracted from market data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticFeature {
    pub name: String,
    pub description: String,
    pub value: FeatureValue,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureValue {
    Boolean(bool),
    Float(f64),
    Category(String),
}

/// Trait for computing semantic features
pub trait FeatureComputer {
    fn name(&self) -> &str;
    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature>;
}

/// Breadth collapse detection
pub struct BreadthCollapseFeature;

impl FeatureComputer for BreadthCollapseFeature {
    fn name(&self) -> &str {
        "breadth_collapse"
    }

    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature> {
        let collapsed = context.breadth.breadth_pct < 30.0
            && context.breadth.breadth_delta < -10.0;
        Some(SemanticFeature {
            name: self.name().to_string(),
            description: "Detect significant breadth deterioration".to_string(),
            value: FeatureValue::Boolean(collapsed),
            confidence: 0.9,
        })
    }
}

/// Rotation concentration detection
pub struct RotationConcentrationFeature;

impl FeatureComputer for RotationConcentrationFeature {
    fn name(&self) -> &str {
        "rotation_concentration"
    }

    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature> {
        let concentrated = matches!(
            context.rotation.state,
            super::RotationState::Concentrated
        );
        Some(SemanticFeature {
            name: self.name().to_string(),
            description: "Detect concentrated rotation".to_string(),
            value: FeatureValue::Boolean(concentrated),
            confidence: 0.85,
        })
    }
}

/// Liquidity fragility detection
pub struct LiquidityFragilityFeature;

impl FeatureComputer for LiquidityFragilityFeature {
    fn name(&self) -> &str {
        "liquidity_fragility"
    }

    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature> {
        let fragile = matches!(
            context.liquidity.pressure,
            super::LiquidityPressure::High | super::LiquidityPressure::Critical
        );
        Some(SemanticFeature {
            name: self.name().to_string(),
            description: "Detect liquidity fragility".to_string(),
            value: FeatureValue::Boolean(fragile),
            confidence: 0.8,
        })
    }
}

/// Market regime risk-off detection
pub struct RegimeRiskOffFeature;

impl FeatureComputer for RegimeRiskOffFeature {
    fn name(&self) -> &str {
        "regime_risk_off"
    }

    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature> {
        let risk_off = context.regime.current == "risk_off";
        Some(SemanticFeature {
            name: self.name().to_string(),
            description: "Detect risk-off regime".to_string(),
            value: FeatureValue::Boolean(risk_off),
            confidence: context.regime.confidence,
        })
    }
}

/// High macro staleness detection
pub struct MacroStaleFeature;

impl FeatureComputer for MacroStaleFeature {
    fn name(&self) -> &str {
        "macro_stale"
    }

    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature> {
        let stale = context.regime.macro_stale_days > 3;
        Some(SemanticFeature {
            name: self.name().to_string(),
            description: "Detect stale macro data".to_string(),
            value: FeatureValue::Boolean(stale),
            confidence: 0.95,
        })
    }
}

/// Bullish signal dominance
pub struct BullishDominanceFeature;

impl FeatureComputer for BullishDominanceFeature {
    fn name(&self) -> &str {
        "bullish_dominance"
    }

    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature> {
        let total = context.signals.bullish_count + context.signals.defensive_count;
        let dominance = if total > 0 {
            context.signals.bullish_count as f64 / total as f64
        } else {
            0.5
        };
        Some(SemanticFeature {
            name: self.name().to_string(),
            description: "Bullish signal proportion".to_string(),
            value: FeatureValue::Float(dominance),
            confidence: 0.8,
        })
    }
}

/// Data starvation detection
pub struct DataStarvedFeature;

impl FeatureComputer for DataStarvedFeature {
    fn name(&self) -> &str {
        "data_starved"
    }

    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature> {
        let starved = context.signals.data_starved_count > 0;
        if !starved {
            return None;
        }
        Some(SemanticFeature {
            name: self.name().to_string(),
            description: "Detect data-starved signals".to_string(),
            value: FeatureValue::Boolean(starved),
            confidence: 0.9,
        })
    }
}

/// Market state transition detection
pub struct StateTransitionFeature;

impl FeatureComputer for StateTransitionFeature {
    fn name(&self) -> &str {
        "state_transition"
    }

    fn compute(&self, context: &super::ResearchContext) -> Option<SemanticFeature> {
        let transitioning = context.market.transition.is_some();
        if !transitioning {
            return None;
        }
        Some(SemanticFeature {
            name: self.name().to_string(),
            description: "Detect regime state transition".to_string(),
            value: FeatureValue::Boolean(transitioning),
            confidence: 0.85,
        })
    }
}

/// Get all built-in feature computers
pub fn builtin_features() -> Vec<Box<dyn FeatureComputer>> {
    vec![
        Box::new(BreadthCollapseFeature),
        Box::new(RotationConcentrationFeature),
        Box::new(LiquidityFragilityFeature),
        Box::new(RegimeRiskOffFeature),
        Box::new(MacroStaleFeature),
        Box::new(BullishDominanceFeature),
        Box::new(DataStarvedFeature),
        Box::new(StateTransitionFeature),
    ]
}

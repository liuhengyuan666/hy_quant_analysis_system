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

/// Get all built-in feature computers
pub fn builtin_features() -> Vec<Box<dyn FeatureComputer>> {
    vec![
        Box::new(BreadthCollapseFeature),
        Box::new(RotationConcentrationFeature),
        Box::new(LiquidityFragilityFeature),
    ]
}

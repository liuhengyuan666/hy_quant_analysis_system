use research_context::ResearchContext;

use super::registry::SkillRegistry;
use super::trigger::{ComparisonOp, Condition, ConditionValue, Trigger};

/// Router that matches skills to ResearchContext based on triggers
pub struct SkillRouter;

/// A matched skill with its weight score
#[derive(Debug, Clone)]
pub struct MatchedSkill {
    pub name: String,
    pub weight: f64,
}

impl SkillRouter {
    /// Find all skills that match the given context
    pub fn route(registry: &SkillRegistry, context: &ResearchContext) -> Vec<MatchedSkill> {
        let mut matched = Vec::new();

        for name in registry.list() {
            if let Some(skill) = registry.get(name) {
                if Self::evaluate_trigger(&skill.definition.trigger, context) {
                    let weight = Self::calculate_weight(&skill.definition.trigger);
                    matched.push(MatchedSkill {
                        name: name.to_string(),
                        weight,
                    });
                }
            }
        }

        // Sort by weight descending
        matched.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matched
    }

    /// Evaluate a trigger against a context.
    /// all AND any AND none must all pass.
    pub fn evaluate_trigger(trigger: &Trigger, context: &ResearchContext) -> bool {
        // all: all conditions must be true (empty = true)
        let all_pass = trigger.all.is_empty()
            || trigger.all.iter().all(|c| Self::evaluate_condition(c, context));

        // any: at least one condition must be true (empty = true)
        let any_pass = trigger.any.is_empty()
            || trigger.any.iter().any(|c| Self::evaluate_condition(c, context));

        // none: no conditions can be true (empty = true)
        let none_pass = trigger.none.is_empty()
            || !trigger.none.iter().any(|c| Self::evaluate_condition(c, context));

        all_pass && any_pass && none_pass
    }

    /// Evaluate a single condition against context
    fn evaluate_condition(condition: &Condition, context: &ResearchContext) -> bool {
        let field_value = Self::extract_field(&condition.field, context);

        match field_value {
            Some(value) => Self::compare(&condition.operator, &value, &condition.value),
            None => false, // Field not found or None = condition fails
        }
    }

    /// Extract a field value from context using dot notation.
    /// Supports all fields used by skill triggers including strings and Option<f64>.
    fn extract_field(field: &str, context: &ResearchContext) -> Option<ConditionValue> {
        let parts: Vec<&str> = field.split('.').collect();

        match parts.as_slice() {
            // ---------- market ----------
            ["market", "confidence"] => Some(ConditionValue::Float(context.market.confidence)),

            // ---------- breadth ----------
            ["breadth", "breadth_pct"] => Some(ConditionValue::Float(context.breadth.breadth_pct)),
            ["breadth", "breadth_delta"] => Some(ConditionValue::Float(context.breadth.breadth_delta)),

            // ---------- regime ----------
            ["regime", "confidence"] => Some(ConditionValue::Float(context.regime.confidence)),
            ["regime", "macro_stale_days"] => {
                Some(ConditionValue::Integer(context.regime.macro_stale_days as i64))
            }
            ["regime", "current"] => Some(ConditionValue::String(context.regime.current.clone())),

            // ---------- signals ----------
            ["signals", "bullish_count"] => {
                Some(ConditionValue::Integer(context.signals.bullish_count as i64))
            }
            ["signals", "defensive_count"] => {
                Some(ConditionValue::Integer(context.signals.defensive_count as i64))
            }
            ["signals", "data_starved_count"] => {
                Some(ConditionValue::Integer(context.signals.data_starved_count as i64))
            }

            // ---------- liquidity ----------
            ["liquidity", "dollar_strength"] => {
                context.liquidity.dollar_strength.map(ConditionValue::Float)
            }
            ["liquidity", "pressure"] => Some(ConditionValue::String(
                format!("{:?}", context.liquidity.pressure).to_lowercase(),
            )),

            // ---------- rotation ----------
            ["rotation", "momentum_factor"] => {
                context.rotation.momentum_factor.map(ConditionValue::Float)
            }
            ["rotation", "value_factor"] => {
                context.rotation.value_factor.map(ConditionValue::Float)
            }
            ["rotation", "quality_factor"] => {
                context.rotation.quality_factor.map(ConditionValue::Float)
            }
            ["rotation", "crowding_factor"] => {
                context.rotation.crowding_factor.map(ConditionValue::Float)
            }

            // ---------- macro ----------
            ["macro", "spread_10y"] => context.macro_.spread_10y.map(ConditionValue::Float),
            ["macro", "dxy_index"] => context.macro_.dxy_index.map(ConditionValue::Float),
            ["macro", "foreign_flow"] => context.macro_.foreign_flow.map(ConditionValue::Float),
            ["macro", "vix"] => context.macro_.vix.map(ConditionValue::Float),

            // ---------- risk ----------
            ["risk", "skewness"] => context.risk.skewness.map(ConditionValue::Float),
            ["risk", "kurtosis"] => context.risk.kurtosis.map(ConditionValue::Float),
            ["risk", "tail_index"] => context.risk.tail_index.map(ConditionValue::Float),

            _ => None,
        }
    }

    /// Compare two ConditionValues using the operator.
    /// Handles numeric (Float/Integer), Boolean, and String comparisons.
    /// Mixed numeric types are promoted to Float.
    fn compare(op: &ComparisonOp, left: &ConditionValue, right: &ConditionValue) -> bool {
        match (left, right) {
            // --- Float/Float ---
            (ConditionValue::Float(l), ConditionValue::Float(r)) => match op {
                ComparisonOp::LessThan => l < r,
                ComparisonOp::LessThanOrEqual => l <= r,
                ComparisonOp::GreaterThan => l > r,
                ComparisonOp::GreaterThanOrEqual => l >= r,
                ComparisonOp::Equal => (l - r).abs() < f64::EPSILON,
                ComparisonOp::NotEqual => (l - r).abs() >= f64::EPSILON,
            },

            // --- Integer/Integer → promote to Float ---
            (ConditionValue::Integer(l), ConditionValue::Integer(r)) => {
                Self::compare(
                    op,
                    &ConditionValue::Float(*l as f64),
                    &ConditionValue::Float(*r as f64),
                )
            }

            // --- Mixed Integer/Float → promote to Float ---
            (ConditionValue::Float(_l), ConditionValue::Integer(r)) => {
                Self::compare(op, left, &ConditionValue::Float(*r as f64))
            }
            (ConditionValue::Integer(l), ConditionValue::Float(_r)) => {
                Self::compare(op, &ConditionValue::Float(*l as f64), right)
            }

            // --- Boolean/Boolean ---
            (ConditionValue::Boolean(l), ConditionValue::Boolean(r)) => match op {
                ComparisonOp::Equal => l == r,
                ComparisonOp::NotEqual => l != r,
                _ => false,
            },

            // --- String/String (case-insensitive == and != only) ---
            (ConditionValue::String(l), ConditionValue::String(r)) => match op {
                ComparisonOp::Equal => l.eq_ignore_ascii_case(r),
                ComparisonOp::NotEqual => !l.eq_ignore_ascii_case(r),
                _ => false,
            },

            // --- Mixed types → not supported ---
            _ => false,
        }
    }

    /// Calculate weight from trigger weight map
    pub fn calculate_weight(trigger: &Trigger) -> f64 {
        if trigger.weight.is_empty() {
            return 0.5; // Default weight
        }

        // Average of all weight values
        let sum: f64 = trigger.weight.values().sum();
        sum / trigger.weight.len() as f64
    }
}

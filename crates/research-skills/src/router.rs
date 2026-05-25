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
            Some(value) => Self::compare(&condition.operator, value, &condition.value),
            None => false, // Field not found = condition fails
        }
    }

    /// Extract a field value from context using dot notation.
    /// Supports: market.confidence, breadth.breadth_pct, regime.macro_stale_days, etc.
    fn extract_field(field: &str, context: &ResearchContext) -> Option<f64> {
        let parts: Vec<&str> = field.split('.').collect();

        match parts.as_slice() {
            ["market", "confidence"] => Some(context.market.confidence),
            ["breadth", "breadth_pct"] => Some(context.breadth.breadth_pct),
            ["breadth", "breadth_delta"] => Some(context.breadth.breadth_delta),
            ["regime", "confidence"] => Some(context.regime.confidence),
            ["regime", "macro_stale_days"] => Some(context.regime.macro_stale_days as f64),
            ["signals", "bullish_count"] => Some(context.signals.bullish_count as f64),
            ["signals", "defensive_count"] => Some(context.signals.defensive_count as f64),
            ["signals", "data_starved_count"] => Some(context.signals.data_starved_count as f64),
            ["liquidity", "dollar_strength"] => context.liquidity.dollar_strength,
            _ => None,
        }
    }

    /// Compare values using the operator
    fn compare(op: &ComparisonOp, left: f64, right: &ConditionValue) -> bool {
        match right {
            ConditionValue::Float(r) => match op {
                ComparisonOp::LessThan => left < *r,
                ComparisonOp::LessThanOrEqual => left <= *r,
                ComparisonOp::GreaterThan => left > *r,
                ComparisonOp::GreaterThanOrEqual => left >= *r,
                ComparisonOp::Equal => (left - r).abs() < f64::EPSILON,
                ComparisonOp::NotEqual => (left - r).abs() >= f64::EPSILON,
            },
            ConditionValue::Integer(r) => {
                Self::compare(op, left, &ConditionValue::Float(*r as f64))
            }
            ConditionValue::Boolean(r) => {
                let left_bool = left != 0.0;
                match op {
                    ComparisonOp::Equal => left_bool == *r,
                    ComparisonOp::NotEqual => left_bool != *r,
                    _ => false,
                }
            }
            ConditionValue::String(_) => false, // String comparison not supported for numeric fields
        }
    }

    /// Calculate weight from trigger weight map
    fn calculate_weight(trigger: &Trigger) -> f64 {
        if trigger.weight.is_empty() {
            return 0.5; // Default weight
        }

        // Average of all weight values
        let sum: f64 = trigger.weight.values().sum();
        sum / trigger.weight.len() as f64
    }
}

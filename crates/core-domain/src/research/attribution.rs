//! Research Attribution Framework.
//!
//! Defines the architecture, lifecycle, and extension contract for explaining why
//! the same signal/condition performs differently across market regimes.
//!
//! Lifecycle:
//!   Observation → Evidence → Attribution → Hypothesis → Validation → Knowledge
//!
//! Phase 1 establishes the framework contract. Phase 2 (TASK-104) adds the
//! Breadth + Liquidity MVP dimensions. Additional dimensions are added without
//! modifying the core contract or frozen layers below.
//!
//! All computation here is pure: no I/O, no DB access, no LLM calls.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Raw market-state facts consumed by attribution dimensions.
///
/// This is the Observation layer: everything is optional because different
/// dimensions need different inputs. Missing fields are handled by the
/// dimension itself, not by the framework.
#[derive(Debug, Clone, Default)]
pub struct AttributionInput {
    pub date: NaiveDate,
    pub scope: String,
    pub condition: String,
    pub state: Option<String>,
    pub signal_summary: Option<String>,
    pub breadth_pct: Option<f64>,
    pub breadth_pct_sma5: Option<f64>,
    pub liquidity_score: Option<f64>,
    pub turnover_coverage_pct: Option<f64>,
    pub volatility_level: Option<f64>,
    pub macro_regime: Option<String>,
    pub rotation_top5_concentration: Option<f64>,
}

/// A single attribution dimension (e.g., Breadth, Liquidity, Macro, Crowding).
///
/// Object-safe trait: no generic methods, no associated types.
pub trait AttributionDimension: Send + Sync {
    fn name(&self) -> &'static str;
    fn evaluate(&self, input: &AttributionInput) -> AttributionResult;
}

/// Result from a single attribution dimension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AttributionResult {
    pub dimension: String,
    pub level: AttributionLevel,
    pub score: Option<f64>,
    pub reason: String,
}

impl AttributionResult {
    pub fn unknown(dimension: &str, reason: impl Into<String>) -> Self {
        Self {
            dimension: dimension.to_string(),
            level: AttributionLevel::Unknown,
            score: None,
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttributionLevel {
    Normal,
    Elevated,
    Extreme,
    Unknown,
}

impl std::fmt::Display for AttributionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "Normal"),
            Self::Elevated => write!(f, "Elevated"),
            Self::Extreme => write!(f, "Extreme"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Observation: raw facts extracted from the frozen Research Platform 1.0.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Observation {
    pub state: String,
    pub signal_summary: String,
    pub breadth_pct: Option<f64>,
    pub liquidity_score: Option<f64>,
    pub macro_regime: Option<String>,
}

/// Evidence: a recurring historical pattern attached to a condition.
///
/// Evidence stores facts (matched dates, forward returns, window) and derived
/// statistics (positive ratio, median forward return). Derived fields are
/// recomputable from the raw facts; they are kept for convenience but are not
/// interpretations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Evidence {
    pub occurrences: usize,
    pub history_window: String,
    pub positive_ratio: f64,
    pub median_forward_return: f64,
    pub median_max_drawdown: f64,
    pub matched_dates: Vec<NaiveDate>,
    pub forward_returns: Vec<f64>,
    pub max_drawdowns: Vec<f64>,
    pub window_start: NaiveDate,
    pub window_end: NaiveDate,
}

impl Evidence {
    /// Build an Evidence value from raw facts.
    ///
    /// `matched_dates`, `forward_returns`, and `max_drawdowns` must have the same
    /// length and be aligned by index. The window is the overall historical window
    /// used to find the matches, not the min/max of the matched dates.
    pub fn from_facts(
        matched_dates: Vec<NaiveDate>,
        forward_returns: Vec<f64>,
        max_drawdowns: Vec<f64>,
        window_start: NaiveDate,
        window_end: NaiveDate,
    ) -> Self {
        let occurrences = forward_returns.len();
        let positive_ratio = if occurrences > 0 {
            forward_returns.iter().filter(|&&r| r > 0.0).count() as f64 / occurrences as f64
        } else {
            0.0
        };
        let mut sorted_returns = forward_returns.clone();
        sorted_returns.sort_by(|a, b| a.total_cmp(b));
        let median_forward_return = if occurrences > 0 {
            percentile_value(&sorted_returns, 0.50)
        } else {
            0.0
        };
        let mut sorted_drawdowns = max_drawdowns.clone();
        sorted_drawdowns.sort_by(|a, b| a.total_cmp(b));
        let median_max_drawdown = if !sorted_drawdowns.is_empty() {
            percentile_value(&sorted_drawdowns, 0.50)
        } else {
            0.0
        };
        Self {
            occurrences,
            history_window: format!("{} ~ {}", window_start, window_end),
            positive_ratio,
            median_forward_return,
            median_max_drawdown,
            matched_dates,
            forward_returns,
            max_drawdowns,
            window_start,
            window_end,
        }
    }
}

impl Default for Evidence {
    fn default() -> Self {
        Self {
            occurrences: 0,
            history_window: String::new(),
            positive_ratio: 0.0,
            median_forward_return: 0.0,
            median_max_drawdown: 0.0,
            matched_dates: Vec::new(),
            forward_returns: Vec::new(),
            max_drawdowns: Vec::new(),
            window_start: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            window_end: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
        }
    }
}

fn percentile_value(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = (p * (values.len() - 1) as f64).round() as usize;
    values[idx.clamp(0, values.len() - 1)]
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
        }
    }
}

/// Full research explanation output produced by the framework.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchExplanation {
    pub condition: String,
    pub scope: String,
    pub date: NaiveDate,
    pub observation: Observation,
    pub evidence: Evidence,
    pub attributions: Vec<AttributionResult>,
    pub hypothesis: String,
    pub confidence: Confidence,
    pub limitations: Vec<String>,
    pub next_validation: String,
}

/// Registry of attribution dimensions.
///
/// New dimensions are registered without modifying the framework core.
#[derive(Default)]
pub struct AttributionRegistry {
    dimensions: Vec<Box<dyn AttributionDimension>>,
}

impl AttributionRegistry {
    pub fn new() -> Self {
        Self {
            dimensions: Vec::new(),
        }
    }

    pub fn register<D: AttributionDimension + 'static>(&mut self, dim: D) {
        self.dimensions.push(Box::new(dim));
    }

    pub fn evaluate(&self, input: &AttributionInput) -> Vec<AttributionResult> {
        self.dimensions.iter().map(|d| d.evaluate(input)).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.dimensions.is_empty()
    }

    pub fn dimension_names(&self) -> Vec<&'static str> {
        self.dimensions.iter().map(|d| d.name()).collect()
    }
}

/// Framework helper that builds an explanation from a single observation.
///
/// In Phase 1 this is intentionally minimal: it assembles the structure and
/// leaves hypothesis/confidence to the caller. As concrete dimensions are
/// registered, the same function produces richer attribution output.
pub fn build_explanation(
    condition: impl Into<String>,
    scope: impl Into<String>,
    date: NaiveDate,
    observation: Observation,
    evidence: Evidence,
    registry: &AttributionRegistry,
    limitations: Vec<String>,
    next_validation: impl Into<String>,
) -> ResearchExplanation {
    let condition: String = condition.into();
    let scope: String = scope.into();
    let next_validation: String = next_validation.into();

    let attributions = if registry.is_empty() {
        vec![AttributionResult::unknown(
            "framework",
            "No attribution dimensions registered yet (Phase 1 architecture only).",
        )]
    } else {
        let input = AttributionInput {
            date,
            scope: scope.clone(),
            condition: condition.clone(),
            state: Some(observation.state.clone()),
            signal_summary: Some(observation.signal_summary.clone()),
            breadth_pct: observation.breadth_pct,
            liquidity_score: observation.liquidity_score,
            macro_regime: observation.macro_regime.clone(),
            ..AttributionInput::default()
        };
        registry.evaluate(&input)
    };

    ResearchExplanation {
        condition,
        scope,
        date,
        observation,
        evidence,
        attributions,
        hypothesis: "No hypothesis generated yet (Phase 1 architecture only).".to_string(),
        confidence: Confidence::Low,
        limitations,
        next_validation,
    }
}

/// MVP attribution registry with Breadth + Liquidity dimensions.
///
/// This is intentionally simple: it validates the framework contract without
/// introducing complex macro/theme/crowding/volatility logic.
pub fn mvp_registry() -> AttributionRegistry {
    let mut registry = AttributionRegistry::new();
    registry.register(BreadthAttribution);
    registry.register(LiquidityAttribution);
    registry
}

/// Breadth attribution dimension.
///
/// Measures whether market participation is broad enough to support the signal.
/// Thresholds are deliberately simple for the MVP and will be refined as the
/// Failure Knowledge Base grows.
pub struct BreadthAttribution;

impl AttributionDimension for BreadthAttribution {
    fn name(&self) -> &'static str {
        "Breadth"
    }

    fn evaluate(&self, input: &AttributionInput) -> AttributionResult {
        let Some(breadth) = input.breadth_pct else {
            return AttributionResult::unknown(self.name(), "Breadth data unavailable.");
        };

        match breadth {
            b if b >= 60.0 => AttributionResult {
                dimension: self.name().to_string(),
                level: AttributionLevel::Normal,
                score: Some(breadth),
                reason: "Breadth is broad (>60%); broad participation supports the signal.".to_string(),
            },
            b if b >= 40.0 => AttributionResult {
                dimension: self.name().to_string(),
                level: AttributionLevel::Elevated,
                score: Some(breadth),
                reason: "Breadth is moderate (40-60%); participation is mixed.".to_string(),
            },
            _ => AttributionResult {
                dimension: self.name().to_string(),
                level: AttributionLevel::Extreme,
                score: Some(breadth),
                reason: "Breadth is narrow/collapsed (<40%); participation is too narrow to sustain the signal.".to_string(),
            },
        }
    }
}

/// Liquidity attribution dimension.
///
/// Measures whether liquidity conditions are supportive enough for the signal
/// to continue. The liquidity score is the system's existing liquidity proxy.
pub struct LiquidityAttribution;

impl AttributionDimension for LiquidityAttribution {
    fn name(&self) -> &'static str {
        "Liquidity"
    }

    fn evaluate(&self, input: &AttributionInput) -> AttributionResult {
        let Some(score) = input.liquidity_score else {
            return AttributionResult::unknown(self.name(), "Liquidity data unavailable.");
        };

        match score {
            s if s >= 60.0 => AttributionResult {
                dimension: self.name().to_string(),
                level: AttributionLevel::Normal,
                score: Some(score),
                reason: "Liquidity is supportive (>60%); capital conditions favor continuation.".to_string(),
            },
            s if s >= 40.0 => AttributionResult {
                dimension: self.name().to_string(),
                level: AttributionLevel::Elevated,
                score: Some(score),
                reason: "Liquidity is neutral (40-60%); neither supportive nor restrictive.".to_string(),
            },
            _ => AttributionResult {
                dimension: self.name().to_string(),
                level: AttributionLevel::Extreme,
                score: Some(score),
                reason: "Liquidity is tight (<40%); capital constraints may limit the signal.".to_string(),
            },
        }
    }
}

fn level_for(dimension: &str, attributions: &[AttributionResult]) -> Option<AttributionLevel> {
    attributions
        .iter()
        .find(|a| a.dimension == dimension)
        .map(|a| a.level)
}

/// Generate a deterministic hypothesis from the attribution results.
///
/// This is a placeholder rule engine. As the Failure Knowledge Base grows,
/// hypothesis generation can be replaced with verified knowledge rules.
pub fn generate_hypothesis(condition: &str, attributions: &[AttributionResult]) -> String {
    let breadth = level_for("Breadth", attributions);
    let liquidity = level_for("Liquidity", attributions);

    match (breadth, liquidity) {
        (Some(AttributionLevel::Normal), Some(AttributionLevel::Normal)) => {
            format!(
                "{} tends to perform well when both breadth and liquidity are supportive: broad participation and healthy capital conditions may cause the State Layer to under-react.",
                condition
            )
        }
        (Some(AttributionLevel::Extreme), _) | (_, Some(AttributionLevel::Extreme)) => {
            format!(
                "{} may struggle when breadth collapses or liquidity tightens: the State Layer's caution may be justified in these conditions.",
                condition
            )
        }
        (Some(AttributionLevel::Elevated), Some(AttributionLevel::Normal)) |
        (Some(AttributionLevel::Normal), Some(AttributionLevel::Elevated)) => {
            format!(
                "{} has mixed support: one of breadth/liquidity is supportive while the other is neutral. The State Layer's caution may be partially justified.",
                condition
            )
        }
        _ => format!(
            "{} attribution is inconclusive in the current environment; more evidence is needed.",
            condition
        ),
    }
}

/// Generate a confidence level from the attribution results.
///
/// High confidence is only assigned when both dimensions agree and are in the
/// Normal range. Low confidence is assigned when either dimension is Extreme
/// or Unknown.
pub fn generate_confidence(attributions: &[AttributionResult]) -> Confidence {
    let breadth = level_for("Breadth", attributions);
    let liquidity = level_for("Liquidity", attributions);

    match (breadth, liquidity) {
        (Some(AttributionLevel::Normal), Some(AttributionLevel::Normal)) => Confidence::High,
        (Some(AttributionLevel::Elevated), Some(AttributionLevel::Normal)) |
        (Some(AttributionLevel::Normal), Some(AttributionLevel::Elevated)) |
        (Some(AttributionLevel::Elevated), Some(AttributionLevel::Elevated)) => Confidence::Medium,
        _ => Confidence::Low,
    }
}

/// Generate limitations based on which dimensions are available.
pub fn generate_limitations(attributions: &[AttributionResult]) -> Vec<String> {
    let mut limitations = vec![
        "Only Breadth and Liquidity dimensions are implemented in the MVP.".to_string(),
        "Macro, Theme, Crowding, and Volatility attributions are not yet available.".to_string(),
    ];

    for attr in attributions {
        if attr.level == AttributionLevel::Unknown {
            limitations.push(format!(
                "{} dimension returned Unknown: {}.",
                attr.dimension, attr.reason
            ));
        }
    }

    limitations
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockDimension;

    impl AttributionDimension for MockDimension {
        fn name(&self) -> &'static str {
            "Mock"
        }

        fn evaluate(&self, input: &AttributionInput) -> AttributionResult {
            AttributionResult {
                dimension: self.name().to_string(),
                level: AttributionLevel::Normal,
                score: input.breadth_pct,
                reason: "mock evaluation".to_string(),
            }
        }
    }

    #[test]
    fn empty_registry_produces_framework_placeholder() {
        let registry = AttributionRegistry::new();
        let observation = Observation {
            state: "DE_RISK".to_string(),
            signal_summary: "StrongBuy dominant".to_string(),
            breadth_pct: None,
            liquidity_score: None,
            macro_regime: None,
        };
        let explanation = build_explanation(
            "srd-strong",
            "GLOBAL",
            NaiveDate::from_ymd_opt(2026, 7, 9).unwrap(),
            observation,
            Evidence::default(),
            &registry,
            vec!["Phase 1 limitations".to_string()],
            "Validate after 90-day Shadow Production.",
        );

        assert_eq!(explanation.attributions.len(), 1);
        assert_eq!(explanation.attributions[0].level, AttributionLevel::Unknown);
        assert!(explanation.attributions[0].reason.contains("Phase 1"));
    }

    #[test]
    fn registered_dimensions_are_evaluated() {
        let mut registry = AttributionRegistry::new();
        registry.register(MockDimension);

        let observation = Observation {
            state: "DE_RISK".to_string(),
            signal_summary: "StrongBuy dominant".to_string(),
            breadth_pct: Some(60.0),
            liquidity_score: None,
            macro_regime: None,
        };
        let explanation = build_explanation(
            "srd-strong",
            "GLOBAL",
            NaiveDate::from_ymd_opt(2026, 7, 9).unwrap(),
            observation,
            Evidence::default(),
            &registry,
            vec![],
            "",
        );

        assert_eq!(explanation.attributions.len(), 1);
        assert_eq!(explanation.attributions[0].dimension, "Mock");
        assert_eq!(explanation.attributions[0].score, Some(60.0));
    }

    #[test]
    fn breadth_attribution_buckets() {
        let dim = BreadthAttribution;
        let normal = dim.evaluate(&AttributionInput {
            breadth_pct: Some(65.0),
            ..AttributionInput::default()
        });
        assert_eq!(normal.level, AttributionLevel::Normal);

        let elevated = dim.evaluate(&AttributionInput {
            breadth_pct: Some(50.0),
            ..AttributionInput::default()
        });
        assert_eq!(elevated.level, AttributionLevel::Elevated);

        let extreme = dim.evaluate(&AttributionInput {
            breadth_pct: Some(25.0),
            ..AttributionInput::default()
        });
        assert_eq!(extreme.level, AttributionLevel::Extreme);
    }

    #[test]
    fn liquidity_attribution_buckets() {
        let dim = LiquidityAttribution;
        let normal = dim.evaluate(&AttributionInput {
            liquidity_score: Some(70.0),
            ..AttributionInput::default()
        });
        assert_eq!(normal.level, AttributionLevel::Normal);

        let extreme = dim.evaluate(&AttributionInput {
            liquidity_score: Some(30.0),
            ..AttributionInput::default()
        });
        assert_eq!(extreme.level, AttributionLevel::Extreme);
    }

    #[test]
    fn mvp_registry_generates_explanation_with_hypothesis() {
        let registry = mvp_registry();
        let attributions = registry.evaluate(&AttributionInput {
            breadth_pct: Some(65.0),
            liquidity_score: Some(70.0),
            ..AttributionInput::default()
        });
        let hypothesis = generate_hypothesis("srd-strong", &attributions);
        let confidence = generate_confidence(&attributions);

        assert!(hypothesis.contains("State Layer"));
        assert_eq!(confidence, Confidence::High);
    }
}

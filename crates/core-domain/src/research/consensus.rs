use serde::{Deserialize, Serialize};

/// High-level research interpretation of the current market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusBias {
    Constructive,
    Neutral,
    Conflicted,
    Fragile,
    Cautious,
}

/// Confidence in the current consensus bias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// A single evidence item that contributed to the consensus, with its weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedEvidence {
    pub source: String,
    pub description: String,
    pub weight: f64,
}

/// Output of the Evidence Aggregator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusSummary {
    /// Consensus algorithm version. Incremented when the aggregation formula,
    /// weights, or thresholds change in a non-backwards-compatible way.
    pub version: u32,
    pub bias: ConsensusBias,
    pub confidence: Confidence,
    pub aggregate_score: f64,
    pub supporting_evidence: Vec<WeightedEvidence>,
    pub contradicting_evidence: Vec<WeightedEvidence>,
    pub summary: String,
}

/// Weights assigned to each evidence source.
///
/// Weights are applied to normalized inputs. A negative weight means
/// higher readings from that source push the consensus toward caution/conflict.
#[derive(Debug, Clone, Copy)]
pub struct EvidenceWeights {
    pub signal: f64,
    pub stretch: f64,
    pub confirmation: f64,
    pub recovery: f64,
    pub analogues: f64,
}

impl Default for EvidenceWeights {
    fn default() -> Self {
        Self {
            signal: 0.20,
            stretch: -0.18,
            confirmation: 0.24,
            recovery: 0.28,
            analogues: 0.10,
        }
    }
}

/// Thresholds for mapping an aggregate score to a `ConsensusBias`.
#[derive(Debug, Clone, Copy)]
pub struct BiasThresholds {
    /// Scores above this threshold are classified as Constructive.
    pub constructive: f64,
    /// Scores above this threshold (and below constructive) are Neutral.
    /// Scores below this threshold may be Fragile or Cautious.
    pub neutral_lower: f64,
    /// Scores above this threshold (and below neutral_lower) are Fragile.
    /// Scores below this threshold are Cautious.
    pub fragile_lower: f64,
}

impl Default for BiasThresholds {
    fn default() -> Self {
        Self {
            constructive: 0.1,
            neutral_lower: -0.1,
            fragile_lower: -0.3,
        }
    }
}

/// Thresholds for mapping an aggregate score to a `Confidence` level.
#[derive(Debug, Clone, Copy)]
pub struct ConfidenceThresholds {
    /// Absolute scores at or above this value map to High confidence.
    pub high: f64,
    /// Absolute scores at or above this value (and below high) map to Medium confidence.
    pub medium: f64,
}

impl Default for ConfidenceThresholds {
    fn default() -> Self {
        Self {
            high: 0.5,
            medium: 0.2,
        }
    }
}

/// Thresholds for detecting a Conflicted bias (strong disagreement on both sides
/// with a near-zero net score).
#[derive(Debug, Clone, Copy)]
pub struct ConflictedThresholds {
    /// Minimum total positive contribution to consider conflict significant.
    pub min_positive_weight: f64,
    /// Minimum total negative contribution (absolute) to consider conflict significant.
    pub min_negative_weight: f64,
    /// Maximum absolute net score for the conflict to be considered balanced.
    pub max_net_score: f64,
}

impl Default for ConflictedThresholds {
    fn default() -> Self {
        Self {
            min_positive_weight: 0.25,
            min_negative_weight: 0.25,
            max_net_score: 0.15,
        }
    }
}

/// Complete configuration for the Consensus aggregator.
#[derive(Debug, Clone, Copy)]
pub struct ConsensusConfig {
    /// Algorithm version written into every `ConsensusSummary`. Bump when
    /// weights, thresholds, or aggregation semantics change incompatibly.
    pub version: u32,
    pub weights: EvidenceWeights,
    pub bias_thresholds: BiasThresholds,
    pub confidence_thresholds: ConfidenceThresholds,
    pub conflicted_thresholds: ConflictedThresholds,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            version: 1,
            weights: EvidenceWeights::default(),
            bias_thresholds: BiasThresholds::default(),
            confidence_thresholds: ConfidenceThresholds::default(),
            conflicted_thresholds: ConflictedThresholds::default(),
        }
    }
}

/// Normalized evidence readings in [-1, 1].
///
/// For each source:
/// - `signal`: higher = more constructive
/// - `stretch`: higher = more stretched/cautious
/// - `confirmation`: higher = stronger confirmation
/// - `recovery`: higher = more recovered
/// - `analogues`: positive = constructive historical outlook
#[derive(Debug, Clone, Default)]
pub struct EvidenceInput {
    pub signal: Option<f64>,
    pub stretch: Option<f64>,
    pub confirmation: Option<f64>,
    pub recovery: Option<f64>,
    pub analogues: Option<f64>,
}

/// Aggregate evidence into a research consensus.
pub fn consensus(input: EvidenceInput, config: &ConsensusConfig) -> ConsensusSummary {
    let weights = &config.weights;
    let mut items: Vec<(&str, Option<f64>, f64, String)> = Vec::new();
    items.push(("Signal", input.signal, weights.signal, signal_description(input.signal)));
    items.push(("Stretch", input.stretch, weights.stretch, stretch_description(input.stretch)));
    items.push(("Confirmation", input.confirmation, weights.confirmation, confirmation_description(input.confirmation)));
    items.push(("Recovery", input.recovery, weights.recovery, recovery_description(input.recovery)));
    items.push(("Analogues", input.analogues, weights.analogues, analogues_description(input.analogues)));

    let mut supporting_evidence = Vec::new();
    let mut contradicting_evidence = Vec::new();
    let mut aggregate_score = 0.0;
    let mut total_positive_weight = 0.0;
    let mut total_negative_weight = 0.0;

    for (source, value_opt, weight, description) in items {
        let Some(value) = value_opt else {
            continue;
        };
        let clamped = value.clamp(-1.0, 1.0);
        let contribution = weight * clamped;
        aggregate_score += contribution;

        let evidence = WeightedEvidence {
            source: source.to_string(),
            description,
            weight: contribution,
        };

        if contribution > 0.0 {
            supporting_evidence.push(evidence);
            total_positive_weight += contribution;
        } else if contribution < 0.0 {
            contradicting_evidence.push(evidence);
            total_negative_weight += contribution.abs();
        }
    }

    supporting_evidence.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap());
    contradicting_evidence.sort_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap());

    let bias = classify_bias(
        aggregate_score,
        total_positive_weight,
        total_negative_weight,
        &config.bias_thresholds,
        &config.conflicted_thresholds,
    );
    let confidence = classify_confidence(
        aggregate_score,
        total_positive_weight,
        total_negative_weight,
        bias,
        &config.confidence_thresholds,
    );
    let summary = build_summary(bias, confidence, &supporting_evidence, &contradicting_evidence);

    ConsensusSummary {
        version: config.version,
        bias,
        confidence,
        aggregate_score,
        supporting_evidence,
        contradicting_evidence,
        summary,
    }
}

fn classify_bias(
    score: f64,
    positive_weight: f64,
    negative_weight: f64,
    bias_thresholds: &BiasThresholds,
    conflicted_thresholds: &ConflictedThresholds,
) -> ConsensusBias {
    // Strong disagreement on both sides with a near-zero net score = conflicted.
    if positive_weight >= conflicted_thresholds.min_positive_weight
        && negative_weight >= conflicted_thresholds.min_negative_weight
        && score.abs() <= conflicted_thresholds.max_net_score
    {
        return ConsensusBias::Conflicted;
    }

    if score > bias_thresholds.constructive {
        ConsensusBias::Constructive
    } else if score > bias_thresholds.neutral_lower {
        ConsensusBias::Neutral
    } else if score > bias_thresholds.fragile_lower {
        ConsensusBias::Fragile
    } else {
        ConsensusBias::Cautious
    }
}

fn classify_confidence(
    score: f64,
    positive_weight: f64,
    negative_weight: f64,
    bias: ConsensusBias,
    confidence_thresholds: &ConfidenceThresholds,
) -> Confidence {
    let abs_score = score.abs();
    let conflict = positive_weight >= conflicted_thresholds().min_positive_weight
        && negative_weight >= conflicted_thresholds().min_negative_weight;

    if bias == ConsensusBias::Conflicted {
        return if abs_score > conflicted_thresholds().max_net_score {
            Confidence::Medium
        } else {
            Confidence::High
        };
    }

    if abs_score >= confidence_thresholds.high {
        Confidence::High
    } else if abs_score >= confidence_thresholds.medium || conflict {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

fn conflicted_thresholds() -> ConflictedThresholds {
    ConflictedThresholds::default()
}

fn build_summary(
    bias: ConsensusBias,
    confidence: Confidence,
    supporting: &[WeightedEvidence],
    contradicting: &[WeightedEvidence],
) -> String {
    let bias_label = match bias {
        ConsensusBias::Constructive => "Constructive",
        ConsensusBias::Neutral => "Neutral",
        ConsensusBias::Conflicted => "Conflicted",
        ConsensusBias::Fragile => "Fragile",
        ConsensusBias::Cautious => "Cautious",
    };
    let confidence_label = match confidence {
        Confidence::Low => "Low",
        Confidence::Medium => "Medium",
        Confidence::High => "High",
    };

    let mut parts = Vec::new();
    parts.push(format!(
        "Research view is {} with {} confidence.",
        bias_label, confidence_label
    ));

    if !supporting.is_empty() {
        let names: Vec<&str> = supporting.iter().map(|e| e.source.as_str()).collect();
        parts.push(format!("Supporting: {}.", names.join(", ")));
    }

    if !contradicting.is_empty() {
        let names: Vec<&str> = contradicting.iter().map(|e| e.source.as_str()).collect();
        parts.push(format!("Contradicting: {}.", names.join(", ")));
    }

    parts.join(" ")
}

fn signal_description(value: Option<f64>) -> String {
    match value {
        None => "No signal reading".to_string(),
        Some(v) if v >= 0.5 => "Signal constructive".to_string(),
        Some(v) if v >= 0.1 => "Signal moderately constructive".to_string(),
        Some(v) if v > -0.1 => "Signal neutral".to_string(),
        Some(v) if v > -0.5 => "Signal moderately cautious".to_string(),
        _ => "Signal cautious".to_string(),
    }
}

fn stretch_description(value: Option<f64>) -> String {
    match value {
        None => "No stretch reading".to_string(),
        Some(v) if v >= 0.5 => "Stretch extreme".to_string(),
        Some(v) if v >= 0.1 => "Stretch elevated".to_string(),
        Some(v) if v > -0.1 => "Stretch neutral".to_string(),
        Some(v) if v > -0.5 => "Stretch below normal".to_string(),
        _ => "Stretch compressed".to_string(),
    }
}

fn confirmation_description(value: Option<f64>) -> String {
    match value {
        None => "No confirmation reading".to_string(),
        Some(v) if v >= 0.5 => "Confirmation strong".to_string(),
        Some(v) if v >= 0.1 => "Confirmation moderate".to_string(),
        Some(v) if v > -0.1 => "Confirmation neutral".to_string(),
        Some(v) if v > -0.5 => "Confirmation weak".to_string(),
        _ => "Confirmation very weak".to_string(),
    }
}

fn recovery_description(value: Option<f64>) -> String {
    match value {
        None => "No recovery reading".to_string(),
        Some(v) if v >= 0.5 => "Recovery strong".to_string(),
        Some(v) if v >= 0.1 => "Recovery improving".to_string(),
        Some(v) if v > -0.1 => "Recovery neutral".to_string(),
        Some(v) if v > -0.5 => "Recovery weak".to_string(),
        _ => "Recovery very weak".to_string(),
    }
}

fn analogues_description(value: Option<f64>) -> String {
    match value {
        None => "No analogue evidence".to_string(),
        Some(v) if v >= 0.3 => "Historical analogues constructive".to_string(),
        Some(v) if v > -0.1 => "Historical analogues neutral".to_string(),
        Some(v) if v > -0.3 => "Historical analogues mixed".to_string(),
        _ => "Historical analogues cautious".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> ConsensusConfig {
        ConsensusConfig::default()
    }

    #[test]
    fn all_positive_evidence_is_constructive_high_confidence() {
        let input = EvidenceInput {
            signal: Some(0.8),
            stretch: Some(-0.2),
            confirmation: Some(0.7),
            recovery: Some(0.9),
            analogues: Some(0.4),
        };
        let summary = consensus(input, &default_config());
        assert_eq!(summary.bias, ConsensusBias::Constructive);
        assert_eq!(summary.confidence, Confidence::High);
        assert!(summary.aggregate_score > 0.0);
        assert!(!summary.supporting_evidence.is_empty());
        assert_eq!(summary.version, 1);
    }

    #[test]
    fn all_negative_evidence_is_cautious_high_confidence() {
        let input = EvidenceInput {
            signal: Some(-0.8),
            stretch: Some(0.8),
            confirmation: Some(-0.6),
            recovery: Some(-0.9),
            analogues: Some(-0.5),
        };
        let summary = consensus(input, &default_config());
        assert_eq!(summary.bias, ConsensusBias::Cautious);
        assert_eq!(summary.confidence, Confidence::High);
        assert!(summary.aggregate_score < 0.0);
        assert!(!summary.contradicting_evidence.is_empty());
    }

    #[test]
    fn balanced_opposing_evidence_is_conflicted() {
        let input = EvidenceInput {
            signal: Some(-0.8),
            stretch: Some(0.9),
            confirmation: Some(0.8),
            recovery: Some(0.8),
            analogues: Some(-0.5),
        };
        let summary = consensus(input, &default_config());
        assert_eq!(summary.bias, ConsensusBias::Conflicted);
    }

    #[test]
    fn neutral_evidence_is_neutral_low_confidence() {
        let input = EvidenceInput {
            signal: Some(0.0),
            stretch: Some(0.0),
            confirmation: Some(0.0),
            recovery: Some(0.0),
            analogues: Some(0.0),
        };
        let summary = consensus(input, &default_config());
        assert_eq!(summary.bias, ConsensusBias::Neutral);
        assert_eq!(summary.confidence, Confidence::Low);
        assert!(summary.aggregate_score.abs() < 0.01);
    }

    #[test]
    fn missing_evidence_is_ignored() {
        let input = EvidenceInput {
            signal: Some(0.8),
            stretch: None,
            confirmation: None,
            recovery: None,
            analogues: None,
        };
        let summary = consensus(input, &default_config());
        assert_eq!(summary.supporting_evidence.len(), 1);
        assert_eq!(summary.supporting_evidence[0].source, "Signal");
    }

    #[test]
    fn summary_contains_bias_and_evidence_names() {
        let input = EvidenceInput {
            signal: Some(0.6),
            stretch: Some(-0.1),
            confirmation: Some(0.4),
            recovery: Some(0.3),
            analogues: Some(0.0),
        };
        let summary = consensus(input, &default_config());
        assert!(summary.summary.contains("Constructive"));
        assert!(summary.summary.contains("Supporting"));
    }

    #[test]
    fn config_version_is_written_to_summary() {
        let mut config = default_config();
        config.version = 2;
        let input = EvidenceInput::default();
        let summary = consensus(input, &config);
        assert_eq!(summary.version, 2);
    }

    #[test]
    fn custom_thresholds_change_bias() {
        let input = EvidenceInput {
            signal: Some(0.15),
            stretch: Some(-0.1),
            confirmation: Some(0.0),
            recovery: Some(0.0),
            analogues: Some(0.0),
        };
        let mut config = default_config();
        config.bias_thresholds.constructive = 0.2;
        let summary = consensus(input, &config);
        assert_eq!(summary.bias, ConsensusBias::Neutral);
    }
}

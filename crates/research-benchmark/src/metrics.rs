use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Top-level benchmark report for a skill across multiple providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub skill_name: String,
    pub runs_per_provider: usize,

    /// Field-level consistency across providers (0-1).
    pub agreement_score: f64,

    /// Correctness against labeled ground-truth, if available (0-1).
    pub ground_truth_score: Option<f64>,

    /// Same provider, same input → same output stability (0-1).
    pub stability_score: f64,

    /// Token cost efficiency (0-1, higher is cheaper/better).
    pub cost_score: f64,

    /// Latency efficiency (0-1, higher is faster/better).
    pub latency_score: f64,

    /// Schema validation pass rate (0-1).
    pub schema_pass_rate: f64,

    /// Per-provider detailed scores.
    pub provider_ranking: Vec<ProviderScore>,

    /// Fields where providers disagree.
    pub divergence_points: Vec<DivergencePoint>,
}

/// Aggregated score for a single provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderScore {
    pub provider: String,
    pub model: String,

    /// Accuracy on key output fields (0-1).
    pub regime_accuracy: f64,

    /// Cost efficiency relative to peers (0-1).
    pub cost_efficiency: f64,

    /// Output stability across runs (0-1).
    pub stability: f64,

    /// Overall weighted score (0-1).
    pub overall: f64,
}

/// A single field where providers produced different values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DivergencePoint {
    pub field: String,
    pub values: Vec<(String, Value)>, // (provider, value)
    pub agreement_ratio: f64,         // providers agreeing on mode / total
}

/// Raw record of one execution on one provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRun {
    pub provider: String,
    pub run_number: usize,
    pub output_json: Value,
    pub latency_ms: u64,
    pub tokens_used: usize,
    pub schema_valid: bool,
}

// ------------------------------------------------------------------
// Legacy types kept for backward compat during V4 transition
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub skill_name: String,
    pub consistency: f64,
    pub hallucination_score: f64,
    pub schema_pass_rate: f64,
    pub semantic_validity: f64,
    pub latency_ms: u64,
    pub token_cost: usize,
    pub runs: usize,
}

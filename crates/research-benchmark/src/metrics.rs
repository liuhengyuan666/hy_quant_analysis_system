use serde::{Deserialize, Serialize};

/// Metrics from a benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub skill_name: String,
    pub consistency: f64,        // 0-1, same input → same output
    pub hallucination_score: f64, // 0-1, lower is better
    pub schema_pass_rate: f64,   // 0-1
    pub semantic_validity: f64,  // 0-1
    pub latency_ms: u64,
    pub token_cost: usize,
    pub runs: usize,
}

/// Single benchmark run result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRun {
    pub run_number: usize,
    pub output_json: String,
    pub schema_valid: bool,
    pub semantic_valid: bool,
    pub latency_ms: u64,
    pub tokens_used: usize,
}

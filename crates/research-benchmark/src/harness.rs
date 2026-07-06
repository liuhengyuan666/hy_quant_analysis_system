use std::collections::HashMap;

use llm_context::ResearchContext;
use research_skills::InferenceConfig;

use super::metrics::{
    BenchmarkReport, DivergencePoint, ProviderRun, ProviderScore,
};

/// Configuration for a single LLM provider in a benchmark suite.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub timeout_secs: u64,
}

/// A benchmark suite: one action, one context, multiple providers, N runs each.
/// TODO: Research Layer 重构后，benchmark 需要重新设计（ADR-074）。
#[derive(Debug, Clone)]
pub struct BenchmarkSuite {
    pub action: String,
    pub context: ResearchContext,
    pub providers: Vec<ProviderConfig>,
    pub runs_per_provider: usize,
    pub inference: InferenceConfig,
}

/// Harness for executing benchmark suites.
pub struct BenchmarkHarness;

impl BenchmarkHarness {
    /// Run a full benchmark suite and produce a report.
    /// TODO: 适配新的 ResearchAction 架构。
    pub async fn run_suite(_suite: &BenchmarkSuite) -> anyhow::Result<BenchmarkReport> {
        // Research Layer 重构后（ADR-074），benchmark 需要重新设计。
        // 旧 Skill/Executor/Schema 框架已删除，新的 Prompt-only 架构需要不同的 benchmark 方法。
        let all_runs: Vec<ProviderRun> = Vec::new();
        let report = compute_report("placeholder", &all_runs, 0);
        Ok(report)
    }
}

// ------------------------------------------------------------------
// Report computation
// ------------------------------------------------------------------

fn compute_report(
    skill_name: &str,
    runs: &[ProviderRun],
    runs_per_provider: usize,
) -> BenchmarkReport {
    let providers = collect_providers(runs);

    let agreement_score = compute_agreement_score(runs, &providers);
    let stability_score = compute_stability_score(runs, &providers, runs_per_provider);
    let (cost_score, latency_score) = compute_efficiency_scores(runs, &providers);
    let schema_pass_rate = compute_schema_pass_rate(runs);
    let provider_ranking = compute_provider_ranking(runs, &providers, runs_per_provider);
    let divergence_points = find_divergence_points(runs, &providers);

    BenchmarkReport {
        skill_name: skill_name.to_string(),
        runs_per_provider,
        agreement_score,
        ground_truth_score: None, // requires labeled dataset
        stability_score,
        cost_score,
        latency_score,
        schema_pass_rate,
        provider_ranking,
        divergence_points,
    }
}

fn collect_providers(runs: &[ProviderRun]) -> Vec<String> {
    let mut providers: Vec<String> = runs
        .iter()
        .map(|r| r.provider.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    providers.sort();
    providers
}

// ------------------------------------------------------------------
// Agreement: how often do providers agree on key fields?
// ------------------------------------------------------------------

fn compute_agreement_score(runs: &[ProviderRun], providers: &[String]) -> f64 {
    if providers.len() < 2 {
        return 1.0;
    }

    // Compare first run of each provider
    let first_runs: Vec<_> = providers
        .iter()
        .filter_map(|p| runs.iter().find(|r| r.provider == *p && r.run_number == 1))
        .collect();

    if first_runs.len() < 2 {
        return 1.0;
    }

    let fields = collect_leaf_fields(&first_runs[0].output_json, "");
    if fields.is_empty() {
        return 0.0;
    }

    let mut agreements = 0usize;
    let mut total = 0usize;

    for (field, ref_value) in &fields {
        total += 1;
        let all_match = first_runs[1..].iter().all(|run| {
            let run_fields = collect_leaf_fields(&run.output_json, "");
            run_fields.get(field).map(|v| json_eq(v, ref_value)).unwrap_or(false)
        });
        if all_match {
            agreements += 1;
        }
    }

    if total == 0 {
        0.0
    } else {
        agreements as f64 / total as f64
    }
}

// ------------------------------------------------------------------
// Stability: same provider, same input → same output?
// ------------------------------------------------------------------

fn compute_stability_score(
    runs: &[ProviderRun],
    providers: &[String],
    runs_per_provider: usize,
) -> f64 {
    if runs_per_provider < 2 {
        return 1.0;
    }

    let mut total_stability = 0.0;
    let mut provider_count = 0usize;

    for provider in providers {
        let provider_runs: Vec<_> = runs
            .iter()
            .filter(|r| r.provider == *provider)
            .collect();
        if provider_runs.len() < 2 {
            continue;
        }

        let fields = collect_leaf_fields(&provider_runs[0].output_json, "");
        if fields.is_empty() {
            continue;
        }

        let mut agreements = 0usize;
        let mut total = 0usize;

        for (field, ref_value) in &fields {
            total += 1;
            let all_match = provider_runs[1..].iter().all(|run| {
                let run_fields = collect_leaf_fields(&run.output_json, "");
                run_fields.get(field).map(|v| json_eq(v, ref_value)).unwrap_or(false)
            });
            if all_match {
                agreements += 1;
            }
        }

        if total > 0 {
            total_stability += agreements as f64 / total as f64;
            provider_count += 1;
        }
    }

    if provider_count == 0 {
        1.0
    } else {
        total_stability / provider_count as f64
    }
}

// ------------------------------------------------------------------
// Efficiency: cost & latency relative to best performer
// ------------------------------------------------------------------

fn compute_schema_pass_rate(runs: &[ProviderRun]) -> f64 {
    if runs.is_empty() {
        return 1.0;
    }
    let valid_count = runs.iter().filter(|r| r.schema_valid).count();
    valid_count as f64 / runs.len() as f64
}

fn compute_efficiency_scores(runs: &[ProviderRun], providers: &[String]) -> (f64, f64) {
    let mut avg_cost: HashMap<String, f64> = HashMap::new();
    let mut avg_latency: HashMap<String, f64> = HashMap::new();

    for provider in providers {
        let provider_runs: Vec<_> = runs.iter().filter(|r| r.provider == *provider).collect();
        if provider_runs.is_empty() {
            continue;
        }
        let total_cost: usize = provider_runs.iter().map(|r| r.tokens_used).sum();
        let total_latency: u64 = provider_runs.iter().map(|r| r.latency_ms).sum();
        avg_cost.insert(
            provider.clone(),
            total_cost as f64 / provider_runs.len() as f64,
        );
        avg_latency.insert(
            provider.clone(),
            total_latency as f64 / provider_runs.len() as f64,
        );
    }

    let min_cost = avg_cost.values().cloned().fold(f64::MAX, f64::min);
    let min_latency = avg_latency.values().cloned().fold(f64::MAX, f64::min);

    if min_cost == 0.0 || min_latency == 0.0 {
        return (1.0, 1.0);
    }

    let cost_score = avg_cost.values().map(|c| min_cost / c).sum::<f64>() / avg_cost.len() as f64;
    let latency_score = avg_latency.values().map(|l| min_latency / l).sum::<f64>()
        / avg_latency.len() as f64;

    (cost_score, latency_score)
}

// ------------------------------------------------------------------
// Provider ranking: per-provider accuracy + efficiency + stability
// ------------------------------------------------------------------

fn compute_provider_ranking(
    runs: &[ProviderRun],
    providers: &[String],
    runs_per_provider: usize,
) -> Vec<ProviderScore> {
    let mut scores = Vec::new();

    for provider in providers {
        let provider_runs: Vec<_> = runs.iter().filter(|r| r.provider == *provider).collect();
        if provider_runs.is_empty() {
            continue;
        }

        let avg_tokens = provider_runs.iter().map(|r| r.tokens_used).sum::<usize>() as f64
            / provider_runs.len() as f64;
        let avg_latency = provider_runs.iter().map(|r| r.latency_ms).sum::<u64>() as f64
            / provider_runs.len() as f64;

        // Regime accuracy: placeholder - parse regime_state field if present
        let regime_accuracy = compute_regime_accuracy(&provider_runs);

        // Stability for this provider
        let stability = if runs_per_provider >= 2 {
            let fields = collect_leaf_fields(&provider_runs[0].output_json, "");
            let mut agreements = 0usize;
            let mut total = 0usize;
            for (field, ref_value) in &fields {
                total += 1;
                let all_match = provider_runs[1..].iter().all(|run| {
                    let run_fields = collect_leaf_fields(&run.output_json, "");
                    run_fields.get(field).map(|v| json_eq(v, ref_value)).unwrap_or(false)
                });
                if all_match {
                    agreements += 1;
                }
            }
            if total > 0 {
                agreements as f64 / total as f64
            } else {
                1.0
            }
        } else {
            1.0
        };

        // Cost efficiency: lower token usage = higher score (relative to 4096 context)
        let cost_efficiency = (1.0 - (avg_tokens / 4096.0)).clamp(0.0, 1.0);

        // Latency efficiency: < 5s = 1.0, > 30s = 0.0
        let latency_efficiency = (1.0 - (avg_latency / 30000.0)).clamp(0.0, 1.0);

        let overall = (regime_accuracy * 0.4
            + cost_efficiency * 0.2
            + latency_efficiency * 0.2
            + stability * 0.2)
            .clamp(0.0, 1.0);

        scores.push(ProviderScore {
            provider: provider.clone(),
            model: "unknown".to_string(), // TODO: populate from config
            regime_accuracy,
            cost_efficiency,
            stability,
            overall,
        });
    }

    scores.sort_by(|a, b| b.overall.partial_cmp(&a.overall).unwrap());
    scores
}

fn compute_regime_accuracy(provider_runs: &[&ProviderRun]) -> f64 {
    // Placeholder: in V4.1, compare against labeled ground truth
    // For now, check if regime_state field is present and valid
    let valid_count = provider_runs
        .iter()
        .filter(|r| {
            r.output_json
                .get("regime_state")
                .and_then(|v| v.as_str())
                .map(|s| matches!(s, "risk_on" | "neutral" | "risk_off" | "de_risk"))
                .unwrap_or(false)
        })
        .count();

    if provider_runs.is_empty() {
        0.0
    } else {
        valid_count as f64 / provider_runs.len() as f64
    }
}

// ------------------------------------------------------------------
// Divergence points
// ------------------------------------------------------------------

fn find_divergence_points(runs: &[ProviderRun], providers: &[String]) -> Vec<DivergencePoint> {
    let first_runs: Vec<_> = providers
        .iter()
        .filter_map(|p| runs.iter().find(|r| r.provider == *p && r.run_number == 1))
        .collect();

    if first_runs.len() < 2 {
        return Vec::new();
    }

    let fields = collect_leaf_fields(&first_runs[0].output_json, "");
    let mut divergences = Vec::new();

    for (field, _) in &fields {
        let values: Vec<(String, serde_json::Value)> = first_runs
            .iter()
            .filter_map(|r| {
                let run_fields = collect_leaf_fields(&r.output_json, "");
                run_fields
                    .get(field)
                    .cloned()
                    .map(|v| (r.provider.clone(), v))
            })
            .collect();

        if values.len() < 2 {
            continue;
        }

        // Find mode (most common value)
        let mut counts: HashMap<String, usize> = HashMap::new();
        for (_, value) in &values {
            let key = value.to_string();
            *counts.entry(key).or_insert(0) += 1;
        }
        let mode_count = counts.values().cloned().max().unwrap_or(0);
        let agreement_ratio = mode_count as f64 / values.len() as f64;

        if agreement_ratio < 1.0 {
            divergences.push(DivergencePoint {
                field: field.clone(),
                values,
                agreement_ratio,
            });
        }
    }

    divergences
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

/// Recursively collect all leaf fields from a JSON object as dot-paths.
fn collect_leaf_fields(value: &serde_json::Value, prefix: &str) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        result.extend(collect_leaf_fields(val, &path));
                    }
                    _ => {
                        result.insert(path, val.clone());
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for (idx, val) in arr.iter().enumerate() {
                let path = format!("{}[{}]", prefix, idx);
                match val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        result.extend(collect_leaf_fields(val, &path));
                    }
                    _ => {
                        result.insert(path, val.clone());
                    }
                }
            }
        }
        _ => {
            if !prefix.is_empty() {
                result.insert(prefix.to_string(), value.clone());
            }
        }
    }
    result
}

fn json_eq(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    // Compare floats with tolerance
    match (a, b) {
        (serde_json::Value::Number(n1), serde_json::Value::Number(n2)) => {
            if let (Some(f1), Some(f2)) = (n1.as_f64(), n2.as_f64()) {
                (f1 - f2).abs() < 1e-6
            } else {
                n1 == n2
            }
        }
        _ => a == b,
    }
}

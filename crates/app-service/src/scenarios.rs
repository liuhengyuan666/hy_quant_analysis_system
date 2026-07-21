//! Strategy scenario configuration loader (RV1 Phase 2).
//!
//! Scenarios are consumption-layer preference weightings ONLY.
//! They never enter final_score / signal_label / portfolio-decision computation paths.
//! If `config/scenarios.toml` is missing, the system falls back to an equal-weight
//! default scenario and never blocks the pipeline.

use core_domain::StrategyPreferenceSnapshot;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioFile {
    pub scenarios: HashMap<String, ScenarioDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioDefinition {
    pub label: String,
    #[serde(default)]
    pub description: String,
    pub weights: HashMap<String, f64>,
}

/// Fallback used when config/scenarios.toml is absent:
/// a single equal-weight scenario covering all four strategies.
pub fn default_scenario_file() -> ScenarioFile {
    let mut weights = HashMap::new();
    weights.insert("ValueLeft".to_string(), 0.25);
    weights.insert("TrendPullback".to_string(), 0.25);
    weights.insert("TrendBreakout".to_string(), 0.25);
    weights.insert("MomentumRight".to_string(), 0.25);

    let mut scenarios = HashMap::new();
    scenarios.insert(
        "balanced".to_string(),
        ScenarioDefinition {
            label: "均衡观察".to_string(),
            description: "四类策略等权重基线（默认降级场景）".to_string(),
            weights,
        },
    );
    ScenarioFile { scenarios }
}

/// Load scenarios from `config/scenarios.toml` under the given project root.
/// Missing or malformed files fall back to the default equal-weight scenario.
pub fn load_scenarios(project_root: &Path) -> ScenarioFile {
    let path: PathBuf = project_root.join("config").join("scenarios.toml");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return default_scenario_file();
    };
    toml::from_str::<ScenarioFile>(&content).unwrap_or_else(|_| default_scenario_file())
}

/// Look up a strategy score from a preference row by strategy kind name.
/// Returns `None` for unknown kind names — unknown weights are ignored,
/// never treated as zero (so a typo cannot silently zero out a strategy).
fn score_by_kind(row: &StrategyPreferenceSnapshot, kind: &str) -> Option<f64> {
    match kind {
        "ValueLeft" => Some(row.value_left_score),
        "TrendPullback" => Some(row.trend_pullback_score),
        "TrendBreakout" => Some(row.trend_breakout_score),
        "MomentumRight" => Some(row.momentum_right_score),
        _ => None,
    }
}

/// Compute the normalized weighted scenario score for one preference row.
/// Returns `None` when no weight in the scenario maps to a known strategy.
pub fn scenario_score(
    scenario: &ScenarioDefinition,
    row: &StrategyPreferenceSnapshot,
) -> Option<f64> {
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;
    for (kind, weight) in &scenario.weights {
        if let Some(score) = score_by_kind(row, kind) {
            weighted_sum += score * weight;
            weight_total += weight;
        }
    }
    if weight_total > 0.0 {
        Some(weighted_sum / weight_total)
    } else {
        None
    }
}

/// Compute scenario scores for all scenarios in the file.
pub fn all_scenario_scores(
    file: &ScenarioFile,
    row: &StrategyPreferenceSnapshot,
) -> Vec<(String, String, f64)> {
    let mut out: Vec<(String, String, f64)> = file
        .scenarios
        .iter()
        .filter_map(|(key, def)| scenario_score(def, row).map(|s| (key.clone(), def.label.clone(), s)))
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use core_domain::StrategyKind;

    fn make_row() -> StrategyPreferenceSnapshot {
        StrategyPreferenceSnapshot {
            date: NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
            symbol: "000300".into(),
            analysis_scope: "CN".into(),
            regime_basis_scope: "CN".into(),
            value_left_score: 40.0,
            trend_pullback_score: 60.0,
            trend_breakout_score: 70.0,
            momentum_right_score: 90.0,
            best_strategy: StrategyKind::MomentumRight,
            confidence: 90.0,
            alignment: 3,
        }
    }

    #[test]
    fn scenario_score_normalizes_weights() {
        let mut weights = HashMap::new();
        weights.insert("MomentumRight".to_string(), 0.50);
        weights.insert("TrendBreakout".to_string(), 0.30);
        weights.insert("TrendPullback".to_string(), 0.20);
        let def = ScenarioDefinition {
            label: "短线动量博弈".into(),
            description: String::new(),
            weights,
        };
        let row = make_row();
        let score = scenario_score(&def, &row).unwrap();
        // (90*0.5 + 70*0.3 + 60*0.2) / 1.0 = 45+21+12 = 78
        assert!((score - 78.0).abs() < 1e-9);
    }

    #[test]
    fn scenario_score_normalizes_when_weights_do_not_sum_to_one() {
        let mut weights = HashMap::new();
        weights.insert("MomentumRight".to_string(), 2.0);
        weights.insert("ValueLeft".to_string(), 1.0);
        let def = ScenarioDefinition {
            label: "test".into(),
            description: String::new(),
            weights,
        };
        let row = make_row();
        let score = scenario_score(&def, &row).unwrap();
        // (90*2 + 40*1) / 3 = 220/3 ≈ 73.33
        assert!((score - (220.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn unknown_strategy_weights_are_ignored_not_zeroed() {
        let mut weights = HashMap::new();
        weights.insert("MomentumRight".to_string(), 1.0);
        weights.insert("NoSuchStrategy".to_string(), 100.0);
        let def = ScenarioDefinition {
            label: "test".into(),
            description: String::new(),
            weights,
        };
        let row = make_row();
        let score = scenario_score(&def, &row).unwrap();
        assert!((score - 90.0).abs() < 1e-9);
    }

    #[test]
    fn default_scenario_file_is_balanced() {
        let file = default_scenario_file();
        let row = make_row();
        let scores = all_scenario_scores(&file, &row);
        assert_eq!(scores.len(), 1);
        // (40+60+70+90)/4 = 65
        assert!((scores[0].2 - 65.0).abs() < 1e-9);
    }
}

//! Strategy perspectives orchestration (RV1 Phase 2, Route B).
//!
//! Consumption-layer only: reads the already-persisted `strategy_preference` rows,
//! attaches scenario weightings and on-demand attribution, and never touches
//! `signal_snapshot` or any decision computation path.

use crate::scenarios::{all_scenario_scores, load_scenarios, ScenarioFile};
use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::{AnalysisScope, StrategyKind, StrategyPreferenceSnapshot};
use market_store::StorageConfig;
use strategy_engine::{build_strategy_attributions, AnalysisContext};

/// One row of the strategy scoreboard.
#[derive(Debug, Clone)]
pub struct StrategyPerspectiveEntry {
    pub symbol: String,
    pub name: Option<String>,
    pub value_left_score: f64,
    pub trend_pullback_score: f64,
    pub trend_breakout_score: f64,
    pub momentum_right_score: f64,
    pub best_strategy: StrategyKind,
    pub confidence: f64,
    pub alignment: u8,
    /// (scenario_key, scenario_label, weighted_score)
    pub scenario_scores: Vec<(String, String, f64)>,
}

/// Attribution for one strategy, recomputed on demand for the detail view.
#[derive(Debug, Clone)]
pub struct StrategyAttributionView {
    pub kind: StrategyKind,
    pub recomputed_score: f64,
    pub stored_score: f64,
    /// |recomputed - stored| — should be ~0 when the pipeline is consistent.
    pub drift: f64,
    pub drivers: Vec<(String, f64, f64, String)>, // (factor, value, contribution, note)
}

/// Full detail for one symbol: stored scores + per-strategy attribution.
#[derive(Debug, Clone)]
pub struct StrategyPerspectiveDetail {
    pub entry: StrategyPerspectiveEntry,
    pub attributions: Vec<StrategyAttributionView>,
}

fn entry_from_row(
    row: &StrategyPreferenceSnapshot,
    name: Option<String>,
    scenarios: &ScenarioFile,
) -> StrategyPerspectiveEntry {
    StrategyPerspectiveEntry {
        symbol: row.symbol.clone(),
        name,
        value_left_score: row.value_left_score,
        trend_pullback_score: row.trend_pullback_score,
        trend_breakout_score: row.trend_breakout_score,
        momentum_right_score: row.momentum_right_score,
        best_strategy: row.best_strategy.clone(),
        confidence: row.confidence,
        alignment: row.alignment,
        scenario_scores: all_scenario_scores(scenarios, row),
    }
}

fn kind_score(row: &StrategyPreferenceSnapshot, kind: &StrategyKind) -> f64 {
    match kind {
        StrategyKind::ValueLeft => row.value_left_score,
        StrategyKind::TrendPullback => row.trend_pullback_score,
        StrategyKind::TrendBreakout => row.trend_breakout_score,
        StrategyKind::MomentumRight => row.momentum_right_score,
    }
}

/// Scoreboard: every symbol's four strategy scores for one date + scope.
pub fn strategy_perspectives_scoreboard(
    storage: &StorageConfig,
    scope: AnalysisScope,
    date: Option<NaiveDate>,
    project_root: &std::path::Path,
) -> Result<(NaiveDate, Vec<StrategyPerspectiveEntry>)> {
    let scope_str = scope.as_str().to_uppercase();
    let rows = market_store::fetch_strategy_preferences(storage)?;
    let scoped: Vec<&StrategyPreferenceSnapshot> = rows
        .iter()
        .filter(|row| row.analysis_scope == scope_str)
        .collect();

    let target_date = date.unwrap_or_else(|| {
        scoped
            .iter()
            .map(|row| row.date)
            .max()
            .unwrap_or_else(|| chrono::Local::now().date_naive())
    });

    let scenarios = load_scenarios(project_root);
    let names = symbol_name_map(storage);

    let mut entries: Vec<StrategyPerspectiveEntry> = scoped
        .iter()
        .filter(|row| row.date == target_date)
        .map(|row| {
            entry_from_row(
                row,
                names.get(&row.symbol).cloned(),
                &scenarios,
            )
        })
        .collect();
    entries.sort_by(|a, b| {
        b.momentum_right_score
            .total_cmp(&a.momentum_right_score)
            .then(a.symbol.cmp(&b.symbol))
    });
    Ok((target_date, entries))
}

/// Detail: one symbol's four strategy scores plus on-demand attribution.
/// Attribution is recomputed from bar + indicators + regime + rotation and
/// cross-checked against the stored scores (drift should be ~0).
pub fn strategy_perspectives_detail(
    storage: &StorageConfig,
    symbol: &str,
    scope: AnalysisScope,
    date: Option<NaiveDate>,
    project_root: &std::path::Path,
) -> Result<StrategyPerspectiveDetail> {
    let scope_str = scope.as_str().to_uppercase();
    let rows = market_store::fetch_strategy_preferences(storage)?;
    let mut symbol_rows: Vec<&StrategyPreferenceSnapshot> = rows
        .iter()
        .filter(|row| row.analysis_scope == scope_str && row.symbol == symbol)
        .collect();
    symbol_rows.sort_by(|a, b| b.date.cmp(&a.date));

    let target_date = date.unwrap_or_else(|| {
        symbol_rows
            .first()
            .map(|row| row.date)
            .unwrap_or_else(|| chrono::Local::now().date_naive())
    });

    let row = symbol_rows
        .iter()
        .find(|row| row.date == target_date)
        .with_context(|| {
            format!(
                "no strategy_preference row for {} on {} (scope {})",
                symbol, target_date, scope_str
            )
        })?;

    let scenarios = load_scenarios(project_root);
    let names = symbol_name_map(storage);
    let entry = entry_from_row(row, names.get(symbol).cloned(), &scenarios);

    // Rebuild the analysis context to recompute attribution on demand.
    let bars = market_store::fetch_daily_bars(storage, symbol)?;
    let bar = bars
        .iter()
        .find(|bar| bar.date == target_date)
        .with_context(|| format!("no daily bar for {} on {}", symbol, target_date))?;
    let indicators = market_store::fetch_indicator_snapshots(storage, symbol)?;
    let indicator = indicators
        .iter()
        .find(|row| row.date == target_date)
        .cloned()
        .with_context(|| format!("no indicator snapshot for {} on {}", symbol, target_date))?;
    let regime = market_store::fetch_latest_market_regime_on_or_before(storage, target_date, scope)?;
    let rotations = market_store::fetch_rotation_ranks_for_date(storage, target_date)?;
    let rotation = rotations
        .into_iter()
        .find(|row| row.symbol == symbol);

    let context = AnalysisContext {
        bar: bar.clone(),
        indicators: indicator,
        regime,
        rotation,
        analysis_scope: scope_str.clone(),
        regime_basis_scope: row.regime_basis_scope.clone(),
    };

    let attributions = build_strategy_attributions(&context)
        .into_iter()
        .map(|(kind, breakdown)| {
            let stored = kind_score(row, &kind);
            StrategyAttributionView {
                kind,
                recomputed_score: breakdown.score,
                stored_score: stored,
                drift: (breakdown.score - stored).abs(),
                drivers: breakdown
                    .drivers
                    .into_iter()
                    .map(|d| (d.factor, d.value, d.contribution, d.note))
                    .collect(),
            }
        })
        .collect();

    Ok(StrategyPerspectiveDetail { entry, attributions })
}

fn symbol_name_map(storage: &StorageConfig) -> std::collections::HashMap<String, String> {
    let Ok(path) = storage.universe_abspath() else {
        return std::collections::HashMap::new();
    };
    data_ingestion::load_universe(&path)
        .unwrap_or_default()
        .into_iter()
        .map(|instrument| (instrument.symbol, instrument.name))
        .collect()
}

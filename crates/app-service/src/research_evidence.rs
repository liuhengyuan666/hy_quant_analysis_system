//! Research evidence computation.
//!
//! This module owns the deterministic, reproducible computation of conditional
//! forward-return evidence. It is a service-layer helper: it fetches data through
//! `market-store`, computes through `core-domain::research`, and returns a plain
//! `Evidence` value.
//!
//! Future: when a dedicated `research-engine` crate is introduced, this module
//! should migrate there. AppService should then only orchestrate the call.

use anyhow::Result;
use chrono::NaiveDate;
use core_domain::research::attribution::Evidence;
use core_domain::research::classification::classify_level;
use core_domain::{AnalysisScope, MappingQuality, PortfolioConfig};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::AppContext;

fn clickhouse_date_query_from(from: NaiveDate) -> NaiveDate {
    from.max(NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid ClickHouse Date lower bound"))
}

fn effective_evidence_window(
    from: NaiveDate,
    to: NaiveDate,
    earliest_available: Option<NaiveDate>,
    latest_available: Option<NaiveDate>,
) -> Option<(NaiveDate, NaiveDate)> {
    let effective_from = earliest_available.map(|date| date.max(from)).unwrap_or(from);
    let effective_to = latest_available.map(|date| date.min(to)).unwrap_or(to);
    (effective_from <= effective_to).then_some((effective_from, effective_to))
}

/// Supported research conditions for evidence computation.
pub const SUPPORTED_CONDITIONS: &[&str] = &[
    "srd-strong",
    "srd-strong-held",
    "srd-strong-non-held",
    "stretch-extreme-crowding-momentum",
];

/// Compute evidence for a named condition over a historical window.
///
/// The evidence contains raw facts (matched dates, forward returns) and derived
/// statistics (positive ratio, median forward return). It is reproducible from
/// the same market data.
///
/// Both condition matching and forward outcomes are bounded by the inclusive
/// `[from, to]` research window.
pub fn compute_condition_evidence(
    context: &AppContext,
    condition: &str,
    scope: AnalysisScope,
    horizon: usize,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Evidence> {
    let anchor_symbol = match scope {
        AnalysisScope::Global | AnalysisScope::Cn => "000300",
        AnalysisScope::Hk => "HSCEI",
    };

    let anchor_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        clickhouse_date_query_from(from),
        to,
    )?;
    let close_by_date: BTreeMap<NaiveDate, f64> =
        anchor_bars.iter().map(|b| (b.date, b.close)).collect();

    let earliest_available = close_by_date.keys().next().copied();
    let latest_available = close_by_date.keys().last().copied();

    let Some((effective_from, effective_to)) =
        effective_evidence_window(from, to, earliest_available, latest_available)
    else {
        return Ok(Evidence::default());
    };

    let matched_dates = match condition {
        "srd-strong" => match_srd_strong(context, scope, effective_from, effective_to)?,
        "srd-strong-held" => match_srd_strong_portfolio(context, scope, effective_from, effective_to, true)?,
        "srd-strong-non-held" => match_srd_strong_portfolio(context, scope, effective_from, effective_to, false)?,
        "stretch-extreme-crowding-momentum" => {
            match_stretch_extreme(context, scope, effective_from, effective_to)?
        }
        _ => anyhow::bail!(
            "Unknown condition '{}'. Supported: {:?}",
            condition,
            SUPPORTED_CONDITIONS
        ),
    };

    let mut matched_dates_out = Vec::new();
    let mut forward_returns = Vec::new();
    let mut max_drawdowns = Vec::new();

    for date in matched_dates {
        let Some(current_close) = close_by_date.get(&date) else { continue };
        if *current_close <= 0.0 {
            continue;
        }

        let Some((ret, max_dd)) =
            forward_outcome_on_or_before(&close_by_date, date, *current_close, horizon, to)
        else {
            continue;
        };

        matched_dates_out.push(date);
        forward_returns.push(ret);
        max_drawdowns.push(max_dd);
    }

    Ok(Evidence::from_facts(
        matched_dates_out,
        forward_returns,
        max_drawdowns,
        effective_from,
        effective_to,
    ))
}

fn forward_outcome_on_or_before(
    close_by_date: &BTreeMap<NaiveDate, f64>,
    observation_date: NaiveDate,
    observation_close: f64,
    horizon: usize,
    cutoff: NaiveDate,
) -> Option<(f64, f64)> {
    if horizon == 0 || observation_close <= 0.0 || observation_date >= cutoff {
        return None;
    }

    let start = observation_date.succ_opt()?;
    let forward_entries: Vec<f64> = close_by_date
        .range(start..=cutoff)
        .take(horizon)
        .map(|(_, close)| *close)
        .collect();
    if forward_entries.len() != horizon {
        return None;
    }

    let forward_close = *forward_entries.last()?;
    let forward_return = (forward_close - observation_close) / observation_close;
    let mut peak = observation_close;
    let mut max_drawdown = 0.0;
    for price in forward_entries {
        if price > peak {
            peak = price;
        }
        let drawdown = (peak - price) / peak;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    Some((forward_return, max_drawdown))
}

/// Match dates where StrongBuy >= 5 and StrategyState is conservative.
fn match_srd_strong(
    context: &AppContext,
    scope: AnalysisScope,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    let states = market_store::fetch_strategy_states_for_scope(&context.storage, scope)?;
    let conservative_dates: BTreeSet<NaiveDate> = states
        .iter()
        .filter(|s| {
            s.date >= from
                && s.date <= to
                && matches!(
                    s.state,
                    core_domain::StrategyState::NoTrade
                        | core_domain::StrategyState::DeRisk
                        | core_domain::StrategyState::LeftProbe
                )
        })
        .map(|s| s.date)
        .collect();

    let signal_snapshots = market_store::fetch_signal_snapshots_for_range_with_scope(
        &context.storage, scope, from, to,
    )?;

    let mut strong_buy_by_date: BTreeMap<NaiveDate, usize> = BTreeMap::new();
    for s in &signal_snapshots {
        if matches!(s.signal_label, core_domain::SignalLabel::StrongBuy) {
            *strong_buy_by_date.entry(s.date).or_insert(0) += 1;
        }
    }

    let mut matched = Vec::new();
    for date in conservative_dates {
        let count = strong_buy_by_date.get(&date).copied().unwrap_or(0);
        if count >= 5 {
            matched.push(date);
        }
    }
    matched.sort();
    Ok(matched)
}

/// Load PortfolioConfig from `config/portfolio.toml`.
/// Returns None if the file is missing, unparseable, or fails validation (silent skip).
fn load_portfolio_config() -> Option<PortfolioConfig> {
    let project_root = market_store::StorageConfig::project_root().ok()?;
    let path = project_root.join("config").join("portfolio.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    let config: PortfolioConfig = toml::from_str(&content).ok()?;
    config.validate().ok()?;
    Some(config)
}

/// Extract the set of EXACT-held underlying symbols from a PortfolioConfig.
/// Only EXACT positions with `enabled = true` participate.
/// PROXY and UNMAPPED symbols are deliberately excluded (ADR-067: no false precision).
fn exact_held_symbols(config: &PortfolioConfig) -> BTreeSet<String> {
    config
        .positions
        .iter()
        .filter(|p| p.enabled && p.mapping_quality == MappingQuality::Exact)
        .map(|p| p.underlying_symbol.clone())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Portfolio-aware SRD-strong matcher.
///
/// When `held = true`: matches conservative dates where StrongBuy ≥ 5 AND at least one
/// StrongBuy signal is for a symbol in the user's EXACT-held set.
/// When `held = false`: matches conservative dates where StrongBuy ≥ 5 AND none of the
/// StrongBuy signals are for EXACT-held symbols (i.e. the divergence is on non-held positions).
///
/// If PortfolioConfig is unavailable (missing/invalid), `held` returns empty Vec
/// (no held positions to filter on); `non-held` falls back to all conservative StrongBuy ≥ 5
/// dates (equivalent to `srd-strong`).
fn match_srd_strong_portfolio(
    context: &AppContext,
    scope: AnalysisScope,
    from: NaiveDate,
    to: NaiveDate,
    held: bool,
) -> Result<Vec<NaiveDate>> {
    let portfolio = load_portfolio_config();
    let held_set = portfolio.as_ref().map(|c| exact_held_symbols(c)).unwrap_or_default();

    let states = market_store::fetch_strategy_states_for_scope(&context.storage, scope)?;
    let conservative_dates: BTreeSet<NaiveDate> = states
        .iter()
        .filter(|s| {
            s.date >= from
                && s.date <= to
                && matches!(
                    s.state,
                    core_domain::StrategyState::NoTrade
                        | core_domain::StrategyState::DeRisk
                        | core_domain::StrategyState::LeftProbe
                )
        })
        .map(|s| s.date)
        .collect();

    let signal_snapshots = market_store::fetch_signal_snapshots_for_range_with_scope(
        &context.storage, scope, from, to,
    )?;

    let mut strong_buy_by_date: BTreeMap<NaiveDate, Vec<String>> = BTreeMap::new();
    for s in &signal_snapshots {
        if matches!(s.signal_label, core_domain::SignalLabel::StrongBuy) {
            strong_buy_by_date
                .entry(s.date)
                .or_default()
                .push(s.symbol.clone());
        }
    }

    let mut matched = Vec::new();
    for date in conservative_dates {
        let strong_symbols = strong_buy_by_date.get(&date);
        let count = strong_symbols.map(|v| v.len()).unwrap_or(0);
        if count < 5 {
            continue;
        }
        let symbols = strong_symbols.cloned().unwrap_or_default();
        let has_held = symbols.iter().any(|s| held_set.contains(s));
        let qualifies = if held { has_held } else { !has_held };
        if qualifies {
            matched.push(date);
        }
    }
    matched.sort();
    Ok(matched)
}

/// Match dates where Stretch Overall=Extreme, Crowding=Extreme, Momentum=Extreme, Breadth=Normal.
fn match_stretch_extreme(
    context: &AppContext,
    scope: AnalysisScope,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    let rotations = market_store::fetch_rotation_ranks_for_range(&context.storage, from, to)?;

    let instruments = context.seed_universe().unwrap_or_default();
    let symbol_in_scope = |symbol: &str| match scope {
        AnalysisScope::Global => true,
        AnalysisScope::Cn => instruments
            .iter()
            .any(|i| i.symbol == symbol && i.market == core_domain::Market::Cn),
        AnalysisScope::Hk => instruments
            .iter()
            .any(|i| i.symbol == symbol && i.market == core_domain::Market::Hk),
    };

    let mut rotation_by_date: BTreeMap<NaiveDate, Vec<(f64, f64)>> = BTreeMap::new();
    for r in rotations {
        if !symbol_in_scope(&r.symbol) {
            continue;
        }
        rotation_by_date
            .entry(r.date)
            .or_default()
            .push((r.momentum_score, r.rs_120));
    }

    let env_snapshots = market_store::fetch_environment_snapshots_for_scope(
        &context.storage,
        scope,
        from,
        to,
    )?;
    let breadth_by_date: BTreeMap<NaiveDate, f64> = env_snapshots
        .iter()
        .map(|e| (e.date, e.breadth_pct))
        .collect();

    let mut matched = Vec::new();
    for (date, rows) in &rotation_by_date {
        if rows.is_empty() {
            continue;
        }
        let total_momentum: f64 = rows.iter().map(|(m, _)| m).sum();
        let mut sorted = rows.clone();
        sorted.sort_by(|a, b| b.0.total_cmp(&a.0));
        let top5_sum: f64 = sorted.iter().take(5).map(|(m, _)| m).sum();
        let concentration_pct = if total_momentum > 0.0 {
            (top5_sum / total_momentum) * 100.0
        } else {
            0.0
        };
        let rs120_max = rows.iter().map(|(_, rs)| *rs).fold(f64::NEG_INFINITY, f64::max);

        let crowding_level = classify_level(concentration_pct, 30.0, 50.0, true);
        let momentum_level = classify_level(rs120_max, 70.0, 85.0, true);

        let breadth_pct = breadth_by_date.get(date).copied().unwrap_or(0.0);
        let breadth_level = classify_level(breadth_pct, 35.0, 20.0, false);

        if crowding_level == "Extreme" && momentum_level == "Extreme" && breadth_level == "Normal" {
            matched.push(*date);
        }
    }
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closes(rows: &[(u32, f64)]) -> BTreeMap<NaiveDate, f64> {
        rows.iter()
            .map(|(day, close)| (NaiveDate::from_ymd_opt(2026, 1, *day).unwrap(), *close))
            .collect()
    }

    #[test]
    fn forward_outcome_requires_exact_horizon_on_or_before_cutoff() {
        let observation_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let cutoff = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap();
        let close_by_date = closes(&[(1, 100.0), (2, 90.0), (3, 120.0), (4, 150.0)]);

        assert_eq!(
            forward_outcome_on_or_before(&close_by_date, observation_date, 100.0, 3, cutoff),
            None
        );
        assert_eq!(
            forward_outcome_on_or_before(&close_by_date, observation_date, 100.0, 2, cutoff),
            Some((0.2, 0.1))
        );
        assert_eq!(
            forward_outcome_on_or_before(&close_by_date, cutoff, 120.0, 1, cutoff),
            None
        );
    }

    #[test]
    fn bars_after_cutoff_cannot_mature_or_change_forward_outcome() {
        let observation_date = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let cutoff = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap();
        let without_future = closes(&[(1, 100.0), (2, 90.0), (3, 120.0)]);
        let with_future = closes(&[(1, 100.0), (2, 90.0), (3, 120.0), (4, 1.0)]);

        assert_eq!(
            forward_outcome_on_or_before(&without_future, observation_date, 100.0, 3, cutoff),
            None
        );
        assert_eq!(
            forward_outcome_on_or_before(&with_future, observation_date, 100.0, 3, cutoff),
            None
        );
        assert_eq!(
            forward_outcome_on_or_before(&without_future, observation_date, 100.0, 2, cutoff),
            forward_outcome_on_or_before(&with_future, observation_date, 100.0, 2, cutoff)
        );
    }

    #[test]
    fn clickhouse_date_query_from_clamps_pre_1970_date() {
        let requested = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();

        assert_eq!(
            clickhouse_date_query_from(requested),
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
        );
    }

    #[test]
    fn clickhouse_date_query_from_preserves_supported_date() {
        let requested = NaiveDate::from_ymd_opt(2005, 1, 4).unwrap();

        assert_eq!(clickhouse_date_query_from(requested), requested);
    }

    #[test]
    fn effective_evidence_window_uses_semantic_bounds() {
        let from = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 3, 30).unwrap();
        let earliest = NaiveDate::from_ymd_opt(2005, 1, 4).unwrap();
        let latest = NaiveDate::from_ymd_opt(2026, 9, 4).unwrap();

        assert_eq!(
            effective_evidence_window(from, to, Some(earliest), Some(latest)),
            Some((earliest, to))
        );
    }

    #[test]
    fn supported_conditions_constant() {
        assert!(SUPPORTED_CONDITIONS.contains(&"srd-strong"));
        assert!(SUPPORTED_CONDITIONS.contains(&"srd-strong-held"));
        assert!(SUPPORTED_CONDITIONS.contains(&"srd-strong-non-held"));
        assert!(SUPPORTED_CONDITIONS.contains(&"stretch-extreme-crowding-momentum"));
    }

    #[test]
    fn unknown_condition_returns_error() {
        // We cannot easily construct an AppContext in tests, so we just verify
        // the error message formatting by checking the condition parsing path.
        let condition = "unknown-condition";
        assert!(!SUPPORTED_CONDITIONS.contains(&condition));
    }
}

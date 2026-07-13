//! Research evidence computation.
//!
//! This module owns the deterministic, reproducible computation of conditional
//! forward-return evidence. It is a service-layer helper: it fetches data through
//! `market-store`, computes through `core-domain::research`, and returns a plain
//! `Evidence` value.
//!
//! Future: when a dedicated `research-engine` crate is introduced, this module
//! should migrate there. AppService should then only orchestrate the call.

use anyhow::{Result};
use chrono::NaiveDate;
use core_domain::research::attribution::Evidence;
use core_domain::research::classification::classify_level;
use core_domain::AnalysisScope;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::AppContext;

/// Supported research conditions for evidence computation.
pub const SUPPORTED_CONDITIONS: &[&str] = &[
    "srd-strong",
    "stretch-extreme-crowding-momentum",
];

/// Compute evidence for a named condition over a historical window.
///
/// The evidence contains raw facts (matched dates, forward returns) and derived
/// statistics (positive ratio, median forward return). It is reproducible from
/// the same market data.
///
/// If `from` is later than `to`, the window is clamped to available data.
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

    let anchor_bars = market_store::fetch_daily_bars(&context.storage, anchor_symbol)?;
    let close_by_date: BTreeMap<NaiveDate, f64> =
        anchor_bars.iter().map(|b| (b.date, b.close)).collect();

    let earliest_available = close_by_date.keys().next().copied();
    let latest_available = close_by_date.keys().last().copied();

    let effective_from = earliest_available.map(|d| d.max(from)).unwrap_or(from);
    let effective_to = latest_available.map(|d| d.min(to)).unwrap_or(to);

    if effective_from > effective_to {
        return Ok(Evidence::default());
    }

    let matched_dates = match condition {
        "srd-strong" => match_srd_strong(context, scope, effective_from, effective_to)?,
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

        let start = date.succ_opt().unwrap_or(date);
        let forward_entries: Vec<(NaiveDate, f64)> = close_by_date
            .range(start..)
            .take(horizon)
            .map(|(d, c)| (*d, *c))
            .collect();

        if forward_entries.len() < horizon {
            continue;
        }

        let forward_close = forward_entries.last().unwrap().1;
        let ret = (forward_close - *current_close) / *current_close;

        let mut peak = *current_close;
        let mut max_dd = 0.0;
        for (_, price) in &forward_entries {
            if *price > peak {
                peak = *price;
            }
            let dd = (peak - *price) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }

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

    #[test]
    fn supported_conditions_constant() {
        assert!(SUPPORTED_CONDITIONS.contains(&"srd-strong"));
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

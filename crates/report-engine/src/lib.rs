use backtest_engine::BacktestSummary;
use chrono::NaiveDate;
use core_domain::{MarketRegimeSnapshot, RotationRankSnapshot, SignalSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportArtifact {
    pub report_type: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub scope: String,
    pub report_date: String,
    pub latest_available_date: String,
    pub regime_as_of_date: String,
    pub regime_stale_days: i64,
    pub regime_label: String,
    pub trend_score: f64,
    pub liquidity_score: f64,
    pub risk_score: f64,
    pub top_rotation: Vec<RotationRankSnapshot>,
    pub bottom_rotation: Vec<RotationRankSnapshot>,
    pub top_signals: Vec<SignalSnapshot>,
    pub bullish_signals: Vec<SignalSnapshot>,
    pub defensive_signals: Vec<SignalSnapshot>,
    pub watchlist_breadth: Option<WatchlistBreadthSnapshot>,
    pub latest_backtest: Option<BacktestSummary>,
    pub load_metrics: Option<DashboardLoadMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLoadMetrics {
    pub available_dates_ms: u64,
    pub regime_ms: u64,
    pub rotations_ms: u64,
    pub signals_ms: u64,
    pub backtest_ms: u64,
    pub breadth_ms: u64,
    pub assembly_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistBreadthMarketSnapshot {
    pub market: String,
    pub universe_label: String,
    pub eligible_count: usize,
    pub above_count: usize,
    pub breadth_pct: f64,
    pub breadth_pct_sma5: Option<f64>,
    pub breadth_5d_delta: Option<f64>,
    pub range_low_60d: Option<f64>,
    pub range_high_60d: Option<f64>,
    pub range_position_60d: Option<f64>,
    pub status_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistBreadthSnapshot {
    pub report_date: String,
    pub markets: Vec<WatchlistBreadthMarketSnapshot>,
    pub methodology_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataHealthSymbolSummary {
    pub symbol: String,
    pub name: String,
    pub display_symbol: Option<String>,
    pub rows: usize,
    pub first_date: Option<NaiveDate>,
    pub last_date: Option<NaiveDate>,
    pub primary_provider_ok: bool,
    pub fallback_provider_ok: Option<bool>,
    pub missing_turnover_rows: usize,
    pub gap_count: usize,
    pub max_gap_days: i64,
    pub suspicious_jump_count: usize,
    pub max_abs_daily_return_pct: f64,
    pub status: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataHealthMacroSourceSummary {
    pub factor_name: String,
    pub source: String,
    pub transport: String,
    pub rows: usize,
    pub first_date: Option<NaiveDate>,
    pub last_date: Option<NaiveDate>,
    pub status: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataHealthSummary {
    pub generated_at: String,
    pub canonical_adjustment: String,
    pub freshest_market_date: Option<NaiveDate>,
    pub symbols_on_freshest_market_date: usize,
    pub symbols_missing_freshest_market_date: usize,
    pub freshest_market_date_complete: bool,
    pub checked_symbols: usize,
    pub healthy_symbols: usize,
    pub review_symbols: usize,
    pub critical_symbols: usize,
    pub healthy_macro_sources: usize,
    pub review_macro_sources: usize,
    pub critical_macro_sources: usize,
    pub macro_sources: Vec<DataHealthMacroSourceSummary>,
    pub symbols: Vec<DataHealthSymbolSummary>,
}

pub fn build_dashboard_snapshot(
    regimes: &[MarketRegimeSnapshot],
    rotations: &[RotationRankSnapshot],
    signals: &[SignalSnapshot],
    latest_backtest: Option<BacktestSummary>,
    selected_date: Option<NaiveDate>,
) -> Option<DashboardSnapshot> {
    let regime_dates = regimes.iter().map(|row| row.date).collect::<BTreeSet<_>>();
    let rotation_dates = rotations
        .iter()
        .map(|row| row.date)
        .collect::<BTreeSet<_>>();
    let available_dates = signals
        .iter()
        .map(|row| row.date)
        .filter(|date| {
            rotation_dates.contains(date)
                && regime_dates.iter().any(|regime_date| regime_date <= date)
        })
        .collect::<BTreeSet<_>>();
    let latest_available_date = available_dates.iter().copied().max()?;
    let report_date = if let Some(date) = selected_date {
        if available_dates.contains(&date) {
            date
        } else {
            return None;
        }
    } else {
        latest_available_date
    };
    let regime = regimes
        .iter()
        .filter(|row| row.date <= report_date)
        .max_by(|left, right| {
            left.date
                .cmp(&right.date)
                .then_with(|| left.market.cmp(&right.market))
        })?;
    let mut top_rotation = rotations
        .iter()
        .filter(|row| row.date == report_date)
        .cloned()
        .collect::<Vec<_>>();
    top_rotation.sort_by(|left, right| left.rank.cmp(&right.rank));
    let mut bottom_rotation = top_rotation.clone();
    bottom_rotation.sort_by(|left, right| {
        left.momentum_score
            .total_cmp(&right.momentum_score)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    bottom_rotation.truncate(5);
    top_rotation.truncate(5);

    let mut top_signals = signals
        .iter()
        .filter(|row| row.date == report_date)
        .cloned()
        .collect::<Vec<_>>();
    top_signals.sort_by(|left, right| {
        right
            .final_score
            .total_cmp(&left.final_score)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    let bullish_signals = top_signals
        .iter()
        .filter(|row| {
            matches!(
                row.signal_label,
                core_domain::SignalLabel::StrongBuy | core_domain::SignalLabel::Buy
            )
        })
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    let defensive_signals = top_signals
        .iter()
        .filter(|row| {
            matches!(
                row.signal_label,
                core_domain::SignalLabel::Reduce
                    | core_domain::SignalLabel::Sell
                    | core_domain::SignalLabel::Hold
                    | core_domain::SignalLabel::Watch
            )
        })
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    top_signals.truncate(5);

    Some(DashboardSnapshot {
        scope: "GLOBAL".to_string(),
        report_date: report_date.to_string(),
        latest_available_date: latest_available_date.to_string(),
        regime_as_of_date: regime.macro_as_of_date.to_string(),
        regime_stale_days: (report_date - regime.macro_as_of_date).num_days(),
        regime_label: regime.regime_label.clone(),
        trend_score: regime.trend_score,
        liquidity_score: regime.liquidity_score,
        risk_score: regime.risk_score,
        top_rotation,
        bottom_rotation,
        top_signals,
        bullish_signals,
        defensive_signals,
        watchlist_breadth: None,
        latest_backtest,
        load_metrics: None,
    })
}

pub fn build_dashboard_snapshot_for_date(
    regime: &MarketRegimeSnapshot,
    rotations: &[RotationRankSnapshot],
    signals: &[SignalSnapshot],
    latest_backtest: Option<BacktestSummary>,
    report_date: NaiveDate,
    latest_available_date: NaiveDate,
    scope: &str,
) -> DashboardSnapshot {
    let mut top_rotation = rotations.to_vec();
    top_rotation.sort_by(|left, right| left.rank.cmp(&right.rank));
    let mut bottom_rotation = top_rotation.clone();
    bottom_rotation.sort_by(|left, right| {
        left.momentum_score
            .total_cmp(&right.momentum_score)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    bottom_rotation.truncate(5);
    top_rotation.truncate(5);

    let mut top_signals = signals.to_vec();
    top_signals.sort_by(|left, right| {
        right
            .final_score
            .total_cmp(&left.final_score)
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    let bullish_signals = top_signals
        .iter()
        .filter(|row| {
            matches!(
                row.signal_label,
                core_domain::SignalLabel::StrongBuy | core_domain::SignalLabel::Buy
            )
        })
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    let defensive_signals = top_signals
        .iter()
        .filter(|row| {
            matches!(
                row.signal_label,
                core_domain::SignalLabel::Reduce
                    | core_domain::SignalLabel::Sell
                    | core_domain::SignalLabel::Hold
                    | core_domain::SignalLabel::Watch
            )
        })
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    top_signals.truncate(5);

    DashboardSnapshot {
        scope: scope.to_string(),
        report_date: report_date.to_string(),
        latest_available_date: latest_available_date.to_string(),
        regime_as_of_date: regime.macro_as_of_date.to_string(),
        regime_stale_days: (report_date - regime.macro_as_of_date).num_days(),
        regime_label: regime.regime_label.clone(),
        trend_score: regime.trend_score,
        liquidity_score: regime.liquidity_score,
        risk_score: regime.risk_score,
        top_rotation,
        bottom_rotation,
        top_signals,
        bullish_signals,
        defensive_signals,
        watchlist_breadth: None,
        latest_backtest,
        load_metrics: None,
    }
}

fn format_optional_pct(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.2}%"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn format_optional_delta(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:+.2} pts"))
        .unwrap_or_else(|| "N/A".to_string())
}

pub fn render_markdown_report(snapshot: &DashboardSnapshot) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "# Daily Quant Report\n\nScope: {}\nDate: {}\n\n",
        snapshot.scope, snapshot.report_date
    ));
    output.push_str("## Market Regime\n\n");
    output.push_str(&format!(
        "- Label: {}\n- Regime As Of: {}\n- Regime Lag: {} day(s)\n- Trend Score: {:.2}\n- Liquidity Score: {:.2}\n- Risk Score: {:.2}\n\n",
        snapshot.regime_label,
        snapshot.regime_as_of_date,
        snapshot.regime_stale_days,
        snapshot.trend_score,
        snapshot.liquidity_score,
        snapshot.risk_score
    ));
    output.push_str("## Watchlist Breadth (MA30)\n\n");
    if let Some(breadth) = &snapshot.watchlist_breadth {
        output.push_str(&format!("- Methodology: {}\n", breadth.methodology_note));
        output.push_str(&format!("- Report Date: {}\n", breadth.report_date));
        for market in &breadth.markets {
            output.push_str(&format!(
                "- {} | breadth={:.2}% | above={}/{} | sma5={} | delta={} | range_pos={} | status={}\n",
                market.universe_label,
                market.breadth_pct,
                market.above_count,
                market.eligible_count,
                format_optional_pct(market.breadth_pct_sma5),
                format_optional_delta(market.breadth_5d_delta),
                format_optional_pct(market.range_position_60d.map(|value| value * 100.0)),
                market.status_label
            ));
        }
        output.push_str("\n");
    } else {
        output.push_str("- Watchlist breadth proxy is unavailable for this report date\n\n");
    }
    output.push_str("## Top Rotation\n\n");
    for item in &snapshot.top_rotation {
        output.push_str(&format!(
            "- #{} {} | momentum={:.2} | rs20={:.2} | rs60={:.2} | rs120={:.2}\n",
            item.rank, item.symbol, item.momentum_score, item.rs_20, item.rs_60, item.rs_120
        ));
    }
    output.push_str("\n## Rotation Laggards\n\n");
    for item in &snapshot.bottom_rotation {
        output.push_str(&format!(
            "- #{} {} | momentum={:.2} | rs20={:.2} | rs60={:.2} | rs120={:.2}\n",
            item.rank, item.symbol, item.momentum_score, item.rs_20, item.rs_60, item.rs_120
        ));
    }
    output.push_str("\n## Top Signals\n\n");
    for item in &snapshot.top_signals {
        output.push_str(&format!(
            "- {} | score={:.2} | label={:?} | {}\n",
            item.symbol, item.final_score, item.signal_label, item.explanation
        ));
    }
    output.push_str("\n## Bullish Signals\n\n");
    for item in &snapshot.bullish_signals {
        output.push_str(&format!(
            "- {} | score={:.2} | label={:?} | {}\n",
            item.symbol, item.final_score, item.signal_label, item.explanation
        ));
    }
    output.push_str("\n## Defensive Signals\n\n");
    for item in &snapshot.defensive_signals {
        output.push_str(&format!(
            "- {} | score={:.2} | label={:?} | {}\n",
            item.symbol, item.final_score, item.signal_label, item.explanation
        ));
    }
    output.push_str("\n## Latest Backtest\n\n");
    if let Some(backtest) = &snapshot.latest_backtest {
        output.push_str(&format!(
            "- Run ID: {}\n- Strategy: {}\n- CAGR: {:.4}\n- Max Drawdown: {:.4}\n- Sharpe: {:.4}\n- Final Equity: {:.2}\n- Trades: {}\n- Trading Days: {}\n",
            backtest.run_id,
            backtest.strategy_name,
            backtest.cagr,
            backtest.max_drawdown,
            backtest.sharpe,
            backtest.final_equity,
            backtest.trades,
            backtest.trading_days
        ));
    } else {
        output.push_str("- No backtest result available\n");
    }
    output
}

pub fn collect_dashboard_dates(
    regimes: &[MarketRegimeSnapshot],
    rotations: &[RotationRankSnapshot],
    signals: &[SignalSnapshot],
) -> Vec<String> {
    let regime_dates = regimes.iter().map(|row| row.date).collect::<BTreeSet<_>>();
    let rotation_dates = rotations
        .iter()
        .map(|row| row.date)
        .collect::<BTreeSet<_>>();
    signals
        .iter()
        .map(|row| row.date)
        .filter(|date| {
            rotation_dates.contains(date)
                && regime_dates.iter().any(|regime_date| regime_date <= date)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .rev()
        .map(|date| date.to_string())
        .collect()
}

pub fn render_data_health_report(summary: &DataHealthSummary) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "# Data Health Report\n\nGenerated At: {}\n\n",
        summary.generated_at
    ));
    output.push_str("## Summary\n\n");
    output.push_str(&format!(
        "- Canonical Adjustment: {}\n- Freshest Market Date: {}\n- Latest-Day Coverage: {}/{}\n- Latest-Day Complete: {}\n- Checked Symbols: {}\n- Healthy Symbols: {}\n- Review Symbols: {}\n- Critical Symbols: {}\n- Healthy Macro Sources: {}\n- Review Macro Sources: {}\n- Critical Macro Sources: {}\n\n",
        summary.canonical_adjustment,
        summary
            .freshest_market_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        summary.symbols_on_freshest_market_date,
        summary.checked_symbols,
        if summary.freshest_market_date_complete { "yes" } else { "no" },
        summary.checked_symbols,
        summary.healthy_symbols,
        summary.review_symbols,
        summary.critical_symbols,
        summary.healthy_macro_sources,
        summary.review_macro_sources,
        summary.critical_macro_sources
    ));
    output.push_str("## Macro Source Checks\n\n");
    for row in &summary.macro_sources {
        output.push_str(&format!(
            "- {} | source={} | transport={} | status={} | rows={} | range={} -> {}\n",
            row.factor_name,
            row.source,
            row.transport,
            row.status,
            row.rows,
            row.first_date
                .map(|date| date.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            row.last_date
                .map(|date| date.to_string())
                .unwrap_or_else(|| "N/A".to_string())
        ));
        if !row.notes.is_empty() {
            for note in &row.notes {
                output.push_str(&format!("  - {}\n", note));
            }
        }
    }
    output.push_str("\n");
    output.push_str("## Symbol Checks\n\n");
    for row in &summary.symbols {
        output.push_str(&format!(
            "- {} ({}) | status={} | rows={} | primary_ok={} | fallback_ok={:?} | gaps={} | max_gap_days={} | suspicious_jumps={} | max_abs_return={:.2}% | missing_turnover={}\n",
            row.symbol,
            row.name,
            row.status,
            row.rows,
            row.primary_provider_ok,
            row.fallback_provider_ok,
            row.gap_count,
            row.max_gap_days,
            row.suspicious_jump_count,
            row.max_abs_daily_return_pct,
            row.missing_turnover_rows
        ));
        if !row.notes.is_empty() {
            for note in &row.notes {
                output.push_str(&format!("  - {}\n", note));
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_markdown_report_includes_watchlist_breadth_section() {
        let snapshot = DashboardSnapshot {
            scope: "GLOBAL".to_string(),
            report_date: "2026-03-20".to_string(),
            latest_available_date: "2026-03-20".to_string(),
            regime_as_of_date: "2026-03-19".to_string(),
            regime_stale_days: 1,
            regime_label: "risk_on".to_string(),
            trend_score: 72.0,
            liquidity_score: 60.0,
            risk_score: 55.0,
            top_rotation: Vec::new(),
            bottom_rotation: Vec::new(),
            top_signals: Vec::new(),
            bullish_signals: Vec::new(),
            defensive_signals: Vec::new(),
            watchlist_breadth: Some(WatchlistBreadthSnapshot {
                report_date: "2026-03-20".to_string(),
                methodology_note: "Proxy only; not full-market stock breadth.".to_string(),
                markets: vec![WatchlistBreadthMarketSnapshot {
                    market: "CN".to_string(),
                    universe_label: "CN tracked universe".to_string(),
                    eligible_count: 4,
                    above_count: 3,
                    breadth_pct: 75.0,
                    breadth_pct_sma5: Some(66.0),
                    breadth_5d_delta: Some(12.0),
                    range_low_60d: Some(25.0),
                    range_high_60d: Some(80.0),
                    range_position_60d: Some(0.91),
                    status_label: "near_local_high".to_string(),
                }],
            }),
            latest_backtest: None,
            load_metrics: None,
        };

        let rendered = render_markdown_report(&snapshot);

        assert!(rendered.contains("## Watchlist Breadth (MA30)"));
        assert!(rendered.contains("Proxy only; not full-market stock breadth."));
        assert!(rendered.contains("CN tracked universe | breadth=75.00% | above=3/4"));
        assert!(rendered.contains("status=near_local_high"));
    }

    #[test]
    fn render_data_health_report_includes_latest_day_coverage_summary() {
        let summary = DataHealthSummary {
            generated_at: "2026-03-30T12:00:00+00:00".to_string(),
            canonical_adjustment: "forward-adjusted daily bars (Eastmoney fqt=1, Tencent qfq)"
                .to_string(),
            freshest_market_date: Some(NaiveDate::from_ymd_opt(2026, 3, 30).unwrap()),
            symbols_on_freshest_market_date: 20,
            symbols_missing_freshest_market_date: 2,
            freshest_market_date_complete: false,
            checked_symbols: 22,
            healthy_symbols: 14,
            review_symbols: 8,
            critical_symbols: 0,
            healthy_macro_sources: 0,
            review_macro_sources: 4,
            critical_macro_sources: 0,
            macro_sources: Vec::new(),
            symbols: Vec::new(),
        };

        let rendered = render_data_health_report(&summary);

        assert!(rendered.contains("Freshest Market Date: 2026-03-30"));
        assert!(rendered.contains("Latest-Day Coverage: 20/22"));
        assert!(rendered.contains("Latest-Day Complete: no"));
    }
}

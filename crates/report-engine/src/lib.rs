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
    pub latest_backtest: Option<BacktestSummary>,
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
        .max_by(|left, right| left.market.cmp(&right.market))?;
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
        latest_backtest,
    })
}

pub fn render_markdown_report(snapshot: &DashboardSnapshot) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "# Daily Quant Report\n\nDate: {}\n\n",
        snapshot.report_date
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
        "- Canonical Adjustment: {}\n- Checked Symbols: {}\n- Healthy Symbols: {}\n- Review Symbols: {}\n- Critical Symbols: {}\n- Healthy Macro Sources: {}\n- Review Macro Sources: {}\n- Critical Macro Sources: {}\n\n",
        summary.canonical_adjustment,
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

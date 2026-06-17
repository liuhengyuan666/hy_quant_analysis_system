use backtest_engine::BacktestSummary;
use chrono::NaiveDate;
use core_domain::{
    EnvironmentSnapshot, MarketRegimeSnapshot, RotationRankSnapshot, SignalSnapshot,
    StrategyStateSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

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
    pub environment: Option<EnvironmentSnapshot>,
    pub strategy_state: Option<StrategyStateSnapshot>,
    pub trust_summary: Option<TrustSummary>,
    pub watchlist_breadth: Option<WatchlistBreadthSnapshot>,
    pub symbol_names: HashMap<String, String>,
    pub latest_backtest: Option<BacktestSummary>,
    pub load_metrics: Option<DashboardLoadMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustSummary {
    pub level: String,
    pub headline: String,
    pub message: String,
    pub pipeline_has_partial_latest: bool,
    pub pipeline_has_stale_stage: bool,
    pub pipeline_partial_latest_stage_count: usize,
    pub pipeline_stale_stage_count: usize,
    pub freshest_market_date: Option<String>,
    pub latest_available_date: Option<String>,
    pub latest_day_complete: bool,
    pub scoped_symbols_expected: usize,
    pub scoped_symbols_on_freshest_market_date: usize,
    pub macro_status: String,
    pub data_health_generated_at: Option<String>,
    pub data_health_review_symbols: Option<usize>,
    pub data_health_critical_symbols: Option<usize>,
    pub data_health_review_macro_sources: Option<usize>,
    pub data_health_critical_macro_sources: Option<usize>,
    pub signal_analysis_scope: Option<String>,
    pub signal_regime_basis_scope: Option<String>,
    pub strategy_state: Option<String>,
    pub strategy_recommended_position_pct: Option<f64>,
    pub backtest_matches_snapshot: Option<bool>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLoadMetrics {
    pub available_dates_ms: u64,
    pub regime_ms: u64,
    pub environment_ms: u64,
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
    pub latest_gate_required: bool,
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
    pub latest_gate_checked_symbols: usize,
    pub latest_gate_symbols_on_freshest_market_date: usize,
    pub latest_gate_symbols_missing_freshest_market_date: usize,
    pub latest_gate_freshest_market_date_complete: bool,
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
        environment: None,
        strategy_state: None,
        trust_summary: None,
        watchlist_breadth: None,
        latest_backtest,
        load_metrics: None,
        symbol_names: HashMap::new(),
    })
}

pub fn build_dashboard_snapshot_for_date(
    regime: &MarketRegimeSnapshot,
    rotations: &[RotationRankSnapshot],
    signals: &[SignalSnapshot],
    strategy_state: Option<StrategyStateSnapshot>,
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
        environment: None,
        strategy_state,
        trust_summary: None,
        watchlist_breadth: None,
        latest_backtest,
        load_metrics: None,
        symbol_names: HashMap::new(),
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

fn first_signal(snapshot: &DashboardSnapshot) -> Option<&SignalSnapshot> {
    snapshot
        .top_signals
        .first()
        .or_else(|| snapshot.bullish_signals.first())
        .or_else(|| snapshot.defensive_signals.first())
}

fn format_signal_breakdown(signal: &SignalSnapshot) -> String {
    let reason = &signal.reason;
    format!(
        "{} | best={:?} strategy={:.2}/{:.2} | alignment={}/{} contrib={:.2} | regime trend={:.2} risk={:.2} combined={:.2} contrib={:.2} | rotation momentum={:.2} rank={} combined={:.2} contrib={:.2}",
        reason.summary,
        reason.best_strategy,
        reason.strategy_score,
        reason.strategy_contribution,
        reason.alignment,
        reason
            .aligned_strategies
            .iter()
            .map(|strategy| format!("{:?}", strategy))
            .collect::<Vec<_>>()
            .join(","),
        reason.alignment_contribution,
        reason.regime.trend_score,
        reason.regime.risk_score,
        reason.regime.combined_score,
        reason.regime.contribution,
        reason.rotation.momentum_score,
        reason
            .rotation
            .rank
            .map(|rank| rank.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        reason.rotation.combined_score,
        reason.rotation.contribution,
    )
}

fn signal_basis_note(snapshot: &DashboardSnapshot) -> Option<String> {
    let signal = first_signal(snapshot)?;
    let analysis_scope = signal.analysis_scope.to_uppercase();
    let regime_basis_scope = signal.regime_basis_scope.to_uppercase();
    let snapshot_scope = snapshot.scope.to_uppercase();
    let mut note = format!(
        "- Signal Analysis Scope: {}\n- Signal Regime Basis: {}\n",
        analysis_scope, regime_basis_scope
    );
    if regime_basis_scope != snapshot_scope {
        note.push_str(&format!(
            "- Trust Note: signal scoring still uses {} regime semantics while this report is scoped to {}\n",
            regime_basis_scope, snapshot_scope
        ));
    }
    Some(note)
}

fn backtest_matches_snapshot(snapshot: &DashboardSnapshot, backtest: &BacktestSummary) -> bool {
    backtest
        .analysis_scope
        .eq_ignore_ascii_case(&snapshot.scope)
        && backtest.signal_scope.eq_ignore_ascii_case(&snapshot.scope)
        && backtest
            .signal_end_date
            .map(|date| date.to_string())
            .as_deref()
            == Some(snapshot.report_date.as_str())
}

pub fn render_markdown_report(snapshot: &DashboardSnapshot) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "# Daily Quant Report\n\nScope: {}\nDate: {}\n\n",
        snapshot.scope, snapshot.report_date
    ));
    output.push_str("## Trust Summary\n\n");
    if let Some(trust) = &snapshot.trust_summary {
        output.push_str(&format!(
            "- Level: {}\n- Headline: {}\n- Message: {}\n- Freshest Market Date: {}\n- Latest Available Date: {}\n- Latest-Day Complete: {}\n- Macro Status: {}\n- Data Health Generated At: {}\n- Data Health Review Symbols: {}\n- Data Health Critical Symbols: {}\n- Data Health Review Macro Sources: {}\n- Data Health Critical Macro Sources: {}\n- Signal Analysis Scope: {}\n- Signal Regime Basis: {}\n- Backtest Matches Current Snapshot: {}\n",
            trust.level,
            trust.headline,
            trust.message,
            trust
                .freshest_market_date
                .clone()
                .unwrap_or_else(|| "N/A".to_string()),
            trust
                .latest_available_date
                .clone()
                .unwrap_or_else(|| "N/A".to_string()),
            if trust.latest_day_complete { "yes" } else { "no" },
            trust.macro_status,
            trust
                .data_health_generated_at
                .clone()
                .unwrap_or_else(|| "N/A".to_string()),
            trust.data_health_review_symbols.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()),
            trust.data_health_critical_symbols.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()),
            trust.data_health_review_macro_sources.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()),
            trust.data_health_critical_macro_sources.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()),
            trust
                .signal_analysis_scope
                .clone()
                .unwrap_or_else(|| "N/A".to_string()),
            trust
                .signal_regime_basis_scope
                .clone()
                .unwrap_or_else(|| "N/A".to_string()),
            match trust.backtest_matches_snapshot {
                Some(true) => "yes",
                Some(false) => "no",
                None => "N/A",
            },
        ));
        if let Some(state) = &trust.strategy_state {
            output.push_str(&format!(
                "- Strategy State: {}\n- Strategy Recommended Position: {}\n",
                state,
                trust
                    .strategy_recommended_position_pct
                    .map(|value| format!("{value:.2}%"))
                    .unwrap_or_else(|| "N/A".to_string())
            ));
        }
        output.push_str(&format!(
            "- Scoped Latest-Day Coverage: {}/{}\n- Pipeline Partial Latest: {} ({})\n- Pipeline Stale Stage: {} ({})\n",
            trust.scoped_symbols_on_freshest_market_date,
            trust.scoped_symbols_expected,
            if trust.pipeline_has_partial_latest { "yes" } else { "no" },
            trust.pipeline_partial_latest_stage_count,
            if trust.pipeline_has_stale_stage { "yes" } else { "no" },
            trust.pipeline_stale_stage_count,
        ));
        if snapshot.report_date != snapshot.latest_available_date {
            output.push_str(&format!(
                "- Trust Evidence Basis: current operational freshness/data-health evidence as of {} while snapshot content remains scoped to report date {}\n",
                snapshot.latest_available_date,
                snapshot.report_date,
            ));
        }
        for note in &trust.notes {
            output.push_str(&format!("- Note: {}\n", note));
        }
        output.push_str("\n");
    } else {
        output.push_str("- Trust summary is unavailable for this report export\n\n");
    }
    output.push_str("## Strategy State\n\n");
    if let Some(strategy_state) = &snapshot.strategy_state {
        output.push_str(&format!(
            "- State: {}\n- State As Of: {}\n- Scope: {}\n- State Score: {:.2}\n- Recommended Position: {:.2}%\n- Transition Reason: {}\n\n",
            strategy_state.state,
            strategy_state.date,
            strategy_state.scope,
            strategy_state.state_score,
            strategy_state.recommended_position_pct,
            strategy_state.transition_reason,
        ));
    } else {
        output.push_str("- Strategy state is unavailable for this report date\n\n");
    }
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
    output.push_str("## Environment Layer\n\n");
    if let Some(environment) = &snapshot.environment {
        output.push_str(&format!(
            "- Label: {}\n- Environment Score: {:.2}\n- Scope: {}\n- Regime As Of: {}\n- Breadth As Of: {}\n- Stress As Of: {}\n- Breadth: {:.2}% ({}/{})\n- Breadth SMA5: {}\n- Breadth 5d Delta: {}\n- Breadth State: {}\n- Liquidity Proxy Score: {:.2}\n- Stress Proxy Score: {:.2}\n- Volume Expansion: {}\n- Turnover Coverage: {}\n\n",
            environment.environment_label,
            environment.environment_score,
            environment.scope,
            environment.regime_as_of_date,
            environment.breadth_as_of_date,
            environment.stress_as_of_date,
            environment.breadth_pct,
            environment.breadth_above_count,
            environment.breadth_eligible_count,
            format_optional_pct(environment.breadth_pct_sma5),
            format_optional_delta(environment.breadth_5d_delta),
            environment.breadth_state,
            environment.liquidity_proxy_score,
            environment.stress_proxy_score,
            format_optional_pct(environment.volume_expansion_pct),
            format_optional_pct(environment.turnover_coverage_pct),
        ));
    } else {
        output.push_str("- Environment snapshot is unavailable for this report date\n\n");
    }
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
        let name = snapshot.symbol_names.get(&item.symbol).map(|s| s.as_str()).unwrap_or("");
        output.push_str(&format!(
            "- {} ({}) | score={:.2} | label={:?} | {}\n",
            item.symbol,
            name,
            item.final_score,
            item.signal_label,
            format_signal_breakdown(item)
        ));
    }
    if let Some(note) = signal_basis_note(snapshot) {
        output.push_str(&note);
    }
    output.push_str("\n## Bullish Signals\n\n");
    for item in &snapshot.bullish_signals {
        let name = snapshot.symbol_names.get(&item.symbol).map(|s| s.as_str()).unwrap_or("");
        output.push_str(&format!(
            "- {} ({}) | score={:.2} | label={:?} | {}\n",
            item.symbol,
            name,
            item.final_score,
            item.signal_label,
            format_signal_breakdown(item)
        ));
    }
    output.push_str("\n## Defensive Signals\n\n");
    for item in &snapshot.defensive_signals {
        let name = snapshot.symbol_names.get(&item.symbol).map(|s| s.as_str()).unwrap_or("");
        output.push_str(&format!(
            "- {} ({}) | score={:.2} | label={:?} | {}\n",
            item.symbol,
            name,
            item.final_score,
            item.signal_label,
            format_signal_breakdown(item)
        ));
    }
    output.push_str("\n## Latest Backtest\n\n");
    if let Some(backtest) = &snapshot.latest_backtest {
        output.push_str(&format!(
            "- Run ID: {}\n- Strategy: {}\n- Analysis Scope: {}\n- Signal Scope: {}\n- Regime Basis: {}\n- Signal Window: {} -> {}\n- Config: {}\n- Matches Current Snapshot: {}\n- CAGR: {:.4}\n- Max Drawdown: {:.4}\n- Drawdown Events: {}\n- Sharpe: {:.4}\n- Final Equity: {:.2}\n- Trades: {}\n- Trading Days: {}\n",
            backtest.run_id,
            backtest.strategy_name,
            backtest.analysis_scope,
            backtest.signal_scope,
            backtest.regime_basis_scope,
            backtest
                .signal_start_date
                .map(|date| date.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            backtest
                .signal_end_date
                .map(|date| date.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            backtest.config_summary,
            if backtest_matches_snapshot(snapshot, backtest) {
                "yes"
            } else {
                "no"
            },
            backtest.cagr,
            backtest.max_drawdown,
            backtest.drawdown_events,
            backtest.sharpe,
            backtest.final_equity,
            backtest.trades,
            backtest.trading_days
        ));
        if backtest.drawdown_events > 0 {
            output.push_str(
                "- Note: Drawdown limit protective actions were triggered during this run.\n",
            );
        }
        if !backtest.state_trajectory.is_empty() {
            output.push_str("\n## Strategy State Trajectory\n\n");
            for (date, state) in &backtest.state_trajectory {
                output.push_str(&format!("- {} | {}\n", date, state));
            }
        }
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
        "- Canonical Adjustment: {}\n- Freshest Market Date: {}\n- Latest-Day Coverage: {}/{}\n- Latest-Day Complete: {}\n- Report-Gate Latest-Day Coverage: {}/{}\n- Report-Gate Latest-Day Complete: {}\n- Checked Symbols: {}\n- Healthy Symbols: {}\n- Review Symbols: {}\n- Critical Symbols: {}\n- Healthy Macro Sources: {}\n- Review Macro Sources: {}\n- Critical Macro Sources: {}\n\n",
        summary.canonical_adjustment,
        summary
            .freshest_market_date
            .map(|date| date.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        summary.symbols_on_freshest_market_date,
        summary.checked_symbols,
        if summary.freshest_market_date_complete { "yes" } else { "no" },
        summary.latest_gate_symbols_on_freshest_market_date,
        summary.latest_gate_checked_symbols,
        if summary.latest_gate_freshest_market_date_complete { "yes" } else { "no" },
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
            "- {} ({}) | status={} | latest_gate_required={} | rows={} | primary_ok={} | fallback_ok={:?} | gaps={} | max_gap_days={} | suspicious_jumps={} | max_abs_return={:.2}% | missing_turnover={}\n",
            row.symbol,
            row.name,
            row.status,
            if row.latest_gate_required { "yes" } else { "no" },
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
    use backtest_engine::BacktestSummary;
    use core_domain::{
        RegimeReason, RotationReason, SignalLabel, SignalReason, SignalSnapshot, StrategyKind,
    };

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
            top_signals: vec![SignalSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
                symbol: "510300".to_string(),
                final_score: 82.0,
                signal_label: SignalLabel::StrongBuy,
                analysis_scope: "GLOBAL".to_string(),
                regime_basis_scope: "GLOBAL".to_string(),
                reason: SignalReason {
                    best_strategy: StrategyKind::TrendBreakout,
                    strategy_score: 82.0,
                    strategy_contribution: 36.9,
                    alignment: 3,
                    aligned_strategies: vec![StrategyKind::TrendBreakout],
                    alignment_contribution: 9.0,
                    regime: RegimeReason {
                        trend_score: 72.0,
                        risk_score: 55.0,
                        combined_score: 63.5,
                        contribution: 12.7,
                    },
                    rotation: RotationReason {
                        momentum_score: 78.0,
                        rank: Some(1),
                        combined_score: 78.0,
                        contribution: 15.6,
                    },
                    final_score: 82.0,
                    label: SignalLabel::StrongBuy,
                    summary: "动量最强策略TrendBreakout得分82.0，趋势分63.5，轮动分78.0，最终信号StrongBuy".to_string(),
                },
            }],
            bullish_signals: Vec::new(),
            defensive_signals: Vec::new(),
            environment: Some(EnvironmentSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
                scope: "GLOBAL".to_string(),
                regime_as_of_date: NaiveDate::from_ymd_opt(2026, 3, 19).unwrap(),
                breadth_as_of_date: NaiveDate::from_ymd_opt(2026, 3, 20).unwrap(),
                stress_as_of_date: NaiveDate::from_ymd_opt(2026, 3, 19).unwrap(),
                breadth_eligible_count: 8,
                breadth_above_count: 6,
                breadth_pct: 75.0,
                breadth_pct_sma5: Some(66.0),
                breadth_5d_delta: Some(12.0),
                breadth_state: "near_local_high".to_string(),
                volume_expansion_pct: Some(62.5),
                turnover_coverage_pct: Some(87.5),
                liquidity_proxy_score: 70.0,
                stress_proxy_score: 55.0,
                environment_score: 68.0,
                environment_label: "constructive".to_string(),
            }),
            strategy_state: None,
            trust_summary: Some(TrustSummary {
                level: "review".to_string(),
                headline: "Use with review".to_string(),
                message: "Pipeline is usable, but signal/basis context and data-health warnings should be checked before acting.".to_string(),
                pipeline_has_partial_latest: true,
                pipeline_has_stale_stage: false,
                pipeline_partial_latest_stage_count: 1,
                pipeline_stale_stage_count: 0,
                freshest_market_date: Some("2026-03-20".to_string()),
                latest_available_date: Some("2026-03-20".to_string()),
                latest_day_complete: false,
                scoped_symbols_expected: 4,
                scoped_symbols_on_freshest_market_date: 3,
                macro_status: "review".to_string(),
                data_health_generated_at: Some("2026-03-20T12:00:00+00:00".to_string()),
                data_health_review_symbols: Some(2),
                data_health_critical_symbols: Some(0),
                data_health_review_macro_sources: Some(1),
                data_health_critical_macro_sources: Some(0),
                signal_analysis_scope: Some("GLOBAL".to_string()),
                signal_regime_basis_scope: Some("GLOBAL".to_string()),
                strategy_state: None,
                strategy_recommended_position_pct: None,
                backtest_matches_snapshot: Some(true),
                notes: vec!["Macro transport is degraded to fallback path.".to_string()],
            }),
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
            latest_backtest: Some(BacktestSummary {
                run_id: "bt-20260320".to_string(),
                strategy_name: "SIGNAL_PORTFOLIO_V1".to_string(),
                analysis_scope: "GLOBAL".to_string(),
                signal_scope: "GLOBAL".to_string(),
                regime_basis_scope: "GLOBAL".to_string(),
                signal_start_date: Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
                signal_end_date: Some(NaiveDate::from_ymd_opt(2026, 3, 20).unwrap()),
                config_summary:
                    "initial_capital=1000000, max_holdings=3, fee_rate=0.0010, slippage_rate=0.0005"
                        .to_string(),
                cagr: 0.18,
                max_drawdown: 0.09,
                sharpe: 1.12,
                final_equity: 1_120_000.0,
                trades: 14,
                trading_days: 14,
                drawdown_events: 0,
                state_trajectory: Vec::new(),
                run_version: "v1".to_string(),
                git_commit: "unknown".to_string(),
                generated_at: "2026-03-20 12:00:00 UTC".to_string(),
            }),
            load_metrics: None,
            symbol_names: HashMap::new(),
        };

        let rendered = render_markdown_report(&snapshot);

        assert!(rendered.contains("## Watchlist Breadth (MA30)"));
        assert!(rendered.contains("## Environment Layer"));
        assert!(rendered.contains("## Trust Summary"));
        assert!(rendered.contains("Level: review"));
        assert!(rendered.contains("Environment Score: 68.00"));
        assert!(rendered.contains("Signal Regime Basis: GLOBAL"));
        assert!(rendered.contains("Matches Current Snapshot: yes"));
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
            latest_gate_checked_symbols: 20,
            latest_gate_symbols_on_freshest_market_date: 20,
            latest_gate_symbols_missing_freshest_market_date: 0,
            latest_gate_freshest_market_date_complete: true,
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
        assert!(rendered.contains("Report-Gate Latest-Day Complete: yes"));
    }
}

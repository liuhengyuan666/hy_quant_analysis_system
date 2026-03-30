use anyhow::{Context, Result};
use backtest_engine::{run_signal_backtest, BacktestConfig};
use chrono::{Duration, NaiveDate, Utc};
use core_domain::{Instrument, InstrumentType, Market};
use data_ingestion::{
    fetch_daily_bars, fetch_eastmoney_daily_bars, fetch_fred_series, fetch_fred_series_with_status,
    fetch_tencent_daily_bars, load_universe,
};
use indicator_engine::build_indicator_snapshots;
use macro_engine::{build_macro_snapshots, build_market_regimes};
use market_store::StorageConfig;
use report_engine::{
    build_dashboard_snapshot_for_date, render_data_health_report, render_markdown_report,
    DashboardLoadMetrics, DashboardSnapshot, DataHealthMacroSourceSummary, DataHealthSummary,
    DataHealthSymbolSummary, WatchlistBreadthMarketSnapshot, WatchlistBreadthSnapshot,
};
use rotation_engine::build_rotation_ranks;
use serde::Serialize;
use signal_engine::build_signal_snapshots;
use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;
use strategy_engine::{build_strategy_preferences, AnalysisContext};

const CALENDAR_GAP_REVIEW_THRESHOLD_DAYS: i64 = 12;

fn format_error_chain(error: &anyhow::Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut current = error.source();
    while let Some(source) = current {
        parts.push(source.to_string());
        current = source.source();
    }
    parts.join(" | caused by: ")
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis() as u64
}

#[derive(Debug, Clone)]
pub struct AppContext {
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppStatus {
    pub clickhouse_url: String,
    pub clickhouse_database: String,
    pub sqlite_path: String,
    pub universe_path: String,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngestSummary {
    pub symbols: usize,
    pub rows: usize,
    pub from_date: String,
    pub to_date: String,
    pub failed_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndicatorSummary {
    pub symbols: usize,
    pub snapshots: usize,
    pub failed_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MacroSummary {
    pub factors: usize,
    pub macro_rows: usize,
    pub regime_rows: usize,
    pub failed_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RotationSummary {
    pub symbols: usize,
    pub rows: usize,
    pub failed_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategySummary {
    pub symbols: usize,
    pub rows: usize,
    pub failed_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalSummary {
    pub rows: usize,
    pub failed_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacktestRunSummary {
    pub run_id: String,
    pub strategy_name: String,
    pub cagr: f64,
    pub max_drawdown: f64,
    pub sharpe: f64,
    pub final_equity: f64,
    pub trades: usize,
    pub trading_days: usize,
    pub failed_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportSummary {
    pub report_date: String,
    pub output_path: String,
    pub failed_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentReportItem {
    pub report_type: String,
    pub report_date: String,
    pub artifact_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardLoadBundle {
    pub status: AppStatus,
    pub available_dates: Vec<String>,
    pub snapshot: Option<DashboardSnapshot>,
    pub recent_reports: Vec<RecentReportItem>,
    pub pipeline_dates: PipelineDateDiagnostics,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineStageDateStatus {
    pub stage: String,
    pub latest_date: Option<String>,
    pub lag_days: Option<i64>,
    pub is_latest: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineDateDiagnostics {
    pub freshest_market_date: Option<String>,
    pub dashboard_latest_date: Option<String>,
    pub stages: Vec<PipelineStageDateStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageGuide {
    pub id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshPlan {
    pub refresh_from: String,
    pub refresh_to: String,
    pub macro_from: String,
    pub macro_to: String,
}

fn analyze_gap_metrics(bars: &[core_domain::DailyBar]) -> (usize, i64) {
    let mut gap_count = 0usize;
    let mut max_gap_days = 0i64;
    for window in bars.windows(2) {
        let gap = (window[1].date - window[0].date).num_days();
        if gap > CALENDAR_GAP_REVIEW_THRESHOLD_DAYS {
            gap_count += 1;
            max_gap_days = max_gap_days.max(gap);
        }
    }
    (gap_count, max_gap_days)
}

fn analyze_jump_metrics(instrument: &Instrument, bars: &[core_domain::DailyBar]) -> (usize, f64) {
    let threshold = match instrument.instrument_type {
        InstrumentType::Index => 0.12,
        InstrumentType::Etf => 0.15,
    };
    let mut suspicious = 0usize;
    let mut max_abs_return = 0.0f64;
    for window in bars.windows(2) {
        let previous = window[0].close;
        let current = window[1].close;
        if previous <= 0.0 {
            continue;
        }
        let abs_return = ((current / previous) - 1.0).abs();
        max_abs_return = max_abs_return.max(abs_return * 100.0);
        if abs_return > threshold {
            suspicious += 1;
        }
    }
    (suspicious, max_abs_return)
}

fn classify_health(
    rows: usize,
    last_date: Option<NaiveDate>,
    now: NaiveDate,
    primary_provider_ok: bool,
    fallback_provider_ok: Option<bool>,
    gap_count: usize,
    suspicious_jump_count: usize,
) -> String {
    let freshness_days = last_date
        .map(|date| (now - date).num_days())
        .unwrap_or(i64::MAX);
    let has_recent_data = freshness_days <= 3;

    if rows == 0 {
        "critical".to_string()
    } else if !has_recent_data {
        "critical".to_string()
    } else if !primary_provider_ok && fallback_provider_ok != Some(true) {
        "review".to_string()
    } else if !primary_provider_ok || gap_count > 0 || suspicious_jump_count > 0 {
        "review".to_string()
    } else {
        "healthy".to_string()
    }
}

#[derive(Debug, Clone)]
struct TrackedInstrumentSeries {
    close_by_date: BTreeMap<NaiveDate, f64>,
    ma30_by_date: BTreeMap<NaiveDate, f64>,
}

#[derive(Debug, Clone)]
struct BreadthPoint {
    breadth_pct: f64,
    eligible_count: usize,
    above_count: usize,
}

fn market_code(market: &Market) -> &'static str {
    match market {
        Market::Cn => "CN",
        Market::Hk => "HK",
    }
}

fn market_universe_label(market: &Market) -> &'static str {
    match market {
        Market::Cn => "CN tracked universe",
        Market::Hk => "HK tracked universe",
    }
}

fn compute_breadth_point(series: &[TrackedInstrumentSeries], date: NaiveDate) -> BreadthPoint {
    let mut eligible_count = 0usize;
    let mut above_count = 0usize;

    for item in series {
        let Some(close) = item.close_by_date.get(&date).copied() else {
            continue;
        };
        let Some(ma30) = item.ma30_by_date.get(&date).copied() else {
            continue;
        };

        eligible_count += 1;
        if close > ma30 {
            above_count += 1;
        }
    }

    let breadth_pct = if eligible_count > 0 {
        above_count as f64 / eligible_count as f64 * 100.0
    } else {
        0.0
    };

    BreadthPoint {
        breadth_pct,
        eligible_count,
        above_count,
    }
}

fn compute_watchlist_breadth_status(
    eligible_count: usize,
    breadth_pct: f64,
    range_position_60d: Option<f64>,
    breadth_5d_delta: Option<f64>,
) -> String {
    if eligible_count == 0 {
        return "unavailable".to_string();
    }
    if let Some(position) = range_position_60d {
        if position <= 0.20 {
            return "near_local_low".to_string();
        }
        if position >= 0.80 {
            return "near_local_high".to_string();
        }
    }
    if let Some(delta) = breadth_5d_delta {
        if delta >= 10.0 {
            return "improving".to_string();
        }
        if delta <= -10.0 {
            return "weakening".to_string();
        }
    }
    if breadth_pct < 35.0 {
        "weak".to_string()
    } else if breadth_pct > 65.0 {
        "strong".to_string()
    } else {
        "neutral".to_string()
    }
}

fn build_market_watchlist_breadth_snapshot(
    market: Market,
    series: &[TrackedInstrumentSeries],
    report_date: NaiveDate,
    dashboard_dates: &[NaiveDate],
) -> WatchlistBreadthMarketSnapshot {
    let current = compute_breadth_point(series, report_date);
    let history = dashboard_dates
        .iter()
        .copied()
        .filter(|date| *date <= report_date)
        .filter_map(|date| {
            let point = compute_breadth_point(series, date);
            (point.eligible_count > 0).then_some(point)
        })
        .collect::<Vec<_>>();

    let breadth_pct_sma5 = (history.len() >= 5).then(|| {
        let window = &history[history.len() - 5..];
        window.iter().map(|point| point.breadth_pct).sum::<f64>() / window.len() as f64
    });
    let breadth_5d_delta = (history.len() >= 6).then(|| {
        let current = history[history.len() - 1].breadth_pct;
        let previous = history[history.len() - 6].breadth_pct;
        current - previous
    });
    let (range_low_60d, range_high_60d, range_position_60d) = if history.len() >= 60 {
        let window = &history[history.len() - 60..];
        let range_low = window
            .iter()
            .map(|point| point.breadth_pct)
            .fold(f64::INFINITY, f64::min);
        let range_high = window
            .iter()
            .map(|point| point.breadth_pct)
            .fold(f64::NEG_INFINITY, f64::max);
        let position = if (range_high - range_low).abs() < f64::EPSILON {
            Some(0.5)
        } else {
            Some(((current.breadth_pct - range_low) / (range_high - range_low)).clamp(0.0, 1.0))
        };
        (Some(range_low), Some(range_high), position)
    } else {
        (None, None, None)
    };

    let status_label = compute_watchlist_breadth_status(
        current.eligible_count,
        current.breadth_pct,
        range_position_60d,
        breadth_5d_delta,
    );

    WatchlistBreadthMarketSnapshot {
        market: market_code(&market).to_string(),
        universe_label: market_universe_label(&market).to_string(),
        eligible_count: current.eligible_count,
        above_count: current.above_count,
        breadth_pct: current.breadth_pct,
        breadth_pct_sma5,
        breadth_5d_delta,
        range_low_60d,
        range_high_60d,
        range_position_60d,
        status_label,
    }
}

impl AppContext {
    pub fn new(storage: StorageConfig) -> Self {
        Self { storage }
    }

    pub fn status(&self) -> Result<AppStatus> {
        Ok(AppStatus {
            clickhouse_url: self.storage.clickhouse_url.clone(),
            clickhouse_database: self.storage.clickhouse_database.clone(),
            sqlite_path: self.storage.sqlite_path.clone(),
            universe_path: self.storage.universe_path.clone(),
            profile: self.storage.profile.clone(),
        })
    }

    pub fn init_storage(&self) -> Result<()> {
        market_store::init_storage(&self.storage)
    }

    pub fn seed_universe(&self) -> Result<Vec<Instrument>> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        market_store::insert_instruments(&self.storage, &instruments)?;
        Ok(instruments)
    }

    pub fn ingest_daily(&self, from: NaiveDate, to: NaiveDate) -> Result<IngestSummary> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let mut total_rows = 0usize;
        let mut failed_symbols = Vec::new();
        for instrument in &instruments {
            let bars = match fetch_daily_bars(instrument, from, to) {
                Ok(bars) => bars,
                Err(error) => {
                    failed_symbols.push(format!("{}: {}", instrument.symbol, error));
                    continue;
                }
            };
            total_rows += bars.len();
            if let Err(error) =
                market_store::insert_daily_bars(&self.storage, &instrument.symbol, &bars)
            {
                failed_symbols.push(format!("{}: {}", instrument.symbol, error));
            }
        }
        Ok(IngestSummary {
            symbols: instruments.len(),
            rows: total_rows,
            from_date: from.to_string(),
            to_date: to.to_string(),
            failed_symbols,
        })
    }

    pub fn build_refresh_plan(&self, to: NaiveDate) -> Result<RefreshPlan> {
        let latest_daily_date = market_store::fetch_latest_daily_bar_date(&self.storage)?;
        let refresh_from = latest_daily_date
            .map(|date| date - Duration::days(7))
            .unwrap_or(to - Duration::days(730));
        let macro_from = to - Duration::days(550);

        Ok(RefreshPlan {
            refresh_from: refresh_from.to_string(),
            refresh_to: to.to_string(),
            macro_from: macro_from.to_string(),
            macro_to: to.to_string(),
        })
    }

    pub fn compute_indicators(&self) -> Result<IndicatorSummary> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let mut total_snapshots = 0usize;
        let mut failed_symbols = Vec::new();

        for instrument in &instruments {
            let bars = match market_store::fetch_daily_bars(&self.storage, &instrument.symbol) {
                Ok(bars) => bars,
                Err(error) => {
                    failed_symbols.push(format!("{}: {}", instrument.symbol, error));
                    continue;
                }
            };
            let snapshots = build_indicator_snapshots(&bars);
            total_snapshots += snapshots.len();
            if let Err(error) = market_store::insert_indicator_snapshots(
                &self.storage,
                &instrument.symbol,
                &snapshots,
            ) {
                failed_symbols.push(format!("{}: {}", instrument.symbol, error));
            }
        }

        Ok(IndicatorSummary {
            symbols: instruments.len(),
            snapshots: total_snapshots,
            failed_symbols,
        })
    }

    pub fn compute_macro_regime(&self, from: NaiveDate, to: NaiveDate) -> Result<MacroSummary> {
        let mut failed_items = Vec::new();
        let factor_specs = [
            ("vix", "VIXCLS", true),
            ("us10y", "DGS10", true),
            ("dollar_index", "DTWEXBGS", true),
            ("fed_funds", "DFF", true),
        ];

        let mut factors = Vec::new();
        for (name, series_id, invert) in factor_specs {
            match fetch_fred_series(name, series_id, invert, from, to) {
                Ok(series) => factors.push(series),
                Err(error) => failed_items.push(format!("{name}: {}", format_error_chain(&error))),
            }
        }

        let macro_rows = build_macro_snapshots(&factors, 20);
        if let Err(error) = market_store::insert_macro_snapshots(&self.storage, &macro_rows) {
            failed_items.push(format!("macro_snapshot: {}", format_error_chain(&error)));
        }

        let cn_anchor = market_store::fetch_daily_bars(&self.storage, "000300")
            .context("failed to load CN anchor daily bars")?;
        let hk_anchor = market_store::fetch_daily_bars(&self.storage, "HSI")
            .context("failed to load HK anchor daily bars")?;
        let regime_rows = build_market_regimes(&macro_rows, &cn_anchor, &hk_anchor);
        if let Err(error) = market_store::insert_market_regimes(&self.storage, &regime_rows) {
            failed_items.push(format!("market_regime: {}", format_error_chain(&error)));
        }

        Ok(MacroSummary {
            factors: factors.len(),
            macro_rows: macro_rows.len(),
            regime_rows: regime_rows.len(),
            failed_items,
        })
    }

    pub fn compute_rotation(&self) -> Result<RotationSummary> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let mut series_by_symbol = BTreeMap::new();
        let mut failed_symbols = Vec::new();

        for instrument in &instruments {
            match market_store::fetch_daily_bars(&self.storage, &instrument.symbol) {
                Ok(bars) => {
                    if !bars.is_empty() {
                        series_by_symbol.insert(instrument.symbol.clone(), bars);
                    }
                }
                Err(error) => failed_symbols.push(format!("{}: {}", instrument.symbol, error)),
            }
        }

        let rows = build_rotation_ranks(&series_by_symbol);
        if let Err(error) = market_store::insert_rotation_ranks(&self.storage, &rows) {
            failed_symbols.push(format!("rotation_rank: {error}"));
        }

        Ok(RotationSummary {
            symbols: series_by_symbol.len(),
            rows: rows.len(),
            failed_symbols,
        })
    }

    pub fn compute_strategy_preferences(&self) -> Result<StrategySummary> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let market_regimes = market_store::fetch_market_regimes(&self.storage)?;
        let rotation_rows = market_store::fetch_rotation_ranks(&self.storage)?;
        let regime_by_date = market_regimes
            .into_iter()
            .map(|row| (row.date, row))
            .collect::<BTreeMap<_, _>>();
        let rotation_by_key = rotation_rows
            .into_iter()
            .map(|row| ((row.date, row.symbol.clone()), row))
            .collect::<BTreeMap<_, _>>();

        let mut contexts = Vec::new();
        let mut failed_symbols = Vec::new();

        for instrument in &instruments {
            let bars = match market_store::fetch_daily_bars(&self.storage, &instrument.symbol) {
                Ok(bars) => bars,
                Err(error) => {
                    failed_symbols.push(format!("{}: {}", instrument.symbol, error));
                    continue;
                }
            };
            let indicators =
                match market_store::fetch_indicator_snapshots(&self.storage, &instrument.symbol) {
                    Ok(rows) => rows,
                    Err(error) => {
                        failed_symbols.push(format!("{}: {}", instrument.symbol, error));
                        continue;
                    }
                };
            let indicator_by_date = indicators
                .into_iter()
                .map(|row| (row.date, row))
                .collect::<BTreeMap<_, _>>();

            for bar in bars {
                let Some(indicators) = indicator_by_date.get(&bar.date).cloned() else {
                    continue;
                };
                let regime = regime_by_date.get(&bar.date).cloned();
                let rotation = rotation_by_key
                    .get(&(bar.date, instrument.symbol.clone()))
                    .cloned();
                contexts.push(AnalysisContext {
                    bar,
                    indicators,
                    regime,
                    rotation,
                });
            }
        }

        let rows = build_strategy_preferences(&contexts);
        if let Err(error) = market_store::insert_strategy_preferences(&self.storage, &rows) {
            failed_symbols.push(format!("strategy_preference: {error}"));
        }

        Ok(StrategySummary {
            symbols: instruments.len(),
            rows: rows.len(),
            failed_symbols,
        })
    }

    pub fn compute_signals(&self) -> Result<SignalSummary> {
        let strategies = market_store::fetch_strategy_preferences(&self.storage)?;
        let regimes = market_store::fetch_market_regimes(&self.storage)?;
        let rotations = market_store::fetch_rotation_ranks(&self.storage)?;
        let rows = build_signal_snapshots(&strategies, &regimes, &rotations);
        let mut failed_items = Vec::new();
        if let Err(error) = market_store::insert_signal_snapshots(&self.storage, &rows) {
            failed_items.push(format!("signal_snapshot: {error}"));
        }
        Ok(SignalSummary {
            rows: rows.len(),
            failed_items,
        })
    }

    pub fn run_backtest(
        &self,
        initial_capital: f64,
        max_holdings: usize,
        fee_rate: f64,
        slippage_rate: f64,
    ) -> Result<BacktestRunSummary> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let signals = market_store::fetch_signal_snapshots(&self.storage)?;
        let mut bars_by_symbol = BTreeMap::new();
        let mut failed_items = Vec::new();

        for instrument in &instruments {
            match market_store::fetch_daily_bars(&self.storage, &instrument.symbol) {
                Ok(bars) => {
                    if !bars.is_empty() {
                        bars_by_symbol.insert(instrument.symbol.clone(), bars);
                    }
                }
                Err(error) => failed_items.push(format!("{}: {}", instrument.symbol, error)),
            }
        }

        let run_id = format!("bt-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
        let config = BacktestConfig {
            strategy_name: "SIGNAL_PORTFOLIO_V1".to_string(),
            initial_capital,
            max_holdings,
            fee_rate,
            slippage_rate,
        };
        let result = run_signal_backtest(&run_id, &config, &signals, &bars_by_symbol);
        if let Err(error) = market_store::insert_backtest_result(
            &self.storage,
            &result.summary,
            &result.trades,
            &result.equity_curve,
        ) {
            failed_items.push(format!("backtest_persist: {error}"));
        }

        Ok(BacktestRunSummary {
            run_id: result.summary.run_id,
            strategy_name: result.summary.strategy_name,
            cagr: result.summary.cagr,
            max_drawdown: result.summary.max_drawdown,
            sharpe: result.summary.sharpe,
            final_equity: result.summary.final_equity,
            trades: result.summary.trades,
            trading_days: result.summary.trading_days,
            failed_items,
        })
    }

    pub fn dashboard_snapshot(
        &self,
        report_date: Option<NaiveDate>,
    ) -> Result<Option<DashboardSnapshot>> {
        let total_started_at = Instant::now();
        let available_dates_started_at = Instant::now();
        let available_dates = market_store::fetch_dashboard_available_dates(&self.storage)?;
        let available_dates_ms = elapsed_ms(available_dates_started_at);
        let (snapshot, mut metrics) =
            self.dashboard_snapshot_from_available_dates(report_date, &available_dates)?;
        metrics.available_dates_ms = available_dates_ms;
        metrics.total_ms = elapsed_ms(total_started_at);
        Ok(snapshot.map(|mut snapshot| {
            snapshot.load_metrics = Some(metrics);
            snapshot
        }))
    }

    pub fn dashboard_bundle(
        &self,
        report_date: Option<NaiveDate>,
        recent_report_limit: usize,
    ) -> Result<DashboardLoadBundle> {
        let total_started_at = Instant::now();
        let available_dates_started_at = Instant::now();
        let status = self.status()?;
        let available_dates = market_store::fetch_dashboard_available_dates(&self.storage)?;
        let available_dates_ms = elapsed_ms(available_dates_started_at);
        let (snapshot, mut metrics) =
            self.dashboard_snapshot_from_available_dates(report_date, &available_dates)?;
        let recent_reports = self.recent_reports(recent_report_limit)?;
        let pipeline_dates =
            self.pipeline_date_diagnostics_from_available_dates(&available_dates)?;
        metrics.available_dates_ms = available_dates_ms;
        metrics.total_ms = elapsed_ms(total_started_at);
        let snapshot = snapshot.map(|mut snapshot| {
            snapshot.load_metrics = Some(metrics);
            snapshot
        });

        Ok(DashboardLoadBundle {
            status,
            available_dates: available_dates
                .into_iter()
                .map(|date| date.to_string())
                .collect(),
            snapshot,
            recent_reports,
            pipeline_dates,
        })
    }

    pub fn pipeline_date_diagnostics(&self) -> Result<PipelineDateDiagnostics> {
        let available_dates = market_store::fetch_dashboard_available_dates(&self.storage)?;
        self.pipeline_date_diagnostics_from_available_dates(&available_dates)
    }

    fn dashboard_snapshot_from_available_dates(
        &self,
        report_date: Option<NaiveDate>,
        available_dates: &[NaiveDate],
    ) -> Result<(Option<DashboardSnapshot>, DashboardLoadMetrics)> {
        let zero_metrics = DashboardLoadMetrics {
            available_dates_ms: 0,
            regime_ms: 0,
            rotations_ms: 0,
            signals_ms: 0,
            backtest_ms: 0,
            breadth_ms: 0,
            assembly_ms: 0,
            total_ms: 0,
        };
        let Some(latest_available_date) = available_dates.first().copied() else {
            return Ok((None, zero_metrics));
        };
        let report_date = if let Some(date) = report_date {
            if available_dates.contains(&date) {
                date
            } else {
                return Ok((None, zero_metrics));
            }
        } else {
            latest_available_date
        };
        let regime_started_at = Instant::now();
        let regime =
            market_store::fetch_latest_market_regime_on_or_before(&self.storage, report_date)?
                .context("no market regime available for dashboard snapshot")?;
        let regime_ms = elapsed_ms(regime_started_at);
        let rotations_started_at = Instant::now();
        let rotations = market_store::fetch_rotation_ranks_for_date(&self.storage, report_date)?;
        let rotations_ms = elapsed_ms(rotations_started_at);
        let signals_started_at = Instant::now();
        let signals = market_store::fetch_signal_snapshots_for_date(&self.storage, report_date)?;
        let signals_ms = elapsed_ms(signals_started_at);
        let backtest_started_at = Instant::now();
        let latest_backtest = market_store::fetch_latest_backtest_run(&self.storage)?;
        let backtest_ms = elapsed_ms(backtest_started_at);
        let assembly_started_at = Instant::now();
        let mut snapshot = build_dashboard_snapshot_for_date(
            &regime,
            &rotations,
            &signals,
            latest_backtest,
            report_date,
            latest_available_date,
        );
        let assembly_ms = elapsed_ms(assembly_started_at);
        let breadth_started_at = Instant::now();
        snapshot.watchlist_breadth =
            self.compute_watchlist_breadth_snapshot(report_date, &available_dates)?;
        let breadth_ms = elapsed_ms(breadth_started_at);
        Ok((
            Some(snapshot),
            DashboardLoadMetrics {
                available_dates_ms: 0,
                regime_ms,
                rotations_ms,
                signals_ms,
                backtest_ms,
                breadth_ms,
                assembly_ms,
                total_ms: 0,
            },
        ))
    }

    pub fn dashboard_available_dates(&self) -> Result<Vec<String>> {
        Ok(
            market_store::fetch_dashboard_available_dates(&self.storage)?
                .into_iter()
                .map(|date| date.to_string())
                .collect(),
        )
    }

    fn pipeline_date_diagnostics_from_available_dates(
        &self,
        available_dates: &[NaiveDate],
    ) -> Result<PipelineDateDiagnostics> {
        let freshest_market_date =
            market_store::fetch_latest_table_date(&self.storage, "daily_bar")?;
        let dashboard_latest_date = available_dates.first().copied();
        let stage_rows = [
            ("daily_bar", freshest_market_date),
            (
                "indicator_snapshot",
                market_store::fetch_latest_table_date(&self.storage, "indicator_snapshot")?,
            ),
            (
                "market_regime",
                market_store::fetch_latest_table_date(&self.storage, "market_regime")?,
            ),
            (
                "rotation_rank",
                market_store::fetch_latest_table_date(&self.storage, "rotation_rank")?,
            ),
            (
                "strategy_preference",
                market_store::fetch_latest_table_date(&self.storage, "strategy_preference")?,
            ),
            (
                "signal_snapshot",
                market_store::fetch_latest_table_date(&self.storage, "signal_snapshot")?,
            ),
            ("dashboard_available", dashboard_latest_date),
        ];

        let stages = stage_rows
            .into_iter()
            .map(|(stage, latest_date)| PipelineStageDateStatus {
                stage: stage.to_string(),
                latest_date: latest_date.map(|date| date.to_string()),
                lag_days: match (freshest_market_date, latest_date) {
                    (Some(reference), Some(stage_date)) => Some((reference - stage_date).num_days()),
                    _ => None,
                },
                is_latest: matches!((freshest_market_date, latest_date), (Some(reference), Some(stage_date)) if reference == stage_date),
            })
            .collect();

        Ok(PipelineDateDiagnostics {
            freshest_market_date: freshest_market_date.map(|date| date.to_string()),
            dashboard_latest_date: dashboard_latest_date.map(|date| date.to_string()),
            stages,
        })
    }

    fn compute_watchlist_breadth_snapshot(
        &self,
        report_date: NaiveDate,
        dashboard_dates: &[NaiveDate],
    ) -> Result<Option<WatchlistBreadthSnapshot>> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let tracked_instruments = instruments
            .into_iter()
            .filter(|instrument| {
                instrument.enabled
                    && matches!(
                        instrument.instrument_type,
                        InstrumentType::Index | InstrumentType::Etf
                    )
            })
            .collect::<Vec<_>>();

        if tracked_instruments.is_empty() {
            return Ok(None);
        }

        let mut relevant_dates = dashboard_dates
            .iter()
            .copied()
            .filter(|date| *date <= report_date)
            .collect::<Vec<_>>();
        if relevant_dates.is_empty() {
            return Ok(None);
        }
        relevant_dates.sort_unstable();
        if relevant_dates.len() > 60 {
            relevant_dates = relevant_dates[relevant_dates.len() - 60..].to_vec();
        }
        let history_window_start = relevant_dates[0];
        let tracked_symbols = tracked_instruments
            .iter()
            .map(|instrument| instrument.symbol.clone())
            .collect::<Vec<_>>();
        let bars = market_store::fetch_daily_bars_for_symbols_in_range(
            &self.storage,
            &tracked_symbols,
            history_window_start,
            report_date,
        )?;
        let indicators = market_store::fetch_indicator_snapshots_for_symbols_in_range(
            &self.storage,
            &tracked_symbols,
            history_window_start,
            report_date,
        )?;

        let mut series_by_symbol = tracked_instruments
            .iter()
            .map(|instrument| {
                (
                    instrument.symbol.clone(),
                    TrackedInstrumentSeries {
                        close_by_date: BTreeMap::new(),
                        ma30_by_date: BTreeMap::new(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        for row in bars {
            if let Some(series) = series_by_symbol.get_mut(&row.symbol) {
                series.close_by_date.insert(row.date, row.close);
            }
        }
        for row in indicators {
            if let Some(ma30) = row.ma30 {
                if let Some(series) = series_by_symbol.get_mut(&row.symbol) {
                    series.ma30_by_date.insert(row.date, ma30);
                }
            }
        }

        let mut cn_series = Vec::new();
        let mut hk_series = Vec::new();

        for instrument in tracked_instruments {
            let Some(series) = series_by_symbol.remove(&instrument.symbol) else {
                continue;
            };

            match instrument.market {
                Market::Cn => cn_series.push(series),
                Market::Hk => hk_series.push(series),
            }
        }

        let methodology_note = "Eligible tracked instruments must be enabled INDEX/ETF universe members with both close and MA30 available on the selected date. Proxy only; not full-market stock breadth.".to_string();

        Ok(Some(WatchlistBreadthSnapshot {
            report_date: report_date.to_string(),
            markets: vec![
                build_market_watchlist_breadth_snapshot(
                    Market::Cn,
                    &cn_series,
                    report_date,
                    &relevant_dates,
                ),
                build_market_watchlist_breadth_snapshot(
                    Market::Hk,
                    &hk_series,
                    report_date,
                    &relevant_dates,
                ),
            ],
            methodology_note,
        }))
    }

    pub fn check_data_health(&self) -> Result<DataHealthSummary> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let now = Utc::now().date_naive();
        let probe_from = now - Duration::days(45);
        let macro_probe_from = now - Duration::days(400);
        let mut summaries = Vec::new();
        let mut macro_sources = Vec::new();

        for (factor_name, series_id, invert) in [
            ("vix", "VIXCLS", true),
            ("us10y", "DGS10", true),
            ("dollar_index", "DTWEXBGS", true),
            ("fed_funds", "DFF", true),
        ] {
            match fetch_fred_series_with_status(
                factor_name,
                series_id,
                invert,
                macro_probe_from,
                now,
            ) {
                Ok(outcome) => {
                    let status = if outcome.transport == "reqwest" {
                        "healthy"
                    } else {
                        "review"
                    }
                    .to_string();
                    let mut notes = Vec::new();
                    if outcome.transport != "reqwest" {
                        notes.push("宏观因子当前使用兼容性 fallback 获取".to_string());
                    }
                    macro_sources.push(DataHealthMacroSourceSummary {
                        factor_name: factor_name.to_string(),
                        source: "FRED".to_string(),
                        transport: outcome.transport,
                        rows: outcome.series.observations.len(),
                        first_date: outcome.series.observations.first().map(|(date, _)| *date),
                        last_date: outcome.series.observations.last().map(|(date, _)| *date),
                        status,
                        notes,
                    });
                }
                Err(error) => {
                    macro_sources.push(DataHealthMacroSourceSummary {
                        factor_name: factor_name.to_string(),
                        source: "FRED".to_string(),
                        transport: "failed".to_string(),
                        rows: 0,
                        first_date: None,
                        last_date: None,
                        status: "critical".to_string(),
                        notes: vec![format_error_chain(&error)],
                    });
                }
            }
        }

        for instrument in &instruments {
            let bars = market_store::fetch_daily_bars(&self.storage, &instrument.symbol)
                .unwrap_or_default();
            let primary_provider_ok = fetch_eastmoney_daily_bars(
                &instrument.symbol,
                &instrument.eastmoney_secid,
                probe_from,
                now,
            )
            .map(|rows| !rows.is_empty())
            .unwrap_or(false);
            let fallback_provider_ok = instrument.tencent_symbol.as_ref().map(|symbol| {
                fetch_tencent_daily_bars(&instrument.symbol, symbol, probe_from, now)
                    .map(|rows| !rows.is_empty())
                    .unwrap_or(false)
            });

            let (gap_count, max_gap_days) = analyze_gap_metrics(&bars);
            let (suspicious_jump_count, max_abs_daily_return_pct) =
                analyze_jump_metrics(instrument, &bars);
            let missing_turnover_rows = bars.iter().filter(|bar| bar.turnover.is_none()).count();

            let mut notes = Vec::new();
            if !primary_provider_ok {
                notes.push("Eastmoney 当前探测失败或无返回".to_string());
            }
            if let Some(false) = fallback_provider_ok {
                notes.push("Tencent fallback 当前探测失败或无返回".to_string());
            }
            if gap_count > 0 {
                notes.push(format!(
                    "存在 {} 个大于 {} 天的时间缺口",
                    gap_count, CALENDAR_GAP_REVIEW_THRESHOLD_DAYS
                ));
            }
            if suspicious_jump_count > 0 {
                notes.push(format!(
                    "检测到 {} 个可疑大波动日，最大绝对涨跌幅 {:.2}%",
                    suspicious_jump_count, max_abs_daily_return_pct
                ));
            }
            if missing_turnover_rows > 0 {
                notes.push(format!("有 {} 根 bar 缺少 turnover", missing_turnover_rows));
            }

            let status = classify_health(
                bars.len(),
                bars.last().map(|bar| bar.date),
                now,
                primary_provider_ok,
                fallback_provider_ok,
                gap_count,
                suspicious_jump_count,
            );

            summaries.push(DataHealthSymbolSummary {
                symbol: instrument.symbol.clone(),
                name: instrument.name.clone(),
                display_symbol: instrument.display_symbol.clone(),
                rows: bars.len(),
                first_date: bars.first().map(|bar| bar.date),
                last_date: bars.last().map(|bar| bar.date),
                primary_provider_ok,
                fallback_provider_ok,
                missing_turnover_rows,
                gap_count,
                max_gap_days,
                suspicious_jump_count,
                max_abs_daily_return_pct,
                status,
                notes,
            });
        }

        summaries.sort_by(|left, right| {
            left.status
                .cmp(&right.status)
                .then_with(|| left.symbol.cmp(&right.symbol))
        });

        let healthy_symbols = summaries
            .iter()
            .filter(|row| row.status == "healthy")
            .count();
        let review_symbols = summaries
            .iter()
            .filter(|row| row.status == "review")
            .count();
        let critical_symbols = summaries
            .iter()
            .filter(|row| row.status == "critical")
            .count();
        let healthy_macro_sources = macro_sources
            .iter()
            .filter(|row| row.status == "healthy")
            .count();
        let review_macro_sources = macro_sources
            .iter()
            .filter(|row| row.status == "review")
            .count();
        let critical_macro_sources = macro_sources
            .iter()
            .filter(|row| row.status == "critical")
            .count();

        Ok(DataHealthSummary {
            generated_at: Utc::now().to_rfc3339(),
            canonical_adjustment: "forward-adjusted daily bars (Eastmoney fqt=1, Tencent qfq)"
                .to_string(),
            checked_symbols: summaries.len(),
            healthy_symbols,
            review_symbols,
            critical_symbols,
            healthy_macro_sources,
            review_macro_sources,
            critical_macro_sources,
            macro_sources,
            symbols: summaries,
        })
    }

    pub fn export_report(&self, report_date: Option<NaiveDate>) -> Result<ReportSummary> {
        let snapshot = self
            .dashboard_snapshot(report_date)?
            .context("no dashboard snapshot available for report export")?;
        let markdown = render_markdown_report(&snapshot);
        let root = StorageConfig::project_root()?;
        let report_dir = root.join("reports");
        fs::create_dir_all(&report_dir).with_context(|| {
            format!(
                "failed to create report directory: {}",
                report_dir.display()
            )
        })?;
        let output_path = report_dir.join(format!("daily-report-{}.md", snapshot.report_date));
        fs::write(&output_path, markdown)
            .with_context(|| format!("failed to write report file: {}", output_path.display()))?;
        market_store::insert_report_snapshot(
            &self.storage,
            &snapshot.report_date,
            "DAILY_REPORT",
            &output_path.display().to_string(),
        )?;
        Ok(ReportSummary {
            report_date: snapshot.report_date,
            output_path: output_path.display().to_string(),
            failed_items: Vec::new(),
        })
    }

    pub fn export_data_health_report(&self) -> Result<ReportSummary> {
        let summary = self.check_data_health()?;
        let markdown = render_data_health_report(&summary);
        let root = StorageConfig::project_root()?;
        let report_dir = root.join("reports");
        fs::create_dir_all(&report_dir).with_context(|| {
            format!(
                "failed to create report directory: {}",
                report_dir.display()
            )
        })?;
        let report_date = Utc::now().date_naive().to_string();
        let output_path = report_dir.join(format!("data-health-{}.md", report_date));
        fs::write(&output_path, markdown)
            .with_context(|| format!("failed to write report file: {}", output_path.display()))?;
        market_store::insert_report_snapshot(
            &self.storage,
            &report_date,
            "DATA_HEALTH_REPORT",
            &output_path.display().to_string(),
        )?;
        Ok(ReportSummary {
            report_date,
            output_path: output_path.display().to_string(),
            failed_items: Vec::new(),
        })
    }

    pub fn recent_reports(&self, limit: usize) -> Result<Vec<RecentReportItem>> {
        Ok(
            market_store::fetch_recent_report_snapshots(&self.storage, limit)?
                .into_iter()
                .map(
                    |(report_type, report_date, artifact_path)| RecentReportItem {
                        report_type,
                        report_date,
                        artifact_path,
                    },
                )
                .collect(),
        )
    }

    pub fn usage_guides(&self) -> Result<Vec<UsageGuide>> {
        let root = StorageConfig::project_root()?;
        let guides = [
            (
                "daily-ops",
                "日常操作手册",
                root.join("docs").join("日常操作手册.md"),
            ),
            (
                "analysis-guide",
                "分析使用手册",
                root.join("docs").join("分析使用手册.md"),
            ),
        ];

        guides
            .into_iter()
            .map(|(id, title, path)| {
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read usage guide: {}", path.display()))?;
                Ok(UsageGuide {
                    id: id.to_string(),
                    title: title.to_string(),
                    content,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_series(points: &[(NaiveDate, f64, f64)]) -> TrackedInstrumentSeries {
        TrackedInstrumentSeries {
            close_by_date: points
                .iter()
                .map(|(date, close, _)| (*date, *close))
                .collect::<BTreeMap<_, _>>(),
            ma30_by_date: points
                .iter()
                .map(|(date, _, ma30)| (*date, *ma30))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    #[test]
    fn watchlist_breadth_status_returns_unavailable_without_eligible_symbols() {
        let status = compute_watchlist_breadth_status(0, 0.0, None, None);
        assert_eq!(status, "unavailable");
    }

    #[test]
    fn watchlist_breadth_status_prioritizes_range_position_over_delta() {
        let status = compute_watchlist_breadth_status(4, 55.0, Some(0.85), Some(-15.0));
        assert_eq!(status, "near_local_high");
    }

    #[test]
    fn market_watchlist_breadth_snapshot_computes_current_ratio_and_history_metrics() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let dates = (0..6)
            .map(|offset| start + Duration::days(offset))
            .collect::<Vec<_>>();
        let always_above = build_series(
            &dates
                .iter()
                .map(|date| (*date, 110.0, 100.0))
                .collect::<Vec<_>>(),
        );
        let recovering = build_series(&[
            (dates[0], 90.0, 100.0),
            (dates[1], 90.0, 100.0),
            (dates[2], 90.0, 100.0),
            (dates[3], 90.0, 100.0),
            (dates[4], 90.0, 100.0),
            (dates[5], 110.0, 100.0),
        ]);

        let snapshot = build_market_watchlist_breadth_snapshot(
            Market::Cn,
            &[always_above, recovering],
            dates[5],
            &dates,
        );

        assert_eq!(snapshot.market, "CN");
        assert_eq!(snapshot.universe_label, "CN tracked universe");
        assert_eq!(snapshot.eligible_count, 2);
        assert_eq!(snapshot.above_count, 2);
        assert!((snapshot.breadth_pct - 100.0).abs() < f64::EPSILON);
        assert_eq!(snapshot.status_label, "improving");
        assert_eq!(snapshot.range_position_60d, None);
        assert_eq!(snapshot.range_low_60d, None);
        assert_eq!(snapshot.range_high_60d, None);
        let sma5 = snapshot.breadth_pct_sma5.unwrap();
        assert!((sma5 - 60.0).abs() < 1e-9);
        let delta = snapshot.breadth_5d_delta.unwrap();
        assert!((delta - 50.0).abs() < 1e-9);
    }
}

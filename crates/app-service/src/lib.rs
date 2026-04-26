use anyhow::{Context, Result};
use backtest_engine::{run_signal_backtest, BacktestConfig};
use chrono::{Duration, NaiveDate, Utc};
use core_domain::{EnvironmentSnapshot, Instrument, InstrumentType, Market};
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
    DataHealthSymbolSummary, TrustSummary, WatchlistBreadthMarketSnapshot,
    WatchlistBreadthSnapshot,
};
use rotation_engine::build_rotation_ranks;
use serde::Serialize;
use signal_engine::build_signal_snapshots;
use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;
use strategy_engine::{build_strategy_preferences, AnalysisContext};

pub use core_domain::AnalysisScope as ReportScope;

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
    pub environment_rows: usize,
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
    pub latest_entities: Option<usize>,
    pub expected_entities: Option<usize>,
    pub is_complete: Option<bool>,
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

fn build_trust_summary(
    scoped_symbols: &std::collections::BTreeSet<String>,
    snapshot: &DashboardSnapshot,
    pipeline_dates: &PipelineDateDiagnostics,
    data_health: &DataHealthSummary,
) -> TrustSummary {
    let scoped_symbols_expected = scoped_symbols.len();
    let scoped_symbols_on_freshest_market_date = data_health
        .freshest_market_date
        .map(|date| {
            data_health
                .symbols
                .iter()
                .filter(|row| scoped_symbols.contains(&row.symbol) && row.last_date == Some(date))
                .count()
        })
        .unwrap_or(0);
    let latest_day_complete = scoped_symbols_expected > 0
        && scoped_symbols_on_freshest_market_date == scoped_symbols_expected;
    let macro_status = if data_health.critical_macro_sources > 0 {
        "critical"
    } else if data_health.review_macro_sources > 0 {
        "review"
    } else {
        "healthy"
    }
    .to_string();

    let signal_analysis_scope = snapshot
        .top_signals
        .first()
        .map(|row| row.analysis_scope.clone());
    let signal_regime_basis_scope = snapshot
        .top_signals
        .first()
        .map(|row| row.regime_basis_scope.clone());
    let backtest_matches_snapshot = snapshot.latest_backtest.as_ref().map(|backtest| {
        backtest
            .analysis_scope
            .eq_ignore_ascii_case(&snapshot.scope)
            && backtest.signal_scope.eq_ignore_ascii_case(&snapshot.scope)
            && backtest
                .signal_end_date
                .map(|date| date.to_string())
                .as_deref()
                == Some(snapshot.report_date.as_str())
    });

    let pipeline_partial_latest = pipeline_dates
        .stages
        .iter()
        .any(|stage| stage.is_latest && stage.is_complete == Some(false));
    let pipeline_partial_latest_stage_count = pipeline_dates
        .stages
        .iter()
        .filter(|stage| stage.is_latest && stage.is_complete == Some(false))
        .count();
    let pipeline_stale = pipeline_dates.stages.iter().any(|stage| {
        matches!(stage.lag_days, Some(lag) if lag > 0)
            && matches!(
                stage.stage.as_str(),
                "market_regime"
                    | "environment_snapshot"
                    | "strategy_preference"
                    | "signal_snapshot"
            )
    });
    let pipeline_stale_stage_count = pipeline_dates
        .stages
        .iter()
        .filter(|stage| {
            matches!(stage.lag_days, Some(lag) if lag > 0)
                && matches!(
                    stage.stage.as_str(),
                    "market_regime"
                        | "environment_snapshot"
                        | "strategy_preference"
                        | "signal_snapshot"
                )
        })
        .count();

    let mut notes = Vec::new();
    if !latest_day_complete {
        notes.push(
            "Latest market date is not fully covered across the active universe.".to_string(),
        );
    }
    if data_health.review_macro_sources > 0 {
        notes.push(
            "One or more macro sources are currently using review/fallback transport.".to_string(),
        );
    }
    if data_health.critical_macro_sources > 0 {
        notes.push("One or more macro sources are currently unavailable.".to_string());
    }
    if pipeline_partial_latest {
        notes.push("At least one pipeline stage is only partially complete on the freshest available date.".to_string());
    }
    if pipeline_stale {
        notes.push(
            "One or more decision stages are lagging behind the freshest market date.".to_string(),
        );
    }
    if let (Some(signal_scope), Some(regime_scope)) = (
        signal_analysis_scope.as_ref(),
        signal_regime_basis_scope.as_ref(),
    ) {
        if !signal_scope.eq_ignore_ascii_case(&snapshot.scope)
            || !regime_scope.eq_ignore_ascii_case(&snapshot.scope)
        {
            notes.push(format!(
                "Signal analysis/regime basis still points to {} / {} while dashboard scope is {}.",
                signal_scope, regime_scope, snapshot.scope
            ));
        }
    }
    if backtest_matches_snapshot == Some(false) {
        notes.push(
            "Latest backtest does not match the current dashboard snapshot scope/date.".to_string(),
        );
    }

    let (level, headline, message) = if data_health.critical_macro_sources > 0 || pipeline_stale {
        (
            "degraded",
            "Use with caution",
            "The current research view is usable, but freshness or macro availability issues reduce trust in the latest outputs.",
        )
    } else if !latest_day_complete
        || data_health.review_macro_sources > 0
        || pipeline_partial_latest
        || backtest_matches_snapshot == Some(false)
    {
        (
            "review",
            "Review before acting",
            "The pipeline completed, but coverage/provenance caveats should be reviewed before treating this as a clean research snapshot.",
        )
    } else {
        (
            "trusted",
            "Ready for analysis",
            "Freshness, provenance, and macro transport checks currently look healthy for this snapshot.",
        )
    };

    TrustSummary {
        level: level.to_string(),
        headline: headline.to_string(),
        message: message.to_string(),
        pipeline_has_partial_latest: pipeline_partial_latest,
        pipeline_has_stale_stage: pipeline_stale,
        pipeline_partial_latest_stage_count,
        pipeline_stale_stage_count,
        freshest_market_date: data_health
            .freshest_market_date
            .map(|date| date.to_string()),
        latest_available_date: Some(snapshot.latest_available_date.clone()),
        latest_day_complete,
        scoped_symbols_expected,
        scoped_symbols_on_freshest_market_date,
        macro_status,
        data_health_generated_at: Some(data_health.generated_at.clone()),
        data_health_review_symbols: data_health.review_symbols,
        data_health_critical_symbols: data_health.critical_symbols,
        data_health_review_macro_sources: data_health.review_macro_sources,
        data_health_critical_macro_sources: data_health.critical_macro_sources,
        signal_analysis_scope,
        signal_regime_basis_scope,
        backtest_matches_snapshot,
        notes,
    }
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
    volume_by_date: BTreeMap<NaiveDate, f64>,
    turnover_present_by_date: BTreeMap<NaiveDate, bool>,
    ma30_by_date: BTreeMap<NaiveDate, f64>,
    vol_ma20_by_date: BTreeMap<NaiveDate, f64>,
}

#[derive(Debug, Clone)]
struct ParticipationPoint {
    breadth_pct: f64,
    eligible_count: usize,
    above_count: usize,
    volume_expansion_pct: Option<f64>,
    turnover_coverage_pct: Option<f64>,
    liquidity_proxy_score: f64,
}

#[derive(Debug, Clone)]
struct ParticipationMetrics {
    current: ParticipationPoint,
    breadth_pct_sma5: Option<f64>,
    breadth_5d_delta: Option<f64>,
    range_low_60d: Option<f64>,
    range_high_60d: Option<f64>,
    range_position_60d: Option<f64>,
    breadth_state: String,
}

#[derive(Debug, Clone)]
struct TrackedUniverseWindow {
    relevant_dates: Vec<NaiveDate>,
    cn_series: Vec<TrackedInstrumentSeries>,
    hk_series: Vec<TrackedInstrumentSeries>,
}

fn scope_label(scope: ReportScope) -> &'static str {
    scope.as_str()
}

fn instrument_in_scope(instrument: &Instrument, scope: ReportScope) -> bool {
    scope.matches_market(&instrument.market)
}

fn instrument_in_latest_gate_scope(instrument: &Instrument, scope: ReportScope) -> bool {
    instrument.enabled && instrument.latest_gate_required && instrument_in_scope(instrument, scope)
}

fn scope_universe_label(scope: ReportScope) -> &'static str {
    match scope {
        ReportScope::Global => "Global tracked universe",
        ReportScope::Cn => "CN tracked universe",
        ReportScope::Hk => "HK tracked universe",
    }
}

fn compute_participation_point(
    series: &[TrackedInstrumentSeries],
    date: NaiveDate,
) -> ParticipationPoint {
    let mut eligible_count = 0usize;
    let mut above_count = 0usize;
    let mut liquidity_eligible_count = 0usize;
    let mut volume_expansion_count = 0usize;
    let mut turnover_present_count = 0usize;

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
        if let (Some(volume), Some(vol_ma20)) = (
            item.volume_by_date.get(&date).copied(),
            item.vol_ma20_by_date.get(&date).copied(),
        ) {
            liquidity_eligible_count += 1;
            if volume > vol_ma20 {
                volume_expansion_count += 1;
            }
        }
        if item
            .turnover_present_by_date
            .get(&date)
            .copied()
            .unwrap_or(false)
        {
            turnover_present_count += 1;
        }
    }

    let breadth_pct = if eligible_count > 0 {
        above_count as f64 / eligible_count as f64 * 100.0
    } else {
        0.0
    };

    let volume_expansion_pct = (liquidity_eligible_count > 0)
        .then(|| volume_expansion_count as f64 / liquidity_eligible_count as f64 * 100.0);
    let turnover_coverage_pct =
        (eligible_count > 0).then(|| turnover_present_count as f64 / eligible_count as f64 * 100.0);
    let liquidity_proxy_score = match (volume_expansion_pct, turnover_coverage_pct) {
        (Some(volume_pct), Some(turnover_pct)) => volume_pct * 0.7 + turnover_pct * 0.3,
        (Some(volume_pct), None) => volume_pct,
        (None, Some(turnover_pct)) => turnover_pct,
        (None, None) => 50.0,
    };

    ParticipationPoint {
        breadth_pct,
        eligible_count,
        above_count,
        volume_expansion_pct,
        turnover_coverage_pct,
        liquidity_proxy_score,
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
    scope: ReportScope,
    series: &[TrackedInstrumentSeries],
    report_date: NaiveDate,
    relevant_dates: &[NaiveDate],
) -> WatchlistBreadthMarketSnapshot {
    let metrics = compute_participation_metrics(series, report_date, relevant_dates);

    WatchlistBreadthMarketSnapshot {
        market: scope_label(scope).to_string(),
        universe_label: scope_universe_label(scope).to_string(),
        eligible_count: metrics.current.eligible_count,
        above_count: metrics.current.above_count,
        breadth_pct: metrics.current.breadth_pct,
        breadth_pct_sma5: metrics.breadth_pct_sma5,
        breadth_5d_delta: metrics.breadth_5d_delta,
        range_low_60d: metrics.range_low_60d,
        range_high_60d: metrics.range_high_60d,
        range_position_60d: metrics.range_position_60d,
        status_label: metrics.breadth_state,
    }
}

fn compute_participation_metrics(
    series: &[TrackedInstrumentSeries],
    report_date: NaiveDate,
    relevant_dates: &[NaiveDate],
) -> ParticipationMetrics {
    let current = compute_participation_point(series, report_date);
    let history = relevant_dates
        .iter()
        .copied()
        .filter(|date| *date <= report_date)
        .filter_map(|date| {
            let point = compute_participation_point(series, date);
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

    let breadth_state = compute_watchlist_breadth_status(
        current.eligible_count,
        current.breadth_pct,
        range_position_60d,
        breadth_5d_delta,
    );

    ParticipationMetrics {
        current,
        breadth_pct_sma5,
        breadth_5d_delta,
        range_low_60d,
        range_high_60d,
        range_position_60d,
        breadth_state,
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

    fn latest_gate_instruments_for_scope(&self, scope: ReportScope) -> Result<Vec<Instrument>> {
        Ok(load_universe(&self.storage.universe_abspath()?)?
            .into_iter()
            .filter(|instrument| instrument_in_latest_gate_scope(instrument, scope))
            .collect())
    }

    fn build_tracked_universe_window(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<TrackedUniverseWindow> {
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
            return Ok(TrackedUniverseWindow {
                relevant_dates: Vec::new(),
                cn_series: Vec::new(),
                hk_series: Vec::new(),
            });
        }

        let tracked_symbols = tracked_instruments
            .iter()
            .map(|instrument| instrument.symbol.clone())
            .collect::<Vec<_>>();
        let bars = market_store::fetch_daily_bars_for_symbols_in_range(
            &self.storage,
            &tracked_symbols,
            from,
            to,
        )?;
        let indicators = market_store::fetch_indicator_snapshots_for_symbols_in_range(
            &self.storage,
            &tracked_symbols,
            from,
            to,
        )?;

        let mut relevant_dates = bars
            .iter()
            .map(|row| row.date)
            .collect::<std::collections::BTreeSet<_>>();
        let mut series_by_symbol = tracked_instruments
            .iter()
            .map(|instrument| {
                (
                    instrument.symbol.clone(),
                    TrackedInstrumentSeries {
                        close_by_date: BTreeMap::new(),
                        volume_by_date: BTreeMap::new(),
                        turnover_present_by_date: BTreeMap::new(),
                        ma30_by_date: BTreeMap::new(),
                        vol_ma20_by_date: BTreeMap::new(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        for row in bars {
            if let Some(series) = series_by_symbol.get_mut(&row.symbol) {
                relevant_dates.insert(row.date);
                series.close_by_date.insert(row.date, row.close);
                series.volume_by_date.insert(row.date, row.volume);
                series
                    .turnover_present_by_date
                    .insert(row.date, row.turnover.is_some());
            }
        }
        for row in indicators {
            if let Some(series) = series_by_symbol.get_mut(&row.symbol) {
                if let Some(ma30) = row.ma30 {
                    series.ma30_by_date.insert(row.date, ma30);
                }
                if let Some(vol_ma20) = row.vol_ma20 {
                    series.vol_ma20_by_date.insert(row.date, vol_ma20);
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

        Ok(TrackedUniverseWindow {
            relevant_dates: relevant_dates.into_iter().collect(),
            cn_series,
            hk_series,
        })
    }

    fn series_for_scope(
        window: &TrackedUniverseWindow,
        scope: ReportScope,
    ) -> Vec<TrackedInstrumentSeries> {
        match scope {
            ReportScope::Global => window
                .cn_series
                .iter()
                .chain(window.hk_series.iter())
                .cloned()
                .collect(),
            ReportScope::Cn => window.cn_series.clone(),
            ReportScope::Hk => window.hk_series.clone(),
        }
    }

    fn breadth_momentum_score(delta: Option<f64>) -> f64 {
        match delta {
            Some(value) if value >= 10.0 => 70.0,
            Some(value) if value >= 3.0 => 60.0,
            Some(value) if value <= -10.0 => 25.0,
            Some(value) if value <= -3.0 => 40.0,
            Some(_) => 50.0,
            None => 45.0,
        }
    }

    fn environment_label(score: f64) -> &'static str {
        if score >= 70.0 {
            "supportive"
        } else if score >= 55.0 {
            "constructive"
        } else if score >= 40.0 {
            "mixed"
        } else if score >= 25.0 {
            "fragile"
        } else {
            "stressed"
        }
    }

    fn build_environment_snapshots(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        regimes: &[core_domain::MarketRegimeSnapshot],
    ) -> Result<Vec<EnvironmentSnapshot>> {
        let history_start = from - Duration::days(180);
        let window = self.build_tracked_universe_window(history_start, to)?;
        let regime_by_key = regimes
            .iter()
            .map(|row| ((row.market.clone(), row.date), row))
            .collect::<BTreeMap<_, _>>();
        let mut rows = Vec::new();

        for scope in [ReportScope::Global, ReportScope::Cn, ReportScope::Hk] {
            let scoped_series = Self::series_for_scope(&window, scope);
            if scoped_series.is_empty() {
                continue;
            }
            for date in window
                .relevant_dates
                .iter()
                .copied()
                .filter(|date| *date >= from && *date <= to)
            {
                let Some(regime) = regime_by_key
                    .get(&(scope_label(scope).to_string(), date))
                    .copied()
                else {
                    continue;
                };
                let metrics =
                    compute_participation_metrics(&scoped_series, date, &window.relevant_dates);
                let breadth_momentum_score = Self::breadth_momentum_score(metrics.breadth_5d_delta);
                let environment_score = (regime.trend_score * 0.35
                    + metrics.current.breadth_pct * 0.25
                    + breadth_momentum_score * 0.15
                    + metrics.current.liquidity_proxy_score * 0.15
                    + regime.risk_score * 0.10)
                    .clamp(0.0, 100.0);
                rows.push(EnvironmentSnapshot {
                    date,
                    scope: scope_label(scope).to_string(),
                    regime_as_of_date: regime.macro_as_of_date,
                    breadth_as_of_date: date,
                    stress_as_of_date: regime.macro_as_of_date,
                    breadth_eligible_count: metrics.current.eligible_count,
                    breadth_above_count: metrics.current.above_count,
                    breadth_pct: metrics.current.breadth_pct,
                    breadth_pct_sma5: metrics.breadth_pct_sma5,
                    breadth_5d_delta: metrics.breadth_5d_delta,
                    breadth_state: metrics.breadth_state,
                    volume_expansion_pct: metrics.current.volume_expansion_pct,
                    turnover_coverage_pct: metrics.current.turnover_coverage_pct,
                    liquidity_proxy_score: metrics.current.liquidity_proxy_score,
                    stress_proxy_score: regime.risk_score,
                    environment_score,
                    environment_label: Self::environment_label(environment_score).to_string(),
                });
            }
        }

        Ok(rows)
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
        let macro_fetch_from = from - Duration::days(550);
        let factor_specs = [
            ("vix", "VIXCLS", true),
            ("us10y", "DGS10", true),
            ("dollar_index", "DTWEXBGS", true),
            ("fed_funds", "DFF", true),
        ];

        let mut factors = Vec::new();
        for (name, series_id, invert) in factor_specs {
            match fetch_fred_series(name, series_id, invert, macro_fetch_from, to) {
                Ok(series) => factors.push(series),
                Err(error) => failed_items.push(format!("{name}: {}", format_error_chain(&error))),
            }
        }

        let fetched_macro_rows = build_macro_snapshots(&factors, 20);
        let persisted_macro_rows =
            market_store::fetch_macro_snapshots_in_range(&self.storage, macro_fetch_from, to)
                .unwrap_or_default();
        let mut all_macro_rows_by_key = persisted_macro_rows
            .into_iter()
            .map(|row| ((row.date, row.factor_name.clone()), row))
            .collect::<BTreeMap<_, _>>();
        for row in fetched_macro_rows {
            all_macro_rows_by_key.insert((row.date, row.factor_name.clone()), row);
        }
        let all_macro_rows = all_macro_rows_by_key.into_values().collect::<Vec<_>>();

        let macro_rows = all_macro_rows
            .iter()
            .filter(|row| row.date >= from && row.date <= to)
            .cloned()
            .collect::<Vec<_>>();
        let fetched_macro_rows_in_range = factors
            .iter()
            .flat_map(|factor| build_macro_snapshots(std::slice::from_ref(factor), 20))
            .filter(|row| row.date >= from && row.date <= to)
            .collect::<Vec<_>>();
        if let Err(error) =
            market_store::insert_macro_snapshots(&self.storage, &fetched_macro_rows_in_range)
        {
            failed_items.push(format!("macro_snapshot: {}", format_error_chain(&error)));
        }

        let cn_anchor = market_store::fetch_daily_bars(&self.storage, "000300")
            .context("failed to load CN anchor daily bars")?;
        let hk_anchor = market_store::fetch_daily_bars(&self.storage, "HSI")
            .context("failed to load HK anchor daily bars")?;
        let regime_rows = build_market_regimes(&all_macro_rows, &cn_anchor, &hk_anchor)
            .into_iter()
            .filter(|row| row.date >= from && row.date <= to)
            .collect::<Vec<_>>();
        if let Err(error) = market_store::insert_market_regimes(&self.storage, &regime_rows) {
            failed_items.push(format!("market_regime: {}", format_error_chain(&error)));
        }
        let environment_rows = self.build_environment_snapshots(from, to, &regime_rows)?;
        if let Err(error) =
            market_store::insert_environment_snapshots(&self.storage, &environment_rows)
        {
            failed_items.push(format!(
                "environment_snapshot: {}",
                format_error_chain(&error)
            ));
        }

        Ok(MacroSummary {
            factors: factors.len(),
            macro_rows: macro_rows.len(),
            regime_rows: regime_rows.len(),
            environment_rows: environment_rows.len(),
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
            .map(|row| ((row.date, row.market.clone()), row))
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

            for scope in [ReportScope::Global, ReportScope::Cn, ReportScope::Hk] {
                if !instrument_in_scope(instrument, scope) {
                    continue;
                }
                for bar in &bars {
                    let Some(indicators) = indicator_by_date.get(&bar.date).cloned() else {
                        continue;
                    };
                    let regime = regime_by_date
                        .get(&(bar.date, scope_label(scope).to_string()))
                        .cloned();
                    let rotation = rotation_by_key
                        .get(&(bar.date, instrument.symbol.clone()))
                        .cloned();
                    contexts.push(AnalysisContext {
                        bar: bar.clone(),
                        indicators,
                        regime,
                        rotation,
                        analysis_scope: scope_label(scope).to_string(),
                        regime_basis_scope: scope_label(scope).to_string(),
                    });
                }
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
        scope: ReportScope,
    ) -> Result<BacktestRunSummary> {
        let instruments = self.instruments_for_scope(scope)?;
        let signals = market_store::fetch_signal_snapshots_with_scope(&self.storage, scope)?;
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
            analysis_scope: scope.to_string(),
            signal_scope: scope.to_string(),
            regime_basis_scope: scope.to_string(),
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

    pub fn refresh_backtests_for_standard_scopes(&self) -> Result<Vec<BacktestRunSummary>> {
        [ReportScope::Global, ReportScope::Cn, ReportScope::Hk]
            .into_iter()
            .map(|scope| self.run_backtest(1_000_000.0, 3, 0.001, 0.0005, scope))
            .collect()
    }

    pub fn dashboard_snapshot(
        &self,
        report_date: Option<NaiveDate>,
    ) -> Result<Option<DashboardSnapshot>> {
        self.dashboard_snapshot_with_scope(report_date, ReportScope::Global)
    }

    pub fn dashboard_snapshot_with_scope(
        &self,
        report_date: Option<NaiveDate>,
        scope: ReportScope,
    ) -> Result<Option<DashboardSnapshot>> {
        let total_started_at = Instant::now();
        let available_dates_started_at = Instant::now();
        let available_dates = self.dashboard_available_dates_for_scope(scope)?;
        let available_dates_ms = elapsed_ms(available_dates_started_at);
        let (snapshot, mut metrics) =
            self.dashboard_snapshot_from_available_dates(report_date, &available_dates, scope)?;
        metrics.available_dates_ms = available_dates_ms;
        metrics.total_ms = elapsed_ms(total_started_at);
        let pipeline_dates = self.pipeline_date_diagnostics_for_scope(scope, &available_dates)?;
        let data_health = self.check_data_health()?;
        let scoped_symbols = self
            .latest_gate_instruments_for_scope(scope)?
            .into_iter()
            .map(|instrument| instrument.symbol)
            .collect::<std::collections::BTreeSet<_>>();
        Ok(snapshot.map(|mut snapshot| {
            snapshot.load_metrics = Some(metrics);
            snapshot.trust_summary = Some(build_trust_summary(
                &scoped_symbols,
                &snapshot,
                &pipeline_dates,
                &data_health,
            ));
            snapshot
        }))
    }

    pub fn dashboard_bundle(
        &self,
        report_date: Option<NaiveDate>,
        recent_report_limit: usize,
    ) -> Result<DashboardLoadBundle> {
        self.dashboard_bundle_with_scope(report_date, ReportScope::Global, recent_report_limit)
    }

    pub fn dashboard_bundle_with_scope(
        &self,
        report_date: Option<NaiveDate>,
        scope: ReportScope,
        recent_report_limit: usize,
    ) -> Result<DashboardLoadBundle> {
        let total_started_at = Instant::now();
        let available_dates_started_at = Instant::now();
        let status = self.status()?;
        let available_dates = self.dashboard_available_dates_for_scope(scope)?;
        let available_dates_ms = elapsed_ms(available_dates_started_at);
        let (snapshot, mut metrics) =
            self.dashboard_snapshot_from_available_dates(report_date, &available_dates, scope)?;
        let recent_reports = self.recent_reports(recent_report_limit)?;
        let pipeline_dates = self.pipeline_date_diagnostics_for_scope(scope, &available_dates)?;
        let data_health = self.check_data_health()?;
        let scoped_symbols = self
            .latest_gate_instruments_for_scope(scope)?
            .into_iter()
            .map(|instrument| instrument.symbol)
            .collect::<std::collections::BTreeSet<_>>();
        metrics.available_dates_ms = available_dates_ms;
        metrics.total_ms = elapsed_ms(total_started_at);
        let snapshot = snapshot.map(|mut snapshot| {
            snapshot.load_metrics = Some(metrics);
            snapshot.trust_summary = Some(build_trust_summary(
                &scoped_symbols,
                &snapshot,
                &pipeline_dates,
                &data_health,
            ));
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
        let available_dates = self.dashboard_available_dates_for_scope(ReportScope::Global)?;
        self.pipeline_date_diagnostics_for_scope(ReportScope::Global, &available_dates)
    }

    pub fn pipeline_date_diagnostics_with_scope(
        &self,
        scope: ReportScope,
    ) -> Result<PipelineDateDiagnostics> {
        let available_dates = self.dashboard_available_dates_for_scope(scope)?;
        self.pipeline_date_diagnostics_for_scope(scope, &available_dates)
    }

    fn pipeline_date_diagnostics_for_scope(
        &self,
        scope: ReportScope,
        available_dates: &[NaiveDate],
    ) -> Result<PipelineDateDiagnostics> {
        let scoped_symbols = self
            .latest_gate_instruments_for_scope(scope)?
            .into_iter()
            .map(|instrument| instrument.symbol)
            .collect::<Vec<_>>();
        let freshest_market_date =
            market_store::fetch_latest_table_date(&self.storage, "daily_bar")?;
        let dashboard_latest_date = available_dates.first().copied();
        let expected_symbol_count = scoped_symbols.len();
        let stage_rows = [
            ("daily_bar", freshest_market_date),
            (
                "indicator_snapshot",
                market_store::fetch_latest_table_date(&self.storage, "indicator_snapshot")?,
            ),
            (
                "market_regime",
                market_store::fetch_latest_market_regime_date_for_scope(&self.storage, scope)?,
            ),
            (
                "environment_snapshot",
                market_store::fetch_latest_environment_date_for_scope(&self.storage, scope)?,
            ),
            (
                "rotation_rank",
                market_store::fetch_latest_table_date(&self.storage, "rotation_rank")?,
            ),
            (
                "strategy_preference",
                market_store::fetch_latest_strategy_preference_date_for_scope(
                    &self.storage,
                    scope,
                )?,
            ),
            (
                "signal_snapshot",
                market_store::fetch_latest_signal_snapshot_date_for_scope(&self.storage, scope)?,
            ),
            ("dashboard_available", dashboard_latest_date),
        ];
        let stages = stage_rows
            .into_iter()
            .map(|(stage, latest_date)| {
                let (latest_entities, expected_entities) = match (stage, latest_date) {
                    ("daily_bar", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_in_symbols(
                            &self.storage,
                            "daily_bar",
                            "symbol",
                            &scoped_symbols,
                            date,
                        )?),
                        Some(expected_symbol_count),
                    ),
                    ("indicator_snapshot", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_in_symbols(
                            &self.storage,
                            "indicator_snapshot",
                            "symbol",
                            &scoped_symbols,
                            date,
                        )?),
                        Some(expected_symbol_count),
                    ),
                    ("rotation_rank", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_in_symbols(
                            &self.storage,
                            "rotation_rank",
                            "symbol",
                            &scoped_symbols,
                            date,
                        )?),
                        Some(expected_symbol_count),
                    ),
                    ("strategy_preference", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_with_filter(
                            &self.storage,
                            "strategy_preference",
                            "symbol",
                            "analysis_scope",
                            scope_label(scope),
                            date,
                        )?),
                        Some(expected_symbol_count),
                    ),
                    ("signal_snapshot", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_with_filter(
                            &self.storage,
                            "signal_snapshot",
                            "symbol",
                            "analysis_scope",
                            scope_label(scope),
                            date,
                        )?),
                        Some(expected_symbol_count),
                    ),
                    ("market_regime", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_with_filter(
                            &self.storage,
                            "market_regime",
                            "market",
                            "market",
                            scope_label(scope),
                            date,
                        )?),
                        Some(1),
                    ),
                    ("environment_snapshot", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_with_filter(
                            &self.storage,
                            "environment_snapshot",
                            "scope",
                            "scope",
                            scope_label(scope),
                            date,
                        )?),
                        Some(1),
                    ),
                    _ => (None, None),
                };

                Ok(PipelineStageDateStatus {
                    stage: stage.to_string(),
                    latest_date: latest_date.map(|date| date.to_string()),
                    lag_days: match (freshest_market_date, latest_date) {
                        (Some(reference), Some(stage_date)) => Some((reference - stage_date).num_days()),
                        _ => None,
                    },
                    is_latest: matches!((freshest_market_date, latest_date), (Some(reference), Some(stage_date)) if reference == stage_date),
                    latest_entities,
                    expected_entities,
                    is_complete: match (latest_entities, expected_entities) {
                        (Some(actual), Some(expected)) if expected > 0 => Some(actual >= expected),
                        _ => None,
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(PipelineDateDiagnostics {
            freshest_market_date: freshest_market_date.map(|date| date.to_string()),
            dashboard_latest_date: dashboard_latest_date.map(|date| date.to_string()),
            stages,
        })
    }

    fn dashboard_snapshot_from_available_dates(
        &self,
        report_date: Option<NaiveDate>,
        available_dates: &[NaiveDate],
        scope: ReportScope,
    ) -> Result<(Option<DashboardSnapshot>, DashboardLoadMetrics)> {
        let zero_metrics = DashboardLoadMetrics {
            available_dates_ms: 0,
            regime_ms: 0,
            environment_ms: 0,
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
        let regime = market_store::fetch_latest_market_regime_on_or_before(
            &self.storage,
            report_date,
            scope,
        )?
        .context("no market regime available for dashboard snapshot")?;
        let regime_ms = elapsed_ms(regime_started_at);
        let environment_started_at = Instant::now();
        let environment =
            market_store::fetch_latest_environment_on_or_before(&self.storage, report_date, scope)?;
        let environment_ms = elapsed_ms(environment_started_at);
        let rotations_started_at = Instant::now();
        let scoped_instruments = self.instruments_for_scope(scope)?;
        let scoped_symbols = scoped_instruments
            .iter()
            .map(|instrument| instrument.symbol.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let rotations = market_store::fetch_rotation_ranks_for_date(&self.storage, report_date)?
            .into_iter()
            .filter(|row| scoped_symbols.contains(&row.symbol))
            .collect::<Vec<_>>();
        let rotations_ms = elapsed_ms(rotations_started_at);
        let signals_started_at = Instant::now();
        let signals = market_store::fetch_signal_snapshots_for_date_with_scope(
            &self.storage,
            report_date,
            scope,
        )?
        .into_iter()
        .filter(|row| scoped_symbols.contains(&row.symbol))
        .collect::<Vec<_>>();
        let signals_ms = elapsed_ms(signals_started_at);
        let backtest_started_at = Instant::now();
        let latest_backtest =
            market_store::fetch_latest_backtest_run_for_scope(&self.storage, scope)?;
        let backtest_ms = elapsed_ms(backtest_started_at);
        let assembly_started_at = Instant::now();
        let mut snapshot = build_dashboard_snapshot_for_date(
            &regime,
            &rotations,
            &signals,
            latest_backtest,
            report_date,
            latest_available_date,
            scope_label(scope),
        );
        snapshot.environment = environment;
        let assembly_ms = elapsed_ms(assembly_started_at);
        let breadth_started_at = Instant::now();
        snapshot.watchlist_breadth = self.compute_watchlist_breadth_snapshot(report_date, scope)?;
        let breadth_ms = elapsed_ms(breadth_started_at);
        Ok((
            Some(snapshot),
            DashboardLoadMetrics {
                available_dates_ms: 0,
                regime_ms,
                environment_ms,
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
        self.dashboard_available_dates_with_scope(ReportScope::Global)
    }

    pub fn dashboard_available_dates_with_scope(&self, scope: ReportScope) -> Result<Vec<String>> {
        Ok(self
            .dashboard_available_dates_for_scope(scope)?
            .into_iter()
            .map(|date| date.to_string())
            .collect())
    }

    fn dashboard_available_dates_for_scope(&self, scope: ReportScope) -> Result<Vec<NaiveDate>> {
        let available_dates = market_store::fetch_dashboard_available_dates(&self.storage)?;
        let scoped_symbols = self
            .latest_gate_instruments_for_scope(scope)?
            .into_iter()
            .map(|instrument| instrument.symbol)
            .collect::<Vec<_>>();
        let expected_count = scoped_symbols.len();
        if expected_count == 0 {
            return Ok(Vec::new());
        }
        let mut scoped_dates = Vec::new();
        for date in available_dates {
            let signal_count = market_store::fetch_distinct_entity_count_for_date_with_filter(
                &self.storage,
                "signal_snapshot",
                "symbol",
                "analysis_scope",
                scope_label(scope),
                date,
            )?;
            let rotation_count = market_store::fetch_distinct_entity_count_for_date_in_symbols(
                &self.storage,
                "rotation_rank",
                "symbol",
                &scoped_symbols,
                date,
            )?;
            let has_regime =
                market_store::fetch_latest_market_regime_on_or_before(&self.storage, date, scope)?
                    .is_some();
            let has_environment =
                market_store::fetch_latest_environment_on_or_before(&self.storage, date, scope)?
                    .is_some();
            if signal_count >= expected_count
                && rotation_count >= expected_count
                && has_regime
                && has_environment
            {
                scoped_dates.push(date);
            }
        }
        Ok(scoped_dates)
    }

    fn instruments_for_scope(&self, scope: ReportScope) -> Result<Vec<Instrument>> {
        Ok(load_universe(&self.storage.universe_abspath()?)?
            .into_iter()
            .filter(|instrument| instrument.enabled && instrument_in_scope(instrument, scope))
            .collect())
    }

    fn compute_watchlist_breadth_snapshot(
        &self,
        report_date: NaiveDate,
        scope: ReportScope,
    ) -> Result<Option<WatchlistBreadthSnapshot>> {
        let history_start = report_date - Duration::days(180);
        let window = self.build_tracked_universe_window(history_start, report_date)?;
        if window.relevant_dates.is_empty() {
            return Ok(None);
        }

        let methodology_note = "Eligible tracked instruments must be enabled INDEX/ETF universe members with both close and MA30 available on the selected date. Proxy only; not full-market stock breadth.".to_string();

        let markets = match scope {
            ReportScope::Global => vec![
                build_market_watchlist_breadth_snapshot(
                    ReportScope::Cn,
                    &window.cn_series,
                    report_date,
                    &window.relevant_dates,
                ),
                build_market_watchlist_breadth_snapshot(
                    ReportScope::Hk,
                    &window.hk_series,
                    report_date,
                    &window.relevant_dates,
                ),
            ],
            ReportScope::Cn => vec![build_market_watchlist_breadth_snapshot(
                ReportScope::Cn,
                &window.cn_series,
                report_date,
                &window.relevant_dates,
            )],
            ReportScope::Hk => vec![build_market_watchlist_breadth_snapshot(
                ReportScope::Hk,
                &window.hk_series,
                report_date,
                &window.relevant_dates,
            )],
        };

        Ok(Some(WatchlistBreadthSnapshot {
            report_date: report_date.to_string(),
            markets,
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
                latest_gate_required: instrument.latest_gate_required,
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
        let freshest_market_date = summaries.iter().filter_map(|row| row.last_date).max();
        let symbols_on_freshest_market_date = freshest_market_date
            .map(|date| {
                summaries
                    .iter()
                    .filter(|row| row.last_date == Some(date))
                    .count()
            })
            .unwrap_or(0);
        let symbols_missing_freshest_market_date = summaries
            .len()
            .saturating_sub(symbols_on_freshest_market_date);
        let freshest_market_date_complete =
            freshest_market_date.is_some() && symbols_missing_freshest_market_date == 0;
        let latest_gate_checked_symbols = summaries
            .iter()
            .filter(|row| row.latest_gate_required)
            .count();
        let latest_gate_symbols_on_freshest_market_date = freshest_market_date
            .map(|date| {
                summaries
                    .iter()
                    .filter(|row| row.latest_gate_required && row.last_date == Some(date))
                    .count()
            })
            .unwrap_or(0);
        let latest_gate_symbols_missing_freshest_market_date =
            latest_gate_checked_symbols.saturating_sub(latest_gate_symbols_on_freshest_market_date);
        let latest_gate_freshest_market_date_complete =
            freshest_market_date.is_some() && latest_gate_symbols_missing_freshest_market_date == 0;

        Ok(DataHealthSummary {
            generated_at: Utc::now().to_rfc3339(),
            canonical_adjustment: "forward-adjusted daily bars (Eastmoney fqt=1, Tencent qfq)"
                .to_string(),
            freshest_market_date,
            symbols_on_freshest_market_date,
            symbols_missing_freshest_market_date,
            freshest_market_date_complete,
            latest_gate_checked_symbols,
            latest_gate_symbols_on_freshest_market_date,
            latest_gate_symbols_missing_freshest_market_date,
            latest_gate_freshest_market_date_complete,
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
        self.export_report_with_scope(report_date, ReportScope::Global)
    }

    pub fn export_report_with_scope(
        &self,
        report_date: Option<NaiveDate>,
        scope: ReportScope,
    ) -> Result<ReportSummary> {
        let snapshot = self
            .dashboard_snapshot_with_scope(report_date, scope)?
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
        let report_slug = match scope {
            ReportScope::Global => format!("daily-report-{}", snapshot.report_date),
            ReportScope::Cn => format!("daily-report-cn-{}", snapshot.report_date),
            ReportScope::Hk => format!("daily-report-hk-{}", snapshot.report_date),
        };
        let report_type = match scope {
            ReportScope::Global => "DAILY_REPORT",
            ReportScope::Cn => "DAILY_REPORT_CN",
            ReportScope::Hk => "DAILY_REPORT_HK",
        };
        let output_path = report_dir.join(format!("{}.md", report_slug));
        fs::write(&output_path, markdown)
            .with_context(|| format!("failed to write report file: {}", output_path.display()))?;
        market_store::insert_report_snapshot(
            &self.storage,
            &snapshot.report_date,
            report_type,
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
        let report_date = summary
            .symbols
            .iter()
            .filter_map(|row| row.last_date)
            .max()
            .unwrap_or_else(|| Utc::now().date_naive())
            .to_string();
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
            volume_by_date: points
                .iter()
                .map(|(date, _, _)| (*date, 1000.0))
                .collect::<BTreeMap<_, _>>(),
            turnover_present_by_date: points
                .iter()
                .map(|(date, _, _)| (*date, true))
                .collect::<BTreeMap<_, _>>(),
            ma30_by_date: points
                .iter()
                .map(|(date, _, ma30)| (*date, *ma30))
                .collect::<BTreeMap<_, _>>(),
            vol_ma20_by_date: points
                .iter()
                .map(|(date, _, _)| (*date, 900.0))
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
            ReportScope::Cn,
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

    #[test]
    fn participation_metrics_compute_liquidity_proxy_fields() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 2).unwrap();
        let strong = TrackedInstrumentSeries {
            close_by_date: BTreeMap::from([(date, 110.0)]),
            volume_by_date: BTreeMap::from([(date, 1500.0)]),
            turnover_present_by_date: BTreeMap::from([(date, true)]),
            ma30_by_date: BTreeMap::from([(date, 100.0)]),
            vol_ma20_by_date: BTreeMap::from([(date, 1000.0)]),
        };
        let weak = TrackedInstrumentSeries {
            close_by_date: BTreeMap::from([(date, 90.0)]),
            volume_by_date: BTreeMap::from([(date, 800.0)]),
            turnover_present_by_date: BTreeMap::from([(date, false)]),
            ma30_by_date: BTreeMap::from([(date, 100.0)]),
            vol_ma20_by_date: BTreeMap::from([(date, 1000.0)]),
        };

        let metrics = compute_participation_metrics(&[strong, weak], date, &[date]);

        assert_eq!(metrics.current.eligible_count, 2);
        assert_eq!(metrics.current.above_count, 1);
        assert_eq!(metrics.current.volume_expansion_pct, Some(50.0));
        assert_eq!(metrics.current.turnover_coverage_pct, Some(50.0));
        assert!((metrics.current.liquidity_proxy_score - 50.0).abs() < 1e-9);
        assert_eq!(metrics.breadth_state, "neutral");
    }
}

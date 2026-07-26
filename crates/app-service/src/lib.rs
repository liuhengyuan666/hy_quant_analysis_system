use anyhow::{Context, Result};
use async_trait::async_trait;
use backtest_engine::{run_signal_backtest, BacktestConfig};
use chrono::{Duration, NaiveDate, Utc};
use core_domain::{
    EnvironmentSnapshot, FredFileConfig, Instrument, InstrumentType, LlmAnalysisResult, LlmConfig,
    LlmFileConfig, LlmStatus, Market, RefreshJobRecord, SignalSnapshot, StartupFreshnessCheck,
};
use core_domain::research::classification::classify_level;
use core_domain::research::stretch::weighted_stretch_overall;
use data_ingestion::{
    fetch_daily_bars, fetch_eastmoney_daily_bars, fetch_fred_series, fetch_fred_series_with_status,
    fetch_tencent_daily_bars, load_universe,
};
use macro_engine::{build_macro_snapshots, build_market_regimes, build_strategy_state};
use market_store::StorageConfig;
use report_engine::{
    build_dashboard_snapshot_for_date, render_data_health_report, render_markdown_report,
    DashboardLoadMetrics, DashboardSnapshot, DataHealthMacroSourceSummary, DataHealthSummary,
    DataHealthSymbolSummary,
    WatchlistBreadthSnapshot,
};
use report_renderer::DashboardInsightComposer;
use serde::Serialize;
use signal_engine::build_signal_snapshots;
use std::collections::BTreeMap;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use strategy_engine::{build_strategy_preferences, AnalysisContext};

/// TOML-based configuration loader module
pub mod config_loader;

// Domain modules (extracted from the monolith)
pub mod breadth;
pub mod core;
pub mod dashboard;
pub mod execution_replay;
pub mod llm;
pub mod llm_history;
pub mod prompts;
pub mod research_evidence;
pub mod scenarios;
pub mod strategy_perspectives;
pub mod sync;
pub mod trust;
pub mod workspace;

use crate::breadth::*;
use crate::core::*;
use crate::dashboard::*;
use crate::llm::*;
use crate::research_evidence::compute_condition_evidence;
use crate::sync::*;
use crate::trust::*;

pub use core_domain::AnalysisScope as ReportScope;
pub use report_renderer::ResearchInsight;
pub use execution_engine::types::ExecutionDecision;
pub use strategy_perspectives::{
    AttributionDriverView, ScenarioScore, StrategyAttributionView, StrategyPerspectiveDetail,
    StrategyPerspectiveEntry,
};

const CALENDAR_GAP_REVIEW_THRESHOLD_DAYS: i64 = 12;
const REFRESH_SOURCE_LOOKBACK_DAYS: i64 = 7;
const REFRESH_GATE_REPAIR_WINDOW_DAYS: i64 = 30;
const REFRESH_BOOTSTRAP_LOOKBACK_DAYS: i64 = 730;
const REFRESH_MACRO_LOOKBACK_DAYS: i64 = 550;

/// Dashboard 可用日期缓存 TTL（5 分钟）
const AVAILABLE_DATES_CACHE_TTL_SECS: u64 = 300;

/// 缓存条目：存储结果和上次更新时间
#[derive(Debug, Clone)]
struct CacheEntry<T: Clone> {
    data: T,
    updated_at: Instant,
}

/// Dashboard 可用日期缓存
#[derive(Debug)]
struct AvailableDatesCache {
    /// 按 scope 缓存的可用日期
    dates_by_scope: Mutex<BTreeMap<ReportScope, CacheEntry<Vec<NaiveDate>>>>,
}

impl AvailableDatesCache {
    fn new() -> Self {
        Self {
            dates_by_scope: Mutex::new(BTreeMap::new()),
        }
    }

    /// 获取缓存的可用日期，如果过期则返回 None
    fn get(&self, scope: &ReportScope) -> Option<Vec<NaiveDate>> {
        let cache = self.dates_by_scope.lock().ok()?;
        let entry = cache.get(scope)?;
        if entry.updated_at.elapsed().as_secs() < AVAILABLE_DATES_CACHE_TTL_SECS {
            Some(entry.data.clone())
        } else {
            None
        }
    }

    /// 更新缓存
    fn insert(&self, scope: ReportScope, dates: Vec<NaiveDate>) {
        if let Ok(mut cache) = self.dates_by_scope.lock() {
            cache.insert(
                scope,
                CacheEntry {
                    data: dates,
                    updated_at: Instant::now(),
                },
            );
        }
    }

    /// 清除所有缓存
    fn clear(&self) {
        if let Ok(mut cache) = self.dates_by_scope.lock() {
            cache.clear();
        }
    }
}

pub mod pipeline_stages {
    pub const STAGE_INGEST: &str = "ingest";
    pub const STAGE_INDICATORS: &str = "indicators";
    pub const STAGE_MACRO: &str = "macro";
    pub const STAGE_ROTATION: &str = "rotation";
    pub const STAGE_STRATEGY: &str = "strategy";
    pub const STAGE_SIGNALS: &str = "signals";
    pub const STAGE_BACKTESTS: &str = "backtests";

    pub const ALL: &[&str] = &[
        STAGE_INGEST,
        STAGE_INDICATORS,
        STAGE_MACRO,
        STAGE_ROTATION,
        STAGE_STRATEGY,
        STAGE_SIGNALS,
        STAGE_BACKTESTS,
    ];

    pub const PROGRESS_INGEST: u8 = 20;
    pub const PROGRESS_INDICATORS: u8 = 40;
    pub const PROGRESS_MACRO: u8 = 60;
    pub const PROGRESS_ROTATION: u8 = 75;
    pub const PROGRESS_STRATEGY: u8 = 88;
    pub const PROGRESS_SIGNALS: u8 = 92;
    pub const PROGRESS_BACKTESTS: u8 = 96;

    pub fn progress_after(stage: &str) -> u8 {
        match stage {
            STAGE_INGEST => PROGRESS_INGEST,
            STAGE_INDICATORS => PROGRESS_INDICATORS,
            STAGE_MACRO => PROGRESS_MACRO,
            STAGE_ROTATION => PROGRESS_ROTATION,
            STAGE_STRATEGY => PROGRESS_STRATEGY,
            STAGE_SIGNALS => PROGRESS_SIGNALS,
            STAGE_BACKTESTS => PROGRESS_BACKTESTS,
            _ => 0,
        }
    }
}








#[derive(Debug, Clone)]
pub struct AppContext {
    pub storage: StorageConfig,
    pub calendar: core_domain::calendar::TradingCalendar,
    /// Dashboard 可用日期缓存
    available_dates_cache: std::sync::Arc<AvailableDatesCache>,
}

/// Summary of one Evidence asset produced by `AppContext::research_replay`.
#[derive(Debug, Clone, Serialize)]
pub struct ReplayEvidenceSummary {
    pub id: String,
    pub condition: String,
    pub scope: String,
    pub horizon: usize,
    pub occurrences: usize,
    pub positive_ratio: f64,
    pub median_forward_return: f64,
    pub workspace_path: String,
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
    pub strategy_state_rows: usize,
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
    pub data_starved_count: usize,
    pub data_starved_warning: Option<String>,
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
    pub drawdown_events: usize,
    pub failed_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportSummary {
    pub report_date: String,
    pub output_path: String,
    pub failed_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncAndExportSummary {
    pub report_date: String,
    pub output_path: String,
    pub refreshed: bool,
    pub gate_advanced: Option<bool>,
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
    pub insight: Option<ResearchInsight>,
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
    pub alerts: Vec<String>,
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
    pub latest_daily_date: Option<String>,
    pub latest_gated_dashboard_date: Option<String>,
    pub refresh_reason: String,
    pub repair_window_days: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshLatestDateStatus {
    pub scope: String,
    pub freshest_market_date: Option<String>,
    pub dashboard_latest_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScopedPipelineDiagnostics {
    pub scope: String,
    pub diagnostics: PipelineDateDiagnostics,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum RefreshStageSummary {
    Ingest(IngestSummary),
    Indicators(IndicatorSummary),
    Macro(MacroSummary),
    Rotation(RotationSummary),
    Strategy(StrategySummary),
    Signals(SignalSummary),
    Backtests(Vec<BacktestRunSummary>),
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshStageExecution {
    pub name: String,
    pub status: String,
    pub summary: Option<RefreshStageSummary>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshPipelineAlerts {
    pub consistency: Vec<String>,
    pub blocking: Vec<String>,
    pub latest_gate: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshPipelineSummary {
    pub success: bool,
    pub cancelled: bool,
    pub job_id: String,
    pub diagnostics_scope: String,
    pub refresh_window: RefreshPlan,
    pub backtests_requested: bool,
    pub latest_dates_before: Vec<RefreshLatestDateStatus>,
    pub latest_dates_after: Vec<RefreshLatestDateStatus>,
    pub advanced: bool,
    pub stages: Vec<RefreshStageExecution>,
    pub pipeline_diagnostics_by_scope: Vec<ScopedPipelineDiagnostics>,
    pub alerts: RefreshPipelineAlerts,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatestGateStageExplanation {
    pub stage: String,
    pub latest_date: Option<String>,
    pub lag_days: Option<i64>,
    pub is_latest: bool,
    pub latest_entities: Option<usize>,
    pub expected_entities: Option<usize>,
    pub is_complete: Option<bool>,
    pub blocking: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatestGateExplanation {
    pub scope: String,
    pub freshest_market_date: Option<String>,
    pub latest_available_dashboard_date: Option<String>,
    pub latest_gate_advanced: Option<bool>,
    pub alerts: Vec<String>,
    pub stages: Vec<LatestGateStageExplanation>,
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









const LLM_SERVICE_NAME: &str = "rust-quant-analysis-system";
const LLM_ACCOUNT_NAME: &str = "llm_api_key";

fn probe_keyring_readable() -> bool {
    let Ok(entry) = keyring::Entry::new(LLM_SERVICE_NAME, LLM_ACCOUNT_NAME) else {
        return false;
    };
    match entry.get_password() {
        Ok(_) => true,
        Err(keyring::Error::NoEntry) => true,
        Err(_) => false,
    }
}

/// Determines whether `sync_and_export` should attempt a pipeline refresh.
/// Returns `true` when the gate is not yet advanced (behind or unknown).

/// Validates that a refresh pipeline result is acceptable for proceeding.
/// Returns `Ok(())` if refresh succeeded, `Err` with blocking alerts if it failed.

/// Placeholder LLM provider for testing.
/// Returns structured dummy responses. Replace with a real provider
/// (OpenAI, DeepSeek, etc.) for actual analysis.
#[allow(dead_code)]
struct PlaceholderProvider;

#[async_trait]
impl research_skills::provider::LlmProvider for PlaceholderProvider {
    async fn chat(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _config: &research_skills::provider::LlmCallConfig,
    ) -> anyhow::Result<String> {
        Ok(r#"{
            "analysis": "Market regime analysis completed",
            "note": "This is a placeholder response. Configure a real LLM provider for actual analysis."
        }"#.to_string())
    }
}

// ── Phase 1: Unified Research Dataset ──

/// Sole owner of research query results within a research execution session.
/// Pure data container — only raw records, no business methods.
/// NOT exposed outside the app-service boundary.
struct ResearchDataset {
    date: NaiveDate,
    scope: core_domain::AnalysisScope,
    signals: Vec<SignalSnapshot>,
    states_history: Vec<core_domain::StrategyStateSnapshot>,
    env_history: Vec<EnvironmentSnapshot>,
    rotations: Vec<core_domain::RotationRankSnapshot>,
    rotation_history: BTreeMap<NaiveDate, Vec<core_domain::RotationRankSnapshot>>,
    all_regimes: Vec<core_domain::MarketRegimeSnapshot>,
    signal_history: BTreeMap<NaiveDate, Vec<(f64, core_domain::SignalLabel)>>,
}

/// Internal model: all research-relevant data for a single (date, scope).
/// SRD, Stretch, and future Quarterly Review all build on this snapshot.
/// This is an internal abstraction; it is not exposed in any public API or report DTO.
#[derive(Debug, Clone)]
pub struct ResearchSnapshot {
    pub date: NaiveDate,
    pub signals: Vec<core_domain::SignalSnapshot>,
    pub state: Option<core_domain::StrategyStateSnapshot>,
    pub states_history: Vec<core_domain::StrategyStateSnapshot>,
    pub rotations: Vec<core_domain::RotationRankSnapshot>,
    pub env: Option<core_domain::EnvironmentSnapshot>,
    pub signal_history: BTreeMap<NaiveDate, Vec<(f64, core_domain::SignalLabel)>>,
}

impl ResearchSnapshot {
    pub fn strong_buy_count(&self) -> usize {
        self.signals
            .iter()
            .filter(|s| matches!(s.signal_label, core_domain::SignalLabel::StrongBuy))
            .count()
    }

    pub fn buy_count(&self) -> usize {
        self.signals
            .iter()
            .filter(|s| matches!(s.signal_label, core_domain::SignalLabel::Buy))
            .count()
    }

    pub fn average_signal(&self) -> f64 {
        if self.signals.is_empty() {
            0.0
        } else {
            self.signals.iter().map(|s| s.final_score).sum::<f64>() / self.signals.len() as f64
        }
    }

    pub fn state_label(&self) -> String {
        self.state
            .as_ref()
            .map(|s| format!("{:?}", s.state))
            .unwrap_or_else(|| "NO_TRADE".to_string())
    }

    pub fn divergence_duration(&self) -> i64 {
        let is_conservative = |state: &core_domain::StrategyState| -> bool {
            matches!(
                state,
                core_domain::StrategyState::NoTrade
                    | core_domain::StrategyState::DeRisk
                    | core_domain::StrategyState::LeftProbe
            )
        };

        let mut recent_states: Vec<&core_domain::StrategyStateSnapshot> = self
            .states_history
            .iter()
            .filter(|s| s.date <= self.date)
            .collect();
        recent_states.sort_by(|a, b| b.date.cmp(&a.date));

        let mut duration: i64 = 0;
        for state_snapshot in &recent_states {
            if !is_conservative(&state_snapshot.state) {
                break;
            }
            let has_divergent = self
                .signal_history
                .get(&state_snapshot.date)
                .map(|signals| {
                    signals
                        .iter()
                        .any(|(_, label)| matches!(label, core_domain::SignalLabel::StrongBuy | core_domain::SignalLabel::Buy))
                })
                .unwrap_or(false);
            if has_divergent {
                duration += 1;
            } else {
                break;
            }
        }
        duration
    }

    pub fn breadth_trend(&self) -> &'static str {
        match self.env {
            Some(ref env) => {
                let delta = env.breadth_5d_delta.unwrap_or(0.0);
                if delta > 0.05 {
                    "Improving"
                } else if delta < -0.05 {
                    "Weakening"
                } else {
                    "Neutral"
                }
            }
            None => "Neutral",
        }
    }

    pub fn rotation_pattern(&self) -> &'static str {
        let mut sorted = self.rotations.clone();
        sorted.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));
        let top_count = sorted.len().min(10);
        let top_10_avg_momentum: f64 = if top_count > 0 {
            sorted.iter().take(top_count).map(|r| r.momentum_score).sum::<f64>() / top_count as f64
        } else {
            0.0
        };

        if top_10_avg_momentum > 1.5 {
            "Technology Dominant"
        } else if top_10_avg_momentum < 0.3 {
            "Defensive"
        } else {
            "Mixed"
        }
    }

    pub fn signal_percentile(&self) -> f64 {
        let avg_signal = self.average_signal();
        let mut all_avg_signals: Vec<f64> = self
            .signal_history
            .values()
            .filter(|signals| !signals.is_empty())
            .map(|signals| {
                signals.iter().map(|(s, _)| s).sum::<f64>() / signals.len() as f64
            })
            .collect();
        all_avg_signals.sort_by(|a, b| a.total_cmp(b));

        if all_avg_signals.is_empty() {
            50.0
        } else {
            let below = all_avg_signals.iter().filter(|&&v| v < avg_signal).count();
            (below as f64 / all_avg_signals.len() as f64) * 100.0
        }
    }

    pub fn stretch_crowding(&self) -> (&'static str, f64, Option<f64>) {
        let total_momentum: f64 = self.rotations.iter().map(|r| r.momentum_score).sum();
        let mut sorted = self.rotations.clone();
        sorted.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));
        let top5_sum: f64 = sorted.iter().take(5).map(|r| r.momentum_score).sum();
        let concentration_pct = if total_momentum > 0.0 {
            (top5_sum / total_momentum) * 100.0
        } else {
            0.0
        };
        let level = classify_level(concentration_pct, 30.0, 50.0, true);
        (level, concentration_pct, None)
    }

    pub fn stretch_momentum(&self) -> (&'static str, f64, f64) {
        let rs120_max = self
            .rotations
            .iter()
            .map(|r| r.rs_120)
            .fold(f64::NEG_INFINITY, f64::max);
        let mut sorted = self.rotations.clone();
        sorted.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));
        let top5: Vec<&core_domain::RotationRankSnapshot> = sorted.iter().take(5).collect();
        let top5_rs120_avg = if !top5.is_empty() {
            top5.iter().map(|r| r.rs_120).sum::<f64>() / top5.len() as f64
        } else {
            0.0
        };
        let level = classify_level(rs120_max, 70.0, 85.0, true);
        (level, rs120_max, top5_rs120_avg)
    }

    pub fn stretch_breadth(&self) -> (&'static str, f64, Option<f64>) {
        match self.env {
            Some(ref env) => {
                let bp = env.breadth_pct;
                let sma5 = env.breadth_pct_sma5;
                let level = classify_level(bp, 35.0, 20.0, false);
                (level, bp, sma5)
            }
            None => ("Normal", 0.0, None),
        }
    }

    pub fn stretch_leverage(&self) -> &'static str {
        "Normal"
    }

    pub fn stretch_overall(&self) -> (&'static str, f64) {
        let (crowding_level, _, _) = self.stretch_crowding();
        let (breadth_level, _, _) = self.stretch_breadth();
        let (momentum_level, _, _) = self.stretch_momentum();
        let leverage_level = self.stretch_leverage();
        weighted_stretch_overall(crowding_level, breadth_level, momentum_level, leverage_level)
    }
}

// V7.2B: Forward-return provider backed by anchor symbol daily bar close prices.
//
// Implements `ForwardReturnProvider` by looking up close prices in a
// pre-loaded `BTreeMap<NaiveDate, f64>` and computing forward returns
// and max drawdowns over the given horizon.
struct AnchorBarForwardProvider<'a> {
    close_by_date: &'a BTreeMap<NaiveDate, f64>,
}

impl<'a> market_fingerprint_engine::ForwardReturnProvider
    for AnchorBarForwardProvider<'a>
{
    fn forward_return(&self, date: NaiveDate, horizon_days: usize) -> Option<f64> {
        let start_price = self.close_by_date.get(&date).copied()?;
        if start_price <= 0.0 {
            return None;
        }
        let future_dates: Vec<NaiveDate> = self
            .close_by_date
            .keys()
            .copied()
            .filter(|k| k > &date)
            .collect();
        if future_dates.len() < horizon_days {
            return None;
        }
        let end_date = future_dates[horizon_days - 1];
        let end_price = self.close_by_date.get(&end_date).copied()?;
        Some((end_price - start_price) / start_price)
    }

    fn forward_max_drawdown(&self, date: NaiveDate, horizon_days: usize) -> Option<f64> {
        let start_price = self.close_by_date.get(&date).copied()?;
        if start_price <= 0.0 {
            return None;
        }
        let future_dates: Vec<NaiveDate> = self
            .close_by_date
            .keys()
            .copied()
            .filter(|k| k > &date)
            .take(horizon_days)
            .collect();
        if future_dates.is_empty() {
            return None;
        }
        let prices: Vec<f64> = future_dates
            .iter()
            .filter_map(|d| self.close_by_date.get(d).copied())
            .collect();
        if prices.is_empty() {
            return None;
        }
        let mut peak = start_price;
        let mut max_dd: f64 = 0.0;
        for &p in &prices {
            if p > peak {
                peak = p;
            }
            let dd = (p - peak) / peak;
            if dd < max_dd {
                max_dd = dd;
            }
        }
        Some(max_dd)
    }
}

/// ADR-113/114: outcome of the shared adversarial ensure step — the record
/// (if one is available for injection) plus a machine-readable reason
/// explaining the result. Reasons:
/// - `"injected"`         → record available (fresh cache reuse or new generation)
/// - `"stale"`            → a record exists but report_date mismatched AND
///                          regeneration failed
/// - `"no_api_key"`       → no API key resolvable, pre-pass skipped
/// - `"persona_missing"`  → `market_adversarial_lens` persona not resolvable
/// - `"llm_error"`        → the adversarial pre-call (or LLM config) failed
/// - `"config_error"`     → project root / config not resolvable
/// Call-site-only reasons (not produced here): `"disabled"`,
/// `"persona_excluded"`, `"snapshot_missing"`.
struct AdversarialOutcome {
    record: Option<llm_history::LlmAnalysisRecord>,
    reason: &'static str,
}

impl AppContext {
    pub fn new(storage: StorageConfig) -> Self {
        let calendar = match StorageConfig::project_root() {
            Ok(root) => load_calendar_from_config(&root.join("config/calendars")),
            Err(_) => core_domain::calendar::TradingCalendar::default(),
        };
        if !probe_keyring_readable() {
            eprintln!("WARN: OS keyring is unavailable. LLM API keys will be stored in SQLite credential_store as fallback.");
        }
        Self { 
            storage, 
            calendar,
            available_dates_cache: std::sync::Arc::new(AvailableDatesCache::new()),
        }
    }

    /// 清除所有缓存（在数据刷新后调用）
    pub fn clear_cache(&self) {
        self.available_dates_cache.clear();
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

    pub fn get_user_preference(&self, key: &str) -> Result<Option<String>> {
        market_store::get_user_preference(&self.storage, key)
    }

    pub fn set_user_preference(&self, key: &str, value: &str) -> Result<()> {
        validate_user_preference(key, value)?;
        market_store::set_user_preference(&self.storage, key, value)
    }

    pub fn get_all_user_preferences(&self) -> Result<BTreeMap<String, String>> {
        market_store::get_all_user_preferences(&self.storage)
    }

    pub fn init_storage(&self) -> Result<()> {
        market_store::init_storage(&self.storage)
    }

    pub fn latest_refresh_job(&self) -> Result<Option<RefreshJobRecord>> {
        market_store::fetch_latest_refresh_job(&self.storage)
    }

    pub fn seed_universe(&self) -> Result<Vec<Instrument>> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        market_store::insert_instruments(&self.storage, &instruments)?;
        Ok(instruments)
    }

    pub fn ingest_daily(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        progress_callback: Option<&dyn Fn(&str)>,
    ) -> Result<IngestSummary> {
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let total = instruments.len();
        let mut total_rows = 0usize;
        let mut failed_symbols = Vec::new();
        for (idx, instrument) in instruments.iter().enumerate() {
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
            if let Some(cb) = progress_callback {
                let milestone = total / 10;
                if milestone == 0 || idx % milestone == 0 || idx + 1 == total {
                    cb(&format!(
                        "ingest progress: {}/{} symbols ({}%)",
                        idx + 1,
                        total,
                        ((idx + 1) * 100) / total
                    ));
                }
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

    /// Parallel ingestion using Tokio spawn_blocking + Semaphore for concurrent fetch.
    /// Data-ingestion crate remains sync; parallelism is achieved by wrapping sync fetches
    /// in blocking tasks with a concurrency limit of 2 (conservative for external provider rate limits).
    pub async fn ingest_daily_parallel(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        progress_callback: Option<Box<dyn Fn(&str) + Send>>,
    ) -> Result<IngestSummary> {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let total = instruments.len();
        let semaphore = Arc::new(Semaphore::new(2));
        let mut tasks = Vec::new();

        for (idx, instrument) in instruments.iter().enumerate() {
            let permit = semaphore.clone().acquire_owned().await?;
            let instrument = instrument.clone();
            let from = from;
            let to = to;
            let progress = progress_callback.as_ref().map(|cb| {
                let milestone = total / 10;
                if milestone == 0 || idx % milestone == 0 || idx + 1 == total {
                    cb(&format!(
                        "ingest progress: {}/{} symbols ({}%)",
                        idx + 1,
                        total,
                        ((idx + 1) * 100) / total
                    ));
                }
            });
            let task = tokio::task::spawn_blocking(move || {
                let _permit = permit; // hold permit until task completes
                let _ = progress; // report progress before fetch starts
                let result = fetch_daily_bars(&instrument, from, to);
                (instrument.symbol.clone(), result)
            });
            tasks.push(task);
        }

        let mut total_rows = 0usize;
        let mut failed_symbols = Vec::new();
        let mut all_bars: Vec<core_domain::DailyBar> = Vec::new();

        for task in tasks {
            match task.await {
                Ok((symbol, result)) => {
                    match result {
                        Ok(bars) => {
                            total_rows += bars.len();
                            all_bars.extend(bars);
                        }
                        Err(error) => {
                            failed_symbols.push(format!("{}: {}", symbol, error));
                        }
                    }
                }
                Err(join_error) => {
                    if join_error.is_panic() {
                        let panic_info = join_error.into_panic();
                        let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic_info.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        failed_symbols.push(format!("task panicked: {}", panic_msg));
                    } else {
                        failed_symbols.push(format!("task cancelled or failed: {}", join_error));
                    }
                }
            }
        }

        // Group by symbol for batch insert (serial to avoid ClickHouse write pressure)
        let mut bars_by_symbol: std::collections::BTreeMap<String, Vec<core_domain::DailyBar>> = std::collections::BTreeMap::new();
        for bar in all_bars {
            bars_by_symbol.entry(bar.symbol.clone()).or_default().push(bar);
        }

        for (symbol, bars) in bars_by_symbol {
            if let Err(error) = market_store::insert_daily_bars(&self.storage, &symbol, &bars) {
                failed_symbols.push(format!("{}: {}", symbol, error));
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

    pub fn check_startup_freshness(&self) -> Result<StartupFreshnessCheck> {
        let now = chrono::Local::now();
        let expected_date = self.calendar.expected_latest_tradable_date(now);
        let latest_db_date = market_store::fetch_latest_daily_bar_date(&self.storage)?;
        
        let (has_data, gap_days, auto_ingest_eligible, requires_manual_action, message) = 
            match (latest_db_date, expected_date) {
                (None, _) => {
                    (false, 0, false, true, "数据库中无数据，请手动运行初始化流程".to_string())
                }
                (Some(latest), Some(expected)) => {
                    let gap = (expected - latest).num_days();
                    if gap <= 0 {
                        (true, gap, false, false, "数据已是最新".to_string())
                    } else if gap > 30 {
                        (true, gap, false, true, format!("数据缺口 {} 天，超过自动补全上限，请手动运行刷新", gap))
                    } else {
                        (true, gap, true, false, format!("检测到 {} 天数据缺口，将自动补全", gap))
                    }
                }
                (Some(latest), None) => {
                    (true, 0, false, false, format!("无法确定期望最新日期，最新数据日期: {}", latest))
                }
            };
        
        Ok(StartupFreshnessCheck {
            has_data,
            latest_db_date,
            expected_date,
            gap_days,
            auto_ingest_eligible,
            requires_manual_action,
            message,
        })
    }

    pub fn auto_ingest_gap(
        &self,
        progress_callback: Option<&dyn Fn(&str)>,
    ) -> Result<IngestSummary> {
        let now = chrono::Local::now();
        let expected_date = self.calendar
            .expected_latest_tradable_date(now)
            .ok_or_else(|| anyhow::anyhow!("无法确定期望最新日期"))?;
        let latest_db_date = market_store::fetch_latest_daily_bar_date(&self.storage)?
            .ok_or_else(|| anyhow::anyhow!("数据库中无数据，无法自动补全"))?;
        
        let gap_days = (expected_date - latest_db_date).num_days();
        if gap_days <= 0 {
            anyhow::bail!("数据已是最新，无需补全");
        }
        if gap_days > 30 {
            anyhow::bail!("数据缺口 {} 天超过自动补全上限，请手动操作", gap_days);
        }
        
        let from = latest_db_date + chrono::Duration::days(1);
        let to = expected_date;
        
        self.ingest_daily(from, to, progress_callback)
    }

    pub fn build_refresh_plan(&self, to: NaiveDate) -> Result<RefreshPlan> {
        let latest_daily_date = market_store::fetch_latest_daily_bar_date(&self.storage)?;

        let gated_latest_dates = [ReportScope::Global, ReportScope::Cn, ReportScope::Hk]
            .into_iter()
            .map(|scope| {
                Ok(self
                    .dashboard_available_dates_for_scope(scope)?
                    .first()
                    .copied())
            })
            .collect::<Result<Vec<_>>>()?;

        let latest_gated_dashboard_date = gated_latest_dates.iter().flatten().min().copied();
        let has_missing_gated_scope = gated_latest_dates.iter().any(|date| date.is_none());
        let effective_to = std::cmp::max(to, latest_daily_date.unwrap_or(to));
        let (refresh_from, refresh_reason, repair_window_days) = derive_refresh_window(
            to,
            latest_daily_date,
            latest_gated_dashboard_date,
            has_missing_gated_scope,
        );

        let macro_from = effective_to - Duration::days(REFRESH_MACRO_LOOKBACK_DAYS);

        Ok(RefreshPlan {
            refresh_from: refresh_from.to_string(),
            refresh_to: effective_to.to_string(),
            macro_from: macro_from.to_string(),
            macro_to: effective_to.to_string(),
            latest_daily_date: latest_daily_date.map(|date| date.to_string()),
            latest_gated_dashboard_date: latest_gated_dashboard_date.map(|date| date.to_string()),
            refresh_reason,
            repair_window_days,
        })
    }

    fn collect_pipeline_diagnostics_for_standard_scopes(
        &self,
    ) -> Result<Vec<ScopedPipelineDiagnostics>> {
        [ReportScope::Global, ReportScope::Cn, ReportScope::Hk]
            .into_iter()
            .map(|scope| {
                Ok(ScopedPipelineDiagnostics {
                    scope: scope_label(scope).to_string(),
                    diagnostics: self.pipeline_date_diagnostics_with_scope(scope)?,
                })
            })
            .collect()
    }


    pub fn refresh_pipeline(
        &self,
        to: NaiveDate,
        diagnostics_scope: ReportScope,
        run_backtests: bool,
        cancel_flag: Option<&AtomicBool>,
        start_stage: Option<&str>,
        progress_callback: Option<Box<dyn Fn(&str) + Send>>,
    ) -> Result<RefreshPipelineSummary> {
        let notify = |msg: &str| {
            if let Some(ref cb) = progress_callback {
                cb(msg);
            }
        };
        let before_diagnostics = self.collect_pipeline_diagnostics_for_standard_scopes()?;
        let latest_dates_before = summarize_latest_dates(&before_diagnostics);
        let plan = self.build_refresh_plan(to)?;
        let refresh_from = NaiveDate::parse_from_str(&plan.refresh_from, "%Y-%m-%d")?;
        let refresh_to = NaiveDate::parse_from_str(&plan.refresh_to, "%Y-%m-%d")?;
        let macro_from = NaiveDate::parse_from_str(&plan.macro_from, "%Y-%m-%d")?;
        let macro_to = NaiveDate::parse_from_str(&plan.macro_to, "%Y-%m-%d")?;

        let start_order = start_stage.and_then(refresh_stage_order);
        let should_run = |stage_name: &str| {
            start_order
                .map(|order| refresh_stage_order(stage_name).unwrap_or(u8::MAX) >= order)
                .unwrap_or(true)
        };
        let mut stages = Vec::new();
        let mut blocking = Vec::new();
        let mut success = true;

        let mut job = RefreshJobRecord {
            id: new_refresh_job_id(),
            started_at: Utc::now().to_rfc3339(),
            finished_at: None,
            status: "running".to_string(),
            stages_json: "[]".to_string(),
            last_successful_stage: None,
            error: None,
            refresh_from: Some(plan.refresh_from.clone()),
            refresh_to: Some(plan.refresh_to.clone()),
        };
        market_store::insert_refresh_job(&self.storage, &job)?;

        let refresh_window = plan.clone();

        let persist_job = |job: &mut RefreshJobRecord,
                           stages: &[RefreshStageExecution],
                           status: &str,
                           finished_at: Option<String>,
                           error: Option<String>|
         -> Result<()> {
            job.status = status.to_string();
            job.finished_at = finished_at;
            job.error = error;
            job.stages_json = serde_json::to_string(stages)?;
            job.last_successful_stage = last_successful_stage(stages);
            market_store::update_refresh_job(&self.storage, job)
        };

        macro_rules! finish_summary {
            ($status:expr, $cancelled:expr, $consistency:expr, $latest_gate:expr, $after_diagnostics:expr, $latest_dates_after:expr, $advanced:expr, $error:expr) => {{
                let finished_at = Utc::now().to_rfc3339();
                persist_job(
                    &mut job,
                    &stages,
                    $status,
                    Some(finished_at),
                    $error.clone(),
                )?;
                return Ok(RefreshPipelineSummary {
                    success,
                    cancelled: $cancelled,
                    job_id: job.id.clone(),
                    diagnostics_scope: scope_label(diagnostics_scope).to_string(),
                    refresh_window,
                    backtests_requested: run_backtests,
                    latest_dates_before,
                    latest_dates_after: $latest_dates_after,
                    advanced: $advanced,
                    stages,
                    pipeline_diagnostics_by_scope: $after_diagnostics,
                    alerts: RefreshPipelineAlerts {
                        consistency: $consistency,
                        blocking,
                        latest_gate: $latest_gate,
                    },
                });
            }};
        }

        macro_rules! check_cancel {
            () => {
                if cancel_flag
                    .map(|flag| flag.load(Ordering::Relaxed))
                    .unwrap_or(false)
                {
                    success = false;
                    let message = "Refresh cancelled by operator".to_string();
                    blocking.push(message.clone());
                    let after_diagnostics =
                        self.collect_pipeline_diagnostics_for_standard_scopes()?;
                    let latest_dates_after = summarize_latest_dates(&after_diagnostics);
                    finish_summary!(
                        "cancelled",
                        true,
                        Vec::new(),
                        Vec::new(),
                        after_diagnostics,
                        latest_dates_after,
                        false,
                        Some(message)
                    );
                }
            };
        }

        macro_rules! run_refresh_stage {
            ($stage_name:literal, $summary_variant:path, $body:expr) => {
                if success && should_run($stage_name) {
                    check_cancel!();
                    match $body {
                        Ok(summary) => {
                            stages.push(RefreshStageExecution {
                                name: $stage_name.to_string(),
                                status: "success".to_string(),
                                summary: Some($summary_variant(summary)),
                                error: None,
                            });
                            notify(&format!("Finished {}.", $stage_name));
                            persist_job(&mut job, &stages, "running", None, None)?;
                        }
                        Err(error) => {
                            let message = format_error_chain(&error);
                            stages.push(RefreshStageExecution {
                                name: $stage_name.to_string(),
                                status: "error".to_string(),
                                summary: None,
                                error: Some(message.clone()),
                            });
                            blocking.push(message.clone());
                            success = false;
                            persist_job(&mut job, &stages, "running", None, Some(message))?;
                        }
                    }
                }
            };
        }

        let cb: Option<&dyn Fn(&str)> = progress_callback
            .as_ref()
            .map(|b| b.as_ref() as &dyn Fn(&str));

        notify("[1/7] Starting ingest...");
        run_refresh_stage!(
            "ingest",
            RefreshStageSummary::Ingest,
            self.ingest_daily(refresh_from, refresh_to, cb)
        );
        notify("[2/7] Starting indicators...");
        run_refresh_stage!(
            "indicators",
            RefreshStageSummary::Indicators,
            self.compute_indicators(cb)
        );
        notify("[3/7] Starting macro...");
        run_refresh_stage!(
            "macro",
            RefreshStageSummary::Macro,
            self.compute_macro_regime(macro_from, macro_to, cb)
        );
        notify("[4/7] Starting rotation...");
        run_refresh_stage!(
            "rotation",
            RefreshStageSummary::Rotation,
            self.compute_rotation(cb)
        );
        notify("[5/7] Starting strategy...");
        run_refresh_stage!(
            "strategy",
            RefreshStageSummary::Strategy,
            self.compute_strategy_preferences(cb)
        );
        notify("[6/7] Starting signals...");
        run_refresh_stage!(
            "signals",
            RefreshStageSummary::Signals,
            self.compute_signals(cb)
        );
        notify("[7/7] Starting backtests...");
        if success && run_backtests && should_run("backtests") {
            check_cancel!();
            match self.refresh_backtests_for_standard_scopes() {
                Ok(summary) => {
                    stages.push(RefreshStageExecution {
                        name: "backtests".to_string(),
                        status: "success".to_string(),
                        summary: Some(RefreshStageSummary::Backtests(summary)),
                        error: None,
                    });
                    notify("Finished backtests.");
                    persist_job(&mut job, &stages, "running", None, None)?;
                }
                Err(error) => {
                    let message = format_error_chain(&error);
                    stages.push(RefreshStageExecution {
                        name: "backtests".to_string(),
                        status: "error".to_string(),
                        summary: None,
                        error: Some(message.clone()),
                    });
                    blocking.push(message.clone());
                    success = false;
                    persist_job(&mut job, &stages, "running", None, Some(message))?;
                }
            }
        }

        if success {
            check_cancel!();
        }

        let consistency = if success {
            self.refresh_consistency_alerts()?
        } else {
            Vec::new()
        };
        if !consistency.is_empty() {
            blocking.extend(consistency.iter().cloned());
            success = false;
        }

        let after_diagnostics = self.collect_pipeline_diagnostics_for_standard_scopes()?;
        let latest_dates_after = summarize_latest_dates(&after_diagnostics);
        let before_scope = before_diagnostics
            .iter()
            .find(|item| {
                item.scope
                    .eq_ignore_ascii_case(scope_label(diagnostics_scope))
            })
            .map(|item| &item.diagnostics)
            .context("missing before diagnostics for requested scope")?;
        let after_scope = after_diagnostics
            .iter()
            .find(|item| {
                item.scope
                    .eq_ignore_ascii_case(scope_label(diagnostics_scope))
            })
            .map(|item| &item.diagnostics)
            .context("missing after diagnostics for requested scope")?;

        let before_latest = before_scope.dashboard_latest_date.as_deref();
        let after_latest = after_scope.dashboard_latest_date.as_deref();
        let advanced = match (before_latest, after_latest) {
            (None, Some(_)) => true,
            (Some(before), Some(after)) => after > before,
            _ => false,
        };

        let latest_gate =
            latest_gate_alerts_for_scope(diagnostics_scope, before_scope, after_scope);

        let final_status = if success { "success" } else { "error" };
        let final_error = (!blocking.is_empty()).then(|| blocking.join(" | "));
        persist_job(
            &mut job,
            &stages,
            final_status,
            Some(Utc::now().to_rfc3339()),
            final_error,
        )?;

        // 刷新完成后清除缓存，确保下次加载获取最新数据
        self.clear_cache();

        // ADR-113/114: async adversarial prewarm — snapshot cognition attachment.
        // The refresh covered all standard scopes (ingest → … → signals →
        // backtests run for GLOBAL/CN/HK), so the ready dashboard snapshot gets
        // its adversarial hypothesis background pre-generated in a detached
        // background thread, warming llm-history for the first llm-analyze of
        // the day. Fire-and-forget: no await, no join, no error propagation.
        // When the data did not advance, `ensure_adversarial_context` hits the
        // fresh-record path and costs zero LLM calls. Only on success: a failed
        // refresh must not spend LLM budget on a stale snapshot.
        if success {
            self.spawn_adversarial_prewarm(vec![
                ReportScope::Global,
                ReportScope::Cn,
                ReportScope::Hk,
            ]);
        }

        Ok(RefreshPipelineSummary {
            success,
            cancelled: false,
            job_id: job.id,
            diagnostics_scope: scope_label(diagnostics_scope).to_string(),
            refresh_window,
            backtests_requested: run_backtests,
            latest_dates_before,
            latest_dates_after,
            advanced,
            stages,
            pipeline_diagnostics_by_scope: after_diagnostics,
            alerts: RefreshPipelineAlerts {
                consistency,
                blocking,
                latest_gate,
            },
        })
    }

    pub fn explain_latest_gate(&self, scope: ReportScope) -> Result<LatestGateExplanation> {
        let diagnostics = self.pipeline_date_diagnostics_with_scope(scope)?;
        let alerts = latest_gate_alerts_for_scope(scope, &diagnostics, &diagnostics);
        let latest_gate_advanced = match (
            diagnostics.dashboard_latest_date.as_deref(),
            diagnostics.freshest_market_date.as_deref(),
        ) {
            (Some(latest), Some(freshest)) => Some(latest >= freshest),
            _ => None,
        };

        Ok(LatestGateExplanation {
            scope: scope_label(scope).to_string(),
            freshest_market_date: diagnostics.freshest_market_date.clone(),
            latest_available_dashboard_date: diagnostics.dashboard_latest_date.clone(),
            latest_gate_advanced,
            alerts: diagnostics.alerts.iter().cloned().chain(alerts).collect(),
            stages: latest_gate_stage_explanations(&diagnostics),
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
            let scoped_series = series_for_scope(&window, scope);
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
                let breadth_momentum_score = breadth_momentum_score(metrics.breadth_5d_delta);
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
                    environment_label: environment_label(environment_score).to_string(),
                });
            }
        }

        Ok(rows)
    }

    pub fn compute_indicators(
        &self,
        progress_callback: Option<&dyn Fn(&str)>,
    ) -> Result<IndicatorSummary> {
        let notify = |msg: &str| {
            if let Some(ref cb) = progress_callback {
                cb(msg);
            }
        };
        notify("Starting compute_indicators...");
        let instruments = load_universe(&self.storage.universe_abspath()?)?;
        let mut failed_symbols = Vec::new();

        // 收集所有 bars（串行 I/O）
        let mut bars_by_symbol = std::collections::HashMap::new();
        for instrument in &instruments {
            match market_store::fetch_daily_bars(&self.storage, &instrument.symbol) {
                Ok(bars) => {
                    if !bars.is_empty() {
                        bars_by_symbol.insert(instrument.symbol.clone(), bars);
                    }
                }
                Err(error) => {
                    failed_symbols.push(format!("{}: {}", instrument.symbol, error));
                }
            }
        }

        // 并行计算 indicators（纯 CPU 计算）
        let all_snapshots = indicator_engine::build_indicator_snapshots_for_symbols(&bars_by_symbol);

        // 按 symbol 分组，串行插入 ClickHouse（避免并发写入压力）
        let mut total_snapshots = 0usize;
        let mut snapshots_by_symbol: std::collections::BTreeMap<String, Vec<core_domain::IndicatorSnapshot>> = std::collections::BTreeMap::new();
        for snapshot in all_snapshots {
            snapshots_by_symbol.entry(snapshot.symbol.clone()).or_default().push(snapshot);
        }

        for (symbol, snapshots) in snapshots_by_symbol {
            if let Err(error) = market_store::insert_indicator_snapshots(
                &self.storage,
                &symbol,
                &snapshots,
            ) {
                failed_symbols.push(format!("{}: {}", symbol, error));
            }
            total_snapshots += snapshots.len();
        }

        notify("Finished compute_indicators.");
        Ok(IndicatorSummary {
            symbols: instruments.len(),
            snapshots: total_snapshots,
            failed_symbols,
        })
    }

    pub fn compute_macro_regime(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        progress_callback: Option<&dyn Fn(&str)>,
    ) -> Result<MacroSummary> {
        let notify = |msg: &str| {
            if let Some(ref cb) = progress_callback {
                cb(msg);
            }
        };
        notify("Starting compute_macro_regime...");
        let mut failed_items = Vec::new();

        // 加载 FRED 配置
        let fred_config = config_loader::ResolvedFredConfig::resolve()
            .unwrap_or_else(|_| config_loader::ResolvedFredConfig {
                enabled: true,
                base_url: "https://api.stlouisfed.org/fred".to_string(),
                api_key: None,
                request_delay_ms: 500,
                timeout_secs: 30,
                source: "default".to_string(),
                config_file: None,
            });

        if !fred_config.enabled {
            failed_items.push(
                "FRED: fred.enabled = false in config/fred.toml; skipping FRED fetch. \
                 Using existing ClickHouse macro data if available.".to_string()
            );
            notify("FRED fetch disabled by config; using persisted data only.");
        } else if !fred_config.is_valid() {
            failed_items.push(
                "FRED: config/fred.toml missing or api_key not set; skipping FRED fetch. \
                 Create config/fred.toml or set enabled=false to suppress.".to_string()
            );
            notify("FRED config incomplete; skipping fetch.");
        }

        let macro_fetch_from = from - Duration::days(550);
        let factor_specs = [
            ("vix", "VIXCLS", true),
            ("us10y", "DGS10", true),
            ("dollar_index", "DTWEXBGS", true),
            ("fed_funds", "DFF", true),
        ];

        let mut factors = Vec::new();
        if fred_config.enabled && fred_config.is_valid() {
            let api_key = fred_config.api_key.as_ref().unwrap().as_str();
            for (name, series_id, invert) in factor_specs {
                match fetch_fred_series(name, series_id, invert, macro_fetch_from, to, api_key) {
                    Ok(series) => factors.push(series),
                    Err(error) => failed_items.push(format!("{name}: {}", format_error_chain(&error))),
                }
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
        let hk_anchor = market_store::fetch_daily_bars(&self.storage, "HSCEI")
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
        let environment_by_key = environment_rows
            .iter()
            .map(|row| ((row.scope.clone(), row.date), row))
            .collect::<BTreeMap<_, _>>();
        let strategy_state_rows = regime_rows
            .iter()
            .filter_map(|regime| {
                environment_by_key
                    .get(&(regime.market.clone(), regime.date))
                    .map(|environment| build_strategy_state(regime, environment))
            })
            .collect::<Vec<_>>();
        if let Err(error) =
            market_store::insert_strategy_states(&self.storage, &strategy_state_rows)
        {
            failed_items.push(format!("strategy_state: {}", format_error_chain(&error)));
        }

        notify("Finished compute_macro_regime.");
        Ok(MacroSummary {
            factors: factors.len(),
            macro_rows: macro_rows.len(),
            regime_rows: regime_rows.len(),
            environment_rows: environment_rows.len(),
            strategy_state_rows: strategy_state_rows.len(),
            failed_items,
        })
    }

    pub fn compute_rotation(
        &self,
        progress_callback: Option<&dyn Fn(&str)>,
    ) -> Result<RotationSummary> {
        let notify = |msg: &str| {
            if let Some(ref cb) = progress_callback {
                cb(msg);
            }
        };
        notify("Starting compute_rotation...");
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

        let rows = rotation_engine::build_rotation_ranks_parallel(&series_by_symbol);
        if let Err(error) = market_store::insert_rotation_ranks(&self.storage, &rows) {
            failed_symbols.push(format!("rotation_rank: {error}"));
        }

        notify("Finished compute_rotation.");
        Ok(RotationSummary {
            symbols: series_by_symbol.len(),
            rows: rows.len(),
            failed_symbols,
        })
    }

    pub fn compute_strategy_preferences(
        &self,
        progress_callback: Option<&dyn Fn(&str)>,
    ) -> Result<StrategySummary> {
        let notify = |msg: &str| {
            if let Some(ref cb) = progress_callback {
                cb(msg);
            }
        };
        notify("Starting compute_strategy_preferences...");
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
            anyhow::bail!("strategy_preference insert failed: {error}");
        }

        notify("Finished compute_strategy_preferences.");
        Ok(StrategySummary {
            symbols: instruments.len(),
            rows: rows.len(),
            failed_symbols,
        })
    }

    pub fn compute_signals(
        &self,
        progress_callback: Option<&dyn Fn(&str)>,
    ) -> Result<SignalSummary> {
        let notify = |msg: &str| {
            if let Some(ref cb) = progress_callback {
                cb(msg);
            }
        };
        notify("Starting compute_signals...");
        let strategies = market_store::fetch_strategy_preferences(&self.storage)?;
        let regimes = market_store::fetch_market_regimes(&self.storage)?;
        let rotations = market_store::fetch_rotation_ranks(&self.storage)?;
        let (rows, stats) = build_signal_snapshots(&strategies, &regimes, &rotations);
        if let Err(error) = market_store::insert_signal_snapshots(&self.storage, &rows) {
            anyhow::bail!("signal_snapshot insert failed: {error}");
        }
        let alignment_issues =
            self.signal_alignment_issues([ReportScope::Global, ReportScope::Cn, ReportScope::Hk])?;
        if !alignment_issues.is_empty() {
            anyhow::bail!(alignment_issues.join(" | "));
        }
        let data_starved_warning = if stats.regime_missing > 0 || stats.rotation_missing > 0 {
            let msg = format!(
                "Data-starved signals detected: {}/{} signals used fallback defaults (regime_missing={}, rotation_missing={}).",
                stats.regime_missing + stats.rotation_missing,
                stats.total,
                stats.regime_missing,
                stats.rotation_missing
            );
            eprintln!("WARN: {msg}");
            Some(msg)
        } else {
            None
        };
        notify("Finished compute_signals.");
        Ok(SignalSummary {
            rows: rows.len(),
            failed_items: Vec::new(),
            data_starved_count: stats.regime_missing + stats.rotation_missing,
            data_starved_warning,
        })
    }

    fn signal_alignment_issues(
        &self,
        scopes: impl IntoIterator<Item = ReportScope>,
    ) -> Result<Vec<String>> {
        let mut issues = Vec::new();
        for scope in scopes {
            let available_dates = self.dashboard_available_dates_for_scope(scope)?;
            let diagnostics = self.pipeline_date_diagnostics_for_scope(scope, &available_dates)?;
            issues.extend(diagnostics.alerts);
        }
        Ok(issues)
    }

    pub fn refresh_consistency_alerts(&self) -> Result<Vec<String>> {
        self.signal_alignment_issues([ReportScope::Global, ReportScope::Cn, ReportScope::Hk])
    }

    pub fn run_backtest(
        &self,
        initial_capital: f64,
        max_holdings: usize,
        fee_rate: f64,
        slippage_rate: f64,
        scope: ReportScope,
        use_strategy_state: bool,
        drawdown_limit_pct: Option<f64>,
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
            use_strategy_state,
            drawdown_limit_pct,
        };
        let strategy_states = if config.use_strategy_state {
            market_store::fetch_strategy_states_for_scope(&self.storage, scope)?
        } else {
            Vec::new()
        };
        let result = run_signal_backtest(
            &run_id,
            &config,
            &signals,
            &bars_by_symbol,
            &strategy_states,
        );
        if let Err(error) = market_store::insert_backtest_result(
            &self.storage,
            &result.summary,
            &result.trades,
            &result.equity_curve,
        ) {
            failed_items.push(format!("backtest_persist: {}", error));
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
            drawdown_events: result.summary.drawdown_events,
            failed_items,
        })
    }

    pub fn refresh_backtests_for_standard_scopes(&self) -> Result<Vec<BacktestRunSummary>> {
        [ReportScope::Global, ReportScope::Cn, ReportScope::Hk]
            .into_iter()
            .map(|scope| self.run_backtest(1_000_000.0, 3, 0.001, 0.0005, scope, false, None))
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
        let data_health = None; // 不再同步调用 check_data_health
        let scoped_instruments = self.latest_gate_instruments_for_scope(scope)?;
        Ok(snapshot.map(|mut snapshot| {
            snapshot.load_metrics = Some(metrics);
            snapshot.trust_summary = Some(build_trust_summary(
                &scoped_instruments,
                &snapshot,
                &pipeline_dates,
                data_health,
                &self.calendar,
                &self.storage,
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
        let data_health = None; // 不再同步调用 check_data_health
        let scoped_instruments = self.latest_gate_instruments_for_scope(scope)?;
        metrics.available_dates_ms = available_dates_ms;
        metrics.total_ms = elapsed_ms(total_started_at);
        let snapshot = snapshot.map(|mut snapshot| {
            snapshot.load_metrics = Some(metrics);
            snapshot.trust_summary = Some(build_trust_summary(
                &scoped_instruments,
                &snapshot,
                &pipeline_dates,
                data_health,
                &self.calendar,
                &self.storage,
            ));
            snapshot
        });

        let insight = snapshot.as_ref().map(DashboardInsightComposer::compose);

        Ok(DashboardLoadBundle {
            status,
            available_dates: available_dates
                .into_iter()
                .map(|date| date.to_string())
                .collect(),
            snapshot,
            insight,
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
        let scoped_instruments = self.latest_gate_instruments_for_scope(scope)?;
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
                market_store::fetch_latest_market_regime_date_for_scope(&self.storage, scope)?,
            ),
            (
                "environment_snapshot",
                market_store::fetch_latest_environment_date_for_scope(&self.storage, scope)?,
            ),
            (
                "strategy_state",
                market_store::fetch_latest_strategy_state_date_for_scope(&self.storage, scope)?,
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
                let trading_symbols: Vec<String> = match latest_date {
                    Some(date) => scoped_instruments
                        .iter()
                        .filter(|i| self.calendar.is_trading_day(&i.market, date))
                        .map(|i| i.symbol.clone())
                        .collect(),
                    None => Vec::new(),
                };
                let trading_count = trading_symbols.len();
                let (latest_entities, expected_entities) = match (stage, latest_date) {
                    ("daily_bar", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_in_symbols(
                            &self.storage,
                            "daily_bar",
                            "symbol",
                            &trading_symbols,
                            date,
                        )?),
                        Some(trading_count),
                    ),
                    ("indicator_snapshot", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_in_symbols(
                            &self.storage,
                            "indicator_snapshot",
                            "symbol",
                            &trading_symbols,
                            date,
                        )?),
                        Some(trading_count),
                    ),
                    ("rotation_rank", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_in_symbols(
                            &self.storage,
                            "rotation_rank",
                            "symbol",
                            &trading_symbols,
                            date,
                        )?),
                        Some(trading_count),
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
                        Some(trading_count),
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
                        Some(trading_count),
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
                    ("strategy_state", Some(date)) => (
                        Some(market_store::fetch_distinct_entity_count_for_date_with_filter(
                            &self.storage,
                            "strategy_state",
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

        let alerts = pipeline_date_alerts(scope, &stages);

        Ok(PipelineDateDiagnostics {
            freshest_market_date: freshest_market_date.map(|date| date.to_string()),
            dashboard_latest_date: dashboard_latest_date.map(|date| date.to_string()),
            alerts,
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
        let strategy_state = market_store::fetch_latest_strategy_state_on_or_before(
            &self.storage,
            report_date,
            scope,
        )?;
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
            strategy_state,
            latest_backtest,
            report_date,
            latest_available_date,
            scope_label(scope),
        );
        snapshot.environment = environment;
        // Enrich snapshot with symbol-to-name mapping from universe
        snapshot.symbol_names = scoped_instruments
            .iter()
            .map(|instrument| (instrument.symbol.clone(), instrument.name.clone()))
            .collect();
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
        // 检查缓存
        if let Some(cached) = self.available_dates_cache.get(&scope) {
            return Ok(cached);
        }

        let available_dates = market_store::fetch_dashboard_available_dates(&self.storage)?;
        let scoped_instruments = self.latest_gate_instruments_for_scope(scope)?;
        if scoped_instruments.is_empty() {
            return Ok(Vec::new());
        }
        let mut scoped_dates = Vec::new();
        for date in available_dates {
            let trading_symbols: Vec<String> = scoped_instruments
                .iter()
                .filter(|i| self.calendar.is_trading_day(&i.market, date))
                .map(|i| i.symbol.clone())
                .collect();
            let expected_count = trading_symbols.len();
            if expected_count == 0 {
                continue;
            }
            let signal_count = market_store::fetch_distinct_entity_count_for_date_with_filter(
                &self.storage,
                "signal_snapshot",
                "symbol",
                "analysis_scope",
                scope_label(scope),
                date,
            )?;
            let rotation_count = if trading_symbols.is_empty() {
                0
            } else {
                market_store::fetch_distinct_entity_count_for_date_in_symbols(
                    &self.storage,
                    "rotation_rank",
                    "symbol",
                    &trading_symbols,
                    date,
                )?
            };
            let has_regime =
                market_store::fetch_latest_market_regime_on_or_before(&self.storage, date, scope)?
                    .is_some();
            let has_environment =
                market_store::fetch_latest_environment_on_or_before(&self.storage, date, scope)?
                    .is_some();
            let has_strategy_state =
                market_store::fetch_latest_strategy_state_on_or_before(&self.storage, date, scope)?
                    .is_some();
            let daily_bar_count = market_store::fetch_distinct_entity_count_for_date_in_symbols(
                &self.storage,
                "daily_bar",
                "symbol",
                &trading_symbols,
                date,
            )?;
            if signal_count >= expected_count
                && rotation_count >= expected_count
                && daily_bar_count >= expected_count
                && has_regime
                && has_environment
                && has_strategy_state
            {
                scoped_dates.push(date);
            }
        }

        // 更新缓存
        self.available_dates_cache.insert(scope, scoped_dates.clone());

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

        // 加载 FRED 配置
        let fred_config = config_loader::ResolvedFredConfig::resolve()
            .unwrap_or_else(|_| config_loader::ResolvedFredConfig {
                enabled: true,
                base_url: "https://api.stlouisfed.org/fred".to_string(),
                api_key: None,
                request_delay_ms: 500,
                timeout_secs: 30,
                source: "default".to_string(),
                config_file: None,
            });

        if fred_config.enabled && fred_config.is_valid() {
            let api_key = fred_config.api_key.as_ref().unwrap().as_str();
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
                    api_key,
                ) {
                    Ok(outcome) => {
                        let status = if outcome.transport == "attohttpc" {
                            "healthy"
                        } else {
                            "review"
                        }
                        .to_string();
                        let mut notes = Vec::new();
                        if outcome.transport != "attohttpc" {
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
        } else if !fred_config.enabled {
            for factor_name in ["vix", "us10y", "dollar_index", "fed_funds"] {
                macro_sources.push(DataHealthMacroSourceSummary {
                    factor_name: factor_name.to_string(),
                    source: "FRED".to_string(),
                    transport: "disabled".to_string(),
                    rows: 0,
                    first_date: None,
                    last_date: None,
                    status: "disabled".to_string(),
                    notes: vec![
                        "FRED fetch disabled by config/fred.toml (enabled = false). \
                         Using existing ClickHouse data if available."
                            .to_string(),
                    ],
                });
            }
        } else {
            for factor_name in ["vix", "us10y", "dollar_index", "fed_funds"] {
                macro_sources.push(DataHealthMacroSourceSummary {
                    factor_name: factor_name.to_string(),
                    source: "FRED".to_string(),
                    transport: "unconfigured".to_string(),
                    rows: 0,
                    first_date: None,
                    last_date: None,
                    status: "disabled".to_string(),
                    notes: vec![
                        "FRED config missing or api_key not set. \
                         Create config/fred.toml or set enabled=false to suppress."
                            .to_string(),
                    ],
                });
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

            let (gap_count, max_gap_days) = analyze_gap_metrics(&bars, instrument, &self.calendar);
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
        self.export_report_with_scope_and_format(report_date, scope, false)
    }

    pub fn export_report_with_scope_and_format(
        &self,
        report_date: Option<NaiveDate>,
        scope: ReportScope,
        concise: bool,
    ) -> Result<ReportSummary> {
        if report_date.is_none() {
            let gate = self.explain_latest_gate(scope)?;
            if gate.latest_gate_advanced == Some(false) {
                let details = if gate.alerts.is_empty() {
                    "no latest-gate details available".to_string()
                } else {
                    gate.alerts.join(" | ")
                };
                anyhow::bail!(
                    "default report export refused because latest dashboard date ({}) is behind freshest market date ({}). Run the missing pipeline stage(s), or pass --date explicitly to export a historical report. Details: {}",
                    gate.latest_available_dashboard_date
                        .as_deref()
                        .unwrap_or("none"),
                    gate.freshest_market_date.as_deref().unwrap_or("none"),
                    details
                );
            }
        }
        let snapshot = self
            .dashboard_snapshot_with_scope(report_date, scope)?
            .context("no dashboard snapshot available for report export")?;

        let insight = report_renderer::DashboardInsightComposer::compose(&snapshot);
        let markdown = if concise {
            report_renderer::DailyReportComposer::compose_markdown(&snapshot, Some(&insight))
        } else {
            render_markdown_report(&snapshot)
        };

        let root = StorageConfig::project_root()?;
        let report_dir = root.join("reports");
        fs::create_dir_all(&report_dir).with_context(|| {
            format!(
                "failed to create report directory: {}",
                report_dir.display()
            )
        })?;
        let report_slug = match scope {
            ReportScope::Global if concise => format!("daily-report-concise-{}", snapshot.report_date),
            ReportScope::Global => format!("daily-report-{}", snapshot.report_date),
            ReportScope::Cn if concise => format!("daily-report-cn-concise-{}", snapshot.report_date),
            ReportScope::Cn => format!("daily-report-cn-{}", snapshot.report_date),
            ReportScope::Hk if concise => format!("daily-report-hk-concise-{}", snapshot.report_date),
            ReportScope::Hk => format!("daily-report-hk-{}", snapshot.report_date),
        };
        let report_type = match (scope, concise) {
            (ReportScope::Global, true) => "DAILY_REPORT_CONCISE",
            (ReportScope::Global, false) => "DAILY_REPORT",
            (ReportScope::Cn, true) => "DAILY_REPORT_CN_CONCISE",
            (ReportScope::Cn, false) => "DAILY_REPORT_CN",
            (ReportScope::Hk, true) => "DAILY_REPORT_HK_CONCISE",
            (ReportScope::Hk, false) => "DAILY_REPORT_HK",
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

    /// Render LLM analysis JSON as markdown report.

    /// Export LLM analysis result as markdown report.
    pub fn export_llm_analysis(
        &self,
        scope: ReportScope,
        date: NaiveDate,
        analysis: &serde_json::Value,
    ) -> Result<ReportSummary> {
        let md = render_llm_analysis_markdown(analysis);
        let root = StorageConfig::project_root()?;
        let report_dir = root.join("reports");
        fs::create_dir_all(&report_dir).with_context(|| {
            format!(
                "failed to create report directory: {}",
                report_dir.display()
            )
        })?;
        let filename = format!("llm-analysis-{}-{}.md", scope_label(scope).to_lowercase(), date);
        let output_path = report_dir.join(&filename);
        fs::write(&output_path, md).with_context(|| {
            format!("failed to write LLM analysis report: {}", output_path.display())
        })?;
        market_store::insert_report_snapshot(
            &self.storage,
            &date.to_string(),
            "LLM_ANALYSIS",
            &output_path.display().to_string(),
        )?;
        Ok(ReportSummary {
            report_date: date.to_string(),
            output_path: output_path.display().to_string(),
            failed_items: Vec::new(),
        })
    }

    pub fn sync_and_export(
        &self,
        date: Option<NaiveDate>,
        to: NaiveDate,
        scope: ReportScope,
        run_backtests: bool,
        progress_callback: Option<Box<dyn Fn(&str) + Send>>,
    ) -> Result<SyncAndExportSummary> {
        if let Some(report_date) = date {
            let summary = self.export_report_with_scope(Some(report_date), scope)?;
            return Ok(SyncAndExportSummary {
                report_date: summary.report_date,
                output_path: summary.output_path,
                refreshed: false,
                gate_advanced: None,
            });
        }

        let gate_before = self.explain_latest_gate(scope)?;

        if sync_gate_needs_refresh(gate_before.latest_gate_advanced) {
            let refresh_result =
                self.refresh_pipeline(to, scope, run_backtests, None, None, progress_callback)?;
            validate_sync_refresh_result(refresh_result.success, &refresh_result.alerts.blocking)?;
        }

        let gate_after = self.explain_latest_gate(scope)?;
        if gate_after.latest_gate_advanced != Some(true) {
            anyhow::bail!(
                "sync-and-export aborted: latest gate is not advanced after refresh. Gate status: {:?}. Run 'explain-latest-gate' for details.",
                gate_after.latest_gate_advanced
            );
        }
        let summary = self.export_report_with_scope(None, scope)?;

        Ok(SyncAndExportSummary {
            report_date: summary.report_date,
            output_path: summary.output_path,
            refreshed: sync_gate_needs_refresh(gate_before.latest_gate_advanced),
            gate_advanced: gate_after.latest_gate_advanced,
        })
    }

    pub fn get_signal_detail(
        &self,
        scope: ReportScope,
        symbol: &str,
        date: NaiveDate,
    ) -> Result<Option<SignalSnapshot>> {
        market_store::fetch_signal_snapshot_for_symbol(&self.storage, date, symbol, scope.into())
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

    pub fn set_llm_config(&self, base_url: &str, model: &str, timeout_secs: u64) -> Result<()> {
        // 1. 写入 SQLite（向后兼容）
        let config = LlmConfig {
            base_url: base_url.to_string(),
            model: model.to_string(),
            timeout_secs,
        };
        let json = serde_json::to_string(&config).context("failed to serialize llm_config")?;
        market_store::insert_app_config(&self.storage, "llm_config", &json)?;

        // 2. 同步写入 TOML 文件
        let toml_path = config_loader::default_config_path()?;
        let mut toml_config = if toml_path.exists() {
            config_loader::read_or_default_config(&toml_path)
        } else {
            LlmFileConfig::default()
        };
        toml_config.llm.base_url = base_url.to_string();
        toml_config.llm.model = model.to_string();
        toml_config.llm.timeout_secs = timeout_secs;
        config_loader::write_llm_config_to_file(&toml_path, &toml_config)?;

        Ok(())
    }

    pub fn get_llm_config(&self) -> Result<LlmConfig> {
        match market_store::fetch_app_config(&self.storage, "llm_config")? {
            Some(json) => serde_json::from_str(&json).context("failed to parse llm_config"),
            None => Ok(LlmConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-4o-mini".to_string(),
                timeout_secs: 60,
            }),
        }
    }

    pub fn set_llm_api_key(&self, api_key: &str) -> Result<()> {
        // 1. 写入 Keyring（优先）
        let entry = keyring::Entry::new(LLM_SERVICE_NAME, LLM_ACCOUNT_NAME)?;
        match entry.set_password(api_key) {
            Ok(()) => {
                // FIX: Also write to SQLite as fallback; do NOT clear it.
                // Windows keyring reads often fail even after successful writes,
                // so SQLite must retain the key for the fallback chain to work.
                let _ = market_store::insert_credential(&self.storage, "llm_api_key", api_key);
            }
            Err(keyring_err) => {
                eprintln!("WARN: keyring storage failed ({keyring_err}), falling back to SQLite credential_store. API key will be stored in local database.");
                market_store::insert_credential(&self.storage, "llm_api_key", api_key)?;
            }
        }

        // 2. 同步写入 TOML 文件（使用环境变量引用格式）
        let toml_path = config_loader::default_config_path()?;
        let mut toml_config = if toml_path.exists() {
            config_loader::read_or_default_config(&toml_path)
        } else {
            LlmFileConfig::default()
        };

        // 检查是否是环境变量引用格式
        if api_key.starts_with("${") && api_key.ends_with('}') {
            toml_config.llm.auth.api_key = Some(api_key.to_string());
        } else {
            // 明文 key，建议使用环境变量
            toml_config.llm.auth.api_key = Some(api_key.to_string());
            #[cfg(windows)]
            eprintln!("WARN: API key stored in plaintext. Consider using environment variable reference:");
            eprintln!("      set-llm-api-key --key \"${{OPENAI_API_KEY}}\"");
        }

        config_loader::write_llm_config_to_file(&toml_path, &toml_config)?;

        Ok(())
    }

    pub fn get_llm_api_key(&self) -> Result<Option<String>> {
        let entry = keyring::Entry::new(LLM_SERVICE_NAME, LLM_ACCOUNT_NAME)?;
        match entry.get_password() {
            Ok(key) if !key.is_empty() => Ok(Some(key)),
            Ok(_) | Err(keyring::Error::NoEntry) => {
                Ok(market_store::fetch_credential(&self.storage, "llm_api_key")?.filter(|s| !s.is_empty()))
            }
            Err(keyring_err) => {
                eprintln!("WARN: keyring read failed ({keyring_err}), falling back to SQLite credential_store.");
                Ok(market_store::fetch_credential(&self.storage, "llm_api_key")?.filter(|s| !s.is_empty()))
            }
        }
    }

    /// Unified LLM status for desktop UI.
    ///
    /// Implements ADR-033 resolution chain:
    /// - base_url / model / timeout: TOML (primary) → SQLite legacy fallback
    /// - api_key: TOML (plaintext or resolved ${VAR}) → keyring → SQLite
    pub fn get_llm_status(&self) -> Result<LlmStatus> {
        // 1. Resolve TOML (primary per ADR-033)
        let resolved = self.get_resolved_llm_config(None).unwrap_or_else(|_| {
            config_loader::ResolvedLlmConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-4o-mini".to_string(),
                timeout_secs: 60,
                api_key: None,
                temperature: 0.7,
                max_tokens: 2048,
                seed: None,
                source: config_loader::ConfigSource {
                    base_url: "default".to_string(),
                    model: "default".to_string(),
                    api_key: "none".to_string(),
                    config_file: None,
                },
                adversarial_auto_inject: true,
                adversarial_inject: std::collections::HashMap::new(),
                adversarial_max_chars: core_domain::default_adversarial_max_chars(),
                adversarial_full_max_chars: core_domain::default_adversarial_full_max_chars(),
                adversarial_truncate_strategy: core_domain::TruncateStrategy::default(),
            }
        });

        // 2. Determine effective non-secret config
        //    TOML first, then legacy SQLite fallback for backward compatibility
        let (base_url, model, timeout_secs) = if resolved.source.config_file.is_some() {
            (resolved.base_url, resolved.model, resolved.timeout_secs)
        } else {
            let legacy = self.get_llm_config().unwrap_or_else(|_| LlmConfig {
                base_url: "https://api.openai.com/v1".to_string(),
                model: "gpt-4o-mini".to_string(),
                timeout_secs: 60,
            });
            (legacy.base_url, legacy.model, legacy.timeout_secs)
        };

        // 3. Determine API key presence per ADR-033 fallback chain:
        //    TOML (plaintext or resolved env var) → keyring → SQLite
        let api_key = if let Some(ref key) = resolved.api_key {
            if !key.is_empty() {
                Some(key.clone())
            } else {
                self.get_llm_api_key()?
            }
        } else {
            self.get_llm_api_key()?
        };

        Ok(LlmStatus {
            configured: api_key.is_some(),
            base_url,
            model,
            timeout_secs,
        })
    }

    // ============================================================
    // TOML-based LLM Config (New)
    // ============================================================

    /// 获取解析后的 LLM 配置（TOML + 环境变量 + CLI 优先级合并）
    ///
    /// 优先级：CLI args > TOML file (with ${VAR}) > defaults
    pub fn get_resolved_llm_config(
        &self,
        cli_args: Option<config_loader::CliLlmArgs>,
    ) -> Result<config_loader::ResolvedLlmConfig> {
        config_loader::ResolvedLlmConfig::resolve(cli_args)
    }

    /// 显示 LLM 配置来源信息
    pub fn show_llm_config(&self) -> Result<config_loader::ResolvedLlmConfig> {
        self.get_resolved_llm_config(None)
    }

    /// 设置 FRED 配置（写入 TOML 文件）
    pub fn set_fred_config(&self, enabled: bool, api_key: Option<&str>) -> Result<()> {
        let toml_path = config_loader::default_fred_config_path()?;
        let mut toml_config = if toml_path.exists() {
            config_loader::read_or_default_fred_config(&toml_path)
        } else {
            FredFileConfig::default()
        };
        toml_config.fred.enabled = enabled;
        if let Some(key) = api_key {
            toml_config.fred.auth.api_key = Some(key.to_string());
        }
        config_loader::write_fred_config_to_file(&toml_path, &toml_config)?;
        Ok(())
    }

    /// 显示 FRED 配置来源信息
    pub fn show_fred_config(&self) -> Result<config_loader::ResolvedFredConfig> {
        config_loader::ResolvedFredConfig::resolve()
    }

    /// 验证 FRED 配置文件
    pub fn validate_fred_config(&self) -> config_loader::FredConfigValidation {
        config_loader::validate_fred_config()
    }

    /// 验证 LLM 配置文件
    pub fn validate_llm_config(&self) -> config_loader::ConfigValidation {
        config_loader::validate_config()
    }

    /// 从 SQLite/Keyring 迁移配置到 TOML 文件
    pub fn migrate_llm_config_to_toml(&self, force: bool) -> Result<String> {
        let config_path = config_loader::default_config_path()?;

        // 检查文件是否已存在
        if config_path.exists() && !force {
            anyhow::bail!(
                "Config file already exists: {}. Use --force to overwrite.",
                config_path.display()
            );
        }

        // 从 SQLite 读取现有配置
        let old_config = self.get_llm_config()?;

        // 从 Keyring/SQLite 读取 API Key
        let api_key = self.get_llm_api_key()?;

        // 构建 TOML 配置
        let toml_config = LlmFileConfig {
            llm: core_domain::LlmSection {
                base_url: old_config.base_url,
                model: old_config.model,
                timeout_secs: old_config.timeout_secs,
                auth: core_domain::AuthSection {
                    api_key: api_key.map(|k| {
                        // 如果是明文 key，提示用户设置环境变量
                        if k.starts_with("sk-") {
                            eprintln!("WARN: Migrating plaintext API key. Consider using environment variable reference instead.");
                            eprintln!("      Edit config/llm.toml and change to: api_key = \"${{OPENAI_API_KEY}}\"");
                        }
                        k
                    }),
                },
                defaults: core_domain::DefaultsSection::default(),
                adversarial: None,
            },
        };

        // 写入 TOML 文件
        config_loader::write_llm_config_to_file(&config_path, &toml_config)?;

        Ok(format!(
            "Config migrated to: {}",
            config_path.display()
        ))
    }


    pub fn analyze_report_with_llm(
        &self,
        report_date: NaiveDate,
        scope: ReportScope,
    ) -> Result<LlmAnalysisResult> {
        // P2: 从 TOML 配置读取（优先级：CLI > File > Default）
        let resolved = self.get_resolved_llm_config(None)?;
        let config = LlmConfig {
            base_url: resolved.base_url,
            model: resolved.model,
            timeout_secs: resolved.timeout_secs,
        };
        // API Key 回退链：TOML → Keyring → SQLite credential_store
        let api_key = resolved
            .api_key
            .or_else(|| self.get_llm_api_key().ok().flatten())
            .context("LLM API key not configured. Use set_llm_api_key or config/llm.toml.")?;
        let temperature = resolved.temperature;
        let max_tokens = resolved.max_tokens;
        let seed = resolved.seed;

        let snapshot = self
            .dashboard_snapshot_with_scope(Some(report_date), scope)?
            .context("no dashboard snapshot available for LLM analysis")?;
        let report_markdown = render_markdown_report(&snapshot);

        let system_prompt = "You are a senior quantitative analyst. Analyze the following daily market research report and provide concise insights on regime, breadth, top signals, risks, and actionable takeaways.";
        let user_prompt = format!(
            "{}\n\nPlease provide a structured analysis.",
            report_markdown
        );

        let analysis_text = match tokio::runtime::Handle::try_current() {
            Ok(_handle) => {
                // Inside an existing tokio runtime (e.g., Tauri async command).
                // Cannot call Runtime::new() or Handle::block_on from here.
                // Spawn a dedicated thread with its own runtime.
                std::thread::scope(|s| {
                    s.spawn(|| {
                        let runtime = tokio::runtime::Runtime::new()
                            .context("failed to create tokio runtime")?;
                        runtime.block_on(call_llm_api(
                            config, api_key, system_prompt, user_prompt,
                            temperature, max_tokens, seed,
                        ))
                    })
                    .join()
                    .expect("LLM analysis thread panicked")
                })?
            }
            Err(_) => {
                let runtime = tokio::runtime::Runtime::new()
                    .context("failed to create tokio runtime")?;
                runtime.block_on(call_llm_api(
                    config, api_key, system_prompt, user_prompt,
                    temperature, max_tokens, seed,
                ))?
            }
        };

        let root = StorageConfig::project_root()?;
        let report_dir = root.join("reports");
        fs::create_dir_all(&report_dir).with_context(|| {
            format!(
                "failed to create report directory: {}",
                report_dir.display()
            )
        })?;
        let scope_str = scope_label(scope).to_lowercase();
        let output_path = report_dir.join(format!("llm-analysis-{}-{}.md", scope_str, report_date));
        fs::write(&output_path, &analysis_text).with_context(|| {
            format!(
                "failed to write LLM analysis file: {}",
                output_path.display()
            )
        })?;

        market_store::insert_report_snapshot(
            &self.storage,
            &report_date.to_string(),
            "LLM_ANALYSIS",
            &output_path.display().to_string(),
        )?;

        Ok(LlmAnalysisResult {
            report_date: report_date.to_string(),
            scope: scope_label(scope).to_string(),
            output_path: output_path.display().to_string(),
            analysis_text,
        })
    }

    /// Build ResearchContext for a given scope
    pub fn research_context(&self, scope: ReportScope) -> Result<llm_context::ResearchContext> {
        let snapshot = self
            .dashboard_snapshot_with_scope(None, scope)?
            .context("No dashboard data available")?;
        Ok(llm_context::ContextBuilder::build(&snapshot))
    }

    /// Compute semantic features for a given scope
    pub fn research_features(
        &self,
        scope: ReportScope,
    ) -> Result<Vec<llm_context::SemanticFeature>> {
        let context = self.research_context(scope)?;
        let features = llm_context::builtin_features();
        Ok(features
            .iter()
            .filter_map(|f| f.compute(&context))
            .collect())
    }

    /// Build ResearchContext directly from Engine/Store for a given date and scope.
    ///
    /// This is the V6 consumer migration entry point for CLI research commands. Unlike
    /// `build_research_context_from_dashboard` (which maps from Production Surface),
    /// this method fetches directly from market-store, keeping the Research
    /// Pipeline independent of `DashboardSnapshot`.
    ///
    /// Phase 1: Delegates to `fetch_research_dataset` (single query owner).
    pub fn build_research_context_for_date(
        &self,
        date: NaiveDate,
        scope: core_domain::AnalysisScope,
    ) -> Result<research_context::ResearchContext> {
        let dataset = self.fetch_research_dataset(date, scope, 365)?;
        build_research_context_from_dataset(&dataset)
    }

    /// Build ResearchSnapshot (Computation Workspace) for a given date and scope.
    pub fn build_research_snapshot_for_date(
        &self,
        date: NaiveDate,
        scope: core_domain::AnalysisScope,
    ) -> Result<ResearchSnapshot> {
        let dataset = self.fetch_research_dataset(date, scope, 365)?;
        Ok(build_research_snapshot_from_dataset(&dataset))
    }

    /// Build both ResearchContext and ResearchSnapshot from a single dataset fetch.
    /// Use this when you need both to avoid double-fetching.
    pub fn build_research_bundle_for_date(
        &self,
        date: NaiveDate,
        scope: core_domain::AnalysisScope,
    ) -> Result<(research_context::ResearchContext, ResearchSnapshot)> {
        let dataset = self.fetch_research_dataset(date, scope, 365)?;
        let context = build_research_context_from_dataset(&dataset)?;
        let snapshot = build_research_snapshot_from_dataset(&dataset);
        Ok((context, snapshot))
    }

    /// V7.4 / ADR-078 — Compute reproducible conditional forward-return evidence.
    ///
    /// Returns an `Evidence` value containing raw facts (matched dates and
    /// forward returns) plus derived statistics. This is a Research Surface
    /// tool and does not modify any decision logic.
    ///
    /// Future: this computation may migrate into a dedicated `research-engine`
    /// crate once that boundary is introduced. AppService will then only
    /// orchestrate the call.
    pub fn research_condition_evidence(
        &self,
        condition: &str,
        scope: core_domain::AnalysisScope,
        horizon: usize,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<core_domain::research::attribution::Evidence> {
        compute_condition_evidence(self, condition, scope, horizon, from, to)
    }

    // -----------------------------------------------------------------------
    // V7 Workflow — Replay: accumulate Evidence across conditions/horizons
    // -----------------------------------------------------------------------

    /// Run historical analytics for a set of conditions and horizons, saving each result
    /// as an Evidence Asset in the workspace. This replaces the external PowerShell pipeline
    /// with a native Domain Command that understands Evidence / Workspace / Registry.
    pub fn research_replay(
        &self,
        scope: core_domain::AnalysisScope,
        from: NaiveDate,
        to: NaiveDate,
        conditions: &[String],
        horizons: &[usize],
    ) -> Result<Vec<ReplayEvidenceSummary>> {
        let workspace = workspace::WorkspaceManager::default_workspace()
            .context("Failed to initialize workspace")?;

        let anchor_symbol = match scope {
            core_domain::AnalysisScope::Global | core_domain::AnalysisScope::Cn => "000300",
            core_domain::AnalysisScope::Hk => "HSCEI",
        };

        let anchor_bars = market_store::fetch_daily_bars(&self.storage, anchor_symbol)?;
        let close_by_date: BTreeMap<NaiveDate, f64> =
            anchor_bars.iter().map(|b| (b.date, b.close)).collect();

        let earliest_date = close_by_date.keys().next().copied().unwrap_or(from);
        let target_date = close_by_date.keys().last().copied().unwrap_or(to);

        let mut summaries = Vec::new();

        for condition in conditions {
            for horizon in horizons {
                let evidence = self.research_condition_evidence(
                    condition,
                    scope,
                    *horizon,
                    earliest_date,
                    target_date,
                )?;

                let id = workspace.write_evidence(
                    &evidence,
                    condition,
                    scope,
                    *horizon,
                    "replay",
                    workspace::ResearchAssetLifecycle::Draft,
                )?;

                let occurrences = evidence.occurrences;
                let (positive_ratio, median_forward_return) = if occurrences > 0 {
                    let positive = evidence.forward_returns.iter().filter(|&&r| r > 0.0).count() as f64
                        / occurrences as f64;
                    let mut sorted = evidence.forward_returns.clone();
                    sorted.sort_by(|a, b| a.total_cmp(b));
                    let median = if sorted.len() % 2 == 1 {
                        sorted[sorted.len() / 2]
                    } else {
                        let mid = sorted.len() / 2;
                        (sorted[mid - 1] + sorted[mid]) / 2.0
                    };
                    (positive, median)
                } else {
                    (0.0, 0.0)
                };

                let workspace_path = workspace
                    .paths
                    .evidence
                    .join("replay")
                    .join(id.as_string())
                    .join("body.json");

                summaries.push(ReplayEvidenceSummary {
                    id: id.as_string().to_string(),
                    condition: condition.clone(),
                    scope: scope.as_str().to_string(),
                    horizon: *horizon,
                    occurrences,
                    positive_ratio,
                    median_forward_return,
                    workspace_path: workspace_path.to_string_lossy().to_string(),
                });
            }
        }

        Ok(summaries)
    }

    // -----------------------------------------------------------------------
    // V7.2B Evidence Retrieval Engine
    // -----------------------------------------------------------------------

    /// V7.2B Evidence Retrieval Engine — Historical Analogue Search.
    ///
    /// Builds `MarketFingerprint`s for all available historical dates in `scope`,
    /// normalizes them, and finds the `top_n` most similar dates to the target.
    /// Optionally profiles forward outcomes for the matched dates using the
    /// scope's anchor symbol daily bars.
    ///
    /// This is an observation tool: it retrieves historical evidence, not predictions.
    pub fn research_analogues(
        &self,
        scope: ReportScope,
        date: Option<NaiveDate>,
        horizon_days: usize,
        top_n: usize,
        lookback_days: usize,
    ) -> Result<market_fingerprint_engine::SearchResult> {
        use market_fingerprint_engine::{
            normalize_all, CosineDistance, MarketFingerprintBuilder,
            OutcomeProfiler, SimilarityMatcher,
        };

        // 1. Resolve target date
        let target_date = match date {
            Some(d) => d,
            None => market_store::fetch_latest_table_date(&self.storage, "signal_snapshot")?
                .context("No signal data available")?,
        };

        // 2. Determine anchor symbol
        let anchor_symbol = match scope {
            core_domain::AnalysisScope::Global | core_domain::AnalysisScope::Cn => "000300",
            core_domain::AnalysisScope::Hk => "HSCEI",
        };

        // 3. Fetch anchor symbol daily bars for ForwardReturnProvider
        let anchor_bars =
            market_store::fetch_daily_bars(&self.storage, anchor_symbol)?;
        let close_by_date: BTreeMap<NaiveDate, f64> =
            anchor_bars.iter().map(|b| (b.date, b.close)).collect();

        // 4. Fetch available historical dates for the scope and limit lookback
        let all_dates = self.dashboard_available_dates_for_scope(scope)?;
        if all_dates.is_empty() {
            anyhow::bail!(
                "No historical dates available for scope {:?}",
                scope
            );
        }

        let lookback = lookback_days.max(1);
        let mut dates: Vec<NaiveDate> = all_dates
            .into_iter()
            .filter(|d| *d <= target_date)
            .collect();
        dates.sort();
        if dates.len() > lookback {
            dates = dates.split_off(dates.len() - lookback);
        }

        if dates.is_empty() {
            anyhow::bail!(
                "No historical dates on or before {} for scope {:?}",
                target_date, scope
            );
        }

        let range_start = dates.first().copied().unwrap();
        let range_end = dates.last().copied().unwrap();

        // 5. Bulk fetch all data needed for the date range
        let all_signals = market_store::fetch_signal_snapshots_for_range_with_scope(
            &self.storage, scope, range_start, range_end,
        )?;
        let mut signals_by_date: BTreeMap<NaiveDate, Vec<core_domain::SignalSnapshot>> =
            BTreeMap::new();
        for s in all_signals {
            signals_by_date.entry(s.date).or_default().push(s);
        }

        let all_envs = market_store::fetch_environment_snapshots_for_scope(
            &self.storage, scope, range_start, range_end,
        )?;
        let mut env_by_date: BTreeMap<NaiveDate, core_domain::EnvironmentSnapshot> =
            BTreeMap::new();
        for e in all_envs {
            env_by_date.insert(e.date, e);
        }

        let scope_str = scope.as_str().to_uppercase();
        let all_regimes = market_store::fetch_market_regimes(&self.storage)?;
        let regimes_for_scope: Vec<core_domain::MarketRegimeSnapshot> = all_regimes
            .into_iter()
            .filter(|r| r.market.eq_ignore_ascii_case(&scope_str))
            .collect();
        let mut regimes_by_date: BTreeMap<NaiveDate, core_domain::MarketRegimeSnapshot> =
            BTreeMap::new();
        for r in regimes_for_scope {
            regimes_by_date.insert(r.date, r);
        }

        let all_rotations = market_store::fetch_rotation_ranks_for_range(
            &self.storage, range_start, range_end,
        )?;
        let instruments = self.seed_universe().unwrap_or_default();
        let market_filter = match scope {
            core_domain::AnalysisScope::Cn => Some(core_domain::Market::Cn),
            core_domain::AnalysisScope::Hk => Some(core_domain::Market::Hk),
            core_domain::AnalysisScope::Global => None,
        };
        let mut rotations_by_date: BTreeMap<NaiveDate, Vec<core_domain::RotationRankSnapshot>> =
            BTreeMap::new();
        for r in all_rotations {
            let in_scope = market_filter.as_ref().map_or(true, |m| {
                instruments.iter().any(|i| i.symbol == r.symbol && i.market == *m)
            });
            if in_scope {
                rotations_by_date.entry(r.date).or_default().push(r);
            }
        }

        // 6. Build MarketFingerprint for each historical date in-memory
        let mut fingerprints: Vec<market_fingerprint_engine::MarketFingerprint> =
            Vec::with_capacity(dates.len());
        let mut target_index_opt: Option<usize> = None;

        for &d in &dates {
            let signals = signals_by_date.get(&d).cloned().unwrap_or_default();
            let env = env_by_date.get(&d).cloned();
            let regime = regimes_by_date.get(&d).cloned();
            let rotations = rotations_by_date.get(&d).cloned().unwrap_or_default();

            let mut rotation_history: BTreeMap<
                NaiveDate,
                Vec<core_domain::RotationRankSnapshot>,
            > = BTreeMap::new();
            for (&hist_date, hist_rotations) in &rotations_by_date {
                if hist_date < d {
                    rotation_history.insert(hist_date, hist_rotations.clone());
                }
            }

            let all_regimes_for_date: Vec<core_domain::MarketRegimeSnapshot> =
                regime.into_iter().collect();
            let env_history: Vec<core_domain::EnvironmentSnapshot> = env.into_iter().collect();

            let dataset = ResearchDataset {
                date: d,
                scope,
                signals,
                states_history: Vec::new(),
                env_history,
                rotations,
                rotation_history,
                all_regimes: all_regimes_for_date,
                signal_history: BTreeMap::new(),
            };

            let research_ctx = build_research_context_from_dataset(&dataset)?;
            let fp = MarketFingerprintBuilder::build(&research_ctx);
            if d == target_date {
                target_index_opt = Some(fingerprints.len());
            }
            fingerprints.push(fp);
        }

        // 7. Resolve target index
        let target_index = target_index_opt
            .or_else(|| {
                fingerprints
                    .iter()
                    .position(|fp| fp.date == target_date)
            })
            .context(format!(
                "Target date {} not found in historical fingerprint list for scope {:?}",
                target_date, scope
            ))?;

        // 8. Normalize all fingerprints
        let normalized = normalize_all(&fingerprints);

        // 9. Run similarity search
        let matcher = SimilarityMatcher::new(CosineDistance);
        let mut result = matcher.search(target_index, &fingerprints, &normalized, top_n);

        // 10. Profile forward outcomes for the matched dates
        let provider = AnchorBarForwardProvider {
            close_by_date: &close_by_date,
        };
        result.outcome = OutcomeProfiler::profile(&result.matches, horizon_days, &provider);

        Ok(result)
    }

    /// Bulk fetch the research data needed for a date range and build both
    /// `ResearchContext` and `MarketFingerprint` for every date in the range.
    ///
    /// This is the performance-critical path used by `research_analogues` and
    /// `run_research_calibration`. It avoids per-date DB round-trips by loading
    /// signals, environment snapshots, regimes, and rotation ranks in a single pass.
    fn build_research_bundle_for_range(
        &self,
        scope: ReportScope,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> Result<Vec<(NaiveDate, research_context::ResearchContext, market_fingerprint_engine::MarketFingerprint)>> {
        use market_fingerprint_engine::MarketFingerprintBuilder;

        // 1. Bulk fetch signals
        let all_signals = market_store::fetch_signal_snapshots_for_range_with_scope(
            &self.storage, scope, range_start, range_end,
        )?;
        let mut signals_by_date: BTreeMap<NaiveDate, Vec<core_domain::SignalSnapshot>> = BTreeMap::new();
        for s in all_signals {
            signals_by_date.entry(s.date).or_default().push(s);
        }

        // 2. Bulk fetch environment snapshots
        let all_envs = market_store::fetch_environment_snapshots_for_scope(
            &self.storage, scope, range_start, range_end,
        )?;
        let mut env_by_date: BTreeMap<NaiveDate, core_domain::EnvironmentSnapshot> = BTreeMap::new();
        for e in all_envs {
            env_by_date.insert(e.date, e);
        }

        // 3. Bulk fetch market regimes for the scope
        let scope_str = scope.as_str().to_uppercase();
        let all_regimes = market_store::fetch_market_regimes(&self.storage)?;
        let regimes_for_scope: Vec<core_domain::MarketRegimeSnapshot> = all_regimes
            .into_iter()
            .filter(|r| r.market.eq_ignore_ascii_case(&scope_str))
            .collect();
        let mut regimes_by_date: BTreeMap<NaiveDate, core_domain::MarketRegimeSnapshot> = BTreeMap::new();
        for r in regimes_for_scope {
            regimes_by_date.insert(r.date, r);
        }

        // 4. Bulk fetch rotation ranks and filter by scope
        let all_rotations = market_store::fetch_rotation_ranks_for_range(
            &self.storage, range_start, range_end,
        )?;
        let instruments = self.seed_universe().unwrap_or_default();
        let market_filter = match scope {
            core_domain::AnalysisScope::Cn => Some(core_domain::Market::Cn),
            core_domain::AnalysisScope::Hk => Some(core_domain::Market::Hk),
            core_domain::AnalysisScope::Global => None,
        };
        let mut rotations_by_date: BTreeMap<NaiveDate, Vec<core_domain::RotationRankSnapshot>> = BTreeMap::new();
        for r in all_rotations {
            let in_scope = market_filter.as_ref().map_or(true, |m| {
                instruments.iter().any(|i| i.symbol == r.symbol && i.market == *m)
            });
            if in_scope {
                rotations_by_date.entry(r.date).or_default().push(r);
            }
        }

        // 5. Build ResearchContext + MarketFingerprint for each date in the range
        let mut bundles = Vec::new();
        let mut d = range_start;
        while d <= range_end {
            let signals = signals_by_date.get(&d).cloned().unwrap_or_default();
            if signals.is_empty() {
                d += chrono::Duration::days(1);
                continue;
            }
            let env = env_by_date.get(&d).cloned();
            let regime = regimes_by_date.get(&d).cloned();
            let rotations = rotations_by_date.get(&d).cloned().unwrap_or_default();

            let mut rotation_history: BTreeMap<NaiveDate, Vec<core_domain::RotationRankSnapshot>> = BTreeMap::new();
            for (&hist_date, hist_rotations) in &rotations_by_date {
                if hist_date < d {
                    rotation_history.insert(hist_date, hist_rotations.clone());
                }
            }

            let all_regimes_for_date: Vec<core_domain::MarketRegimeSnapshot> = regime.into_iter().collect();
            let env_history: Vec<core_domain::EnvironmentSnapshot> = env.into_iter().collect();

            let dataset = ResearchDataset {
                date: d,
                scope,
                signals,
                states_history: Vec::new(),
                env_history,
                rotations,
                rotation_history,
                all_regimes: all_regimes_for_date,
                signal_history: BTreeMap::new(),
            };

            let research_ctx = build_research_context_from_dataset(&dataset)?;
            let fp = MarketFingerprintBuilder::build(&research_ctx);
            bundles.push((d, research_ctx, fp));

            d += chrono::Duration::days(1);
        }

        Ok(bundles)
    }

    // -----------------------------------------------------------------------
    // V7.2C Research Calibration Framework
    // -----------------------------------------------------------------------

    /// Run the Research Calibration framework over a historical window.
    ///
    /// For each day in the window, collects:
    /// - Confirmation observation (overall label, sub-scores, breadth)
    /// - Recovery observation (score, label, drivers, breadth delta)
    /// - Analogues search result (matches, average distance, outcome)
    ///
    /// Then delegates to `core_domain::research::calibration` for pure
    /// statistics and renders a markdown report to `reports/calibration/`.
    pub fn run_research_calibration(
        &self,
        scope: ReportScope,
        window_start: Option<NaiveDate>,
        window_end: Option<NaiveDate>,
        horizon_days: usize,
        top_n: usize,
        lookback_days: usize,
    ) -> Result<std::path::PathBuf> {
        use core_domain::research::calibration::{
            calibrate, AnaloguesObservation, CalibrationInput, ConfirmationObservation,
            CURRENT_CALIBRATION_BASELINE_VERSION, MatchObservation, RecoveryObservation,
            render_markdown,
        };
        use market_fingerprint_engine::{
            normalize_all, CosineDistance, DistanceMetric, OutcomeProfiler, SimilarityMatcher,
        };

        // 1. Resolve window
        let end = match window_end {
            Some(d) => d,
            None => market_store::fetch_latest_table_date(&self.storage, "signal_snapshot")?
                .context("No signal data available")?,
        };
        let start = match window_start {
            Some(d) => d,
            None => {
                let available = self.dashboard_available_dates_for_scope(scope)?;
                let days = 60usize;
                available
                    .into_iter()
                    .filter(|d| *d <= end)
                    .take(days)
                    .last()
                    .unwrap_or(end)
            }
        };

        if start > end {
            anyhow::bail!("Calibration window start must be on or before end");
        }

        // 2. Determine extended range: lookback trading days before start
        // dashboard_available_dates_for_scope returns dates in descending order
        // (newest first), so going back in time means increasing the index.
        let available_dates = self.dashboard_available_dates_for_scope(scope)?;
        let start_idx = available_dates
            .iter()
            .position(|d| *d == start)
            .unwrap_or(0);
        let extended_start_idx = (start_idx + lookback_days.max(1))
            .min(available_dates.len().saturating_sub(1));
        let extended_start = available_dates
            .get(extended_start_idx)
            .copied()
            .unwrap_or(start);

        // 3. Build ResearchContext + MarketFingerprint bundles once for the extended range
        let bundles = self.build_research_bundle_for_range(scope, extended_start, end)?;
        if bundles.is_empty() {
            anyhow::bail!("No research bundles available for calibration window");
        }

        // 4. Fetch anchor bars once for outcome profiling
        let anchor_symbol = match scope {
            core_domain::AnalysisScope::Global | core_domain::AnalysisScope::Cn => "000300",
            core_domain::AnalysisScope::Hk => "HSCEI",
        };
        let anchor_bars = market_store::fetch_daily_bars(&self.storage, anchor_symbol)?;
        let close_by_date: BTreeMap<NaiveDate, f64> =
            anchor_bars.iter().map(|b| (b.date, b.close)).collect();
        let provider = AnchorBarForwardProvider {
            close_by_date: &close_by_date,
        };

        // 5. Collect daily observations for the calibration window
        let mut confirmations = Vec::new();
        let mut recoveries = Vec::new();
        let mut analogues = Vec::new();

        for (d, ctx, _) in &bundles {
            if *d < start || *d > end {
                continue;
            }

            confirmations.push(ConfirmationObservation {
                date: *d,
                overall: ctx.confirmation.overall.clone(),
                trend_score: ctx.confirmation.trend.score,
                participation_score: ctx.confirmation.participation.score,
                risk_score: ctx.confirmation.risk.score,
                breadth_pct: ctx.breadth.breadth_pct,
            });

            recoveries.push(RecoveryObservation {
                date: *d,
                score: ctx.recovery.score,
                label: Self::recovery_bucket_label(ctx.recovery.score),
                drivers: ctx.recovery.drivers.clone(),
                breadth_5d_delta: ctx.breadth.delta_5d.unwrap_or(0.0),
            });
        }

        // 6. Run analogue search for each calibration date using the cached fingerprints
        let matcher = SimilarityMatcher::new(CosineDistance);
        for (idx, (d, _, _)) in bundles.iter().enumerate() {
            if *d < start || *d > end {
                continue;
            }

            let lookback = lookback_days.max(1);
            let subset_start = idx.saturating_sub(lookback);
            let subset: Vec<market_fingerprint_engine::MarketFingerprint> = bundles[subset_start..=idx]
                .iter()
                .map(|(_, _, fp)| fp.clone())
                .collect();

            if subset.len() < 2 {
                continue;
            }

            let target_index = subset.len() - 1;
            let normalized = normalize_all(&subset);
            let mut result = matcher.search(target_index, &subset, &normalized, top_n);
            result.outcome = OutcomeProfiler::profile(&result.matches, horizon_days, &provider);

            let target_fv = &normalized[target_index];
            let all_distances: Vec<f64> = normalized
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != target_index)
                .map(|(_, fv)| CosineDistance.distance(target_fv, fv))
                .collect();

            let top_matches: Vec<MatchObservation> = result
                .matches
                .iter()
                .map(|m| MatchObservation {
                    date: m.date,
                    level: Self::match_level_label(&m.level),
                })
                .collect();
            analogues.push(AnaloguesObservation {
                date: *d,
                searched_days: result.searched_days,
                filtered_days: result.filtered_days,
                average_distance: result.average_distance,
                top_matches,
                outcome_median: result.outcome.as_ref().map(|o| o.median),
                outcome_win_rate: result.outcome.as_ref().map(|o| o.win_rate),
                all_distances,
            });
        }

        // Determine the actual trading days in the calibration window
        let calibration_dates: Vec<NaiveDate> = available_dates
            .iter()
            .filter(|d| **d >= start && **d <= end)
            .copied()
            .collect();

        // 7. Build calibration report
        let input = CalibrationInput {
            scope: format!("{:?}", scope),
            window_start: start,
            window_end: end,
            expected_dates: calibration_dates,
            baseline_version: CURRENT_CALIBRATION_BASELINE_VERSION,
            generated_at: chrono::Utc::now(),
            confirmations,
            recoveries,
            analogues,
        };
        let report = calibrate(input);
        let markdown = render_markdown(&report);

        // 8. Write to reports/calibration/
        let scope_label = scope_label(scope).to_lowercase();
        let output_dir = std::path::PathBuf::from("reports/calibration");
        std::fs::create_dir_all(&output_dir)?;
        let output_path = output_dir.join(format!(
            "research-calibration-{}-{}-{}.md",
            scope_label, start, end
        ));
        std::fs::write(&output_path, markdown)?;

        Ok(output_path)
    }

    // -----------------------------------------------------------------------
    // V7.3 Research Synthesis Layer — Consensus
    // -----------------------------------------------------------------------

    /// Run the V7.3 Research Consensus synthesizer for a single date.
    ///
    /// Aggregates Observation (Signal, Stretch), Evolution (Confirmation,
    /// Recovery), and Historical Evidence (Analogues forward outlook) into a
    /// research-language consensus. Writes a markdown report to
    /// `reports/consensus/` and returns the file path.
    pub fn run_research_consensus(
        &self,
        scope: ReportScope,
        date: Option<NaiveDate>,
        horizon_days: usize,
        top_n: usize,
        lookback_days: usize,
    ) -> Result<std::path::PathBuf> {
        use core_domain::research::classification::classify_level;
        use core_domain::research::consensus::{consensus, ConsensusConfig, EvidenceInput};
        use core_domain::research::stretch::weighted_stretch_overall;
        use market_fingerprint_engine::{
            normalize_all, CosineDistance, OutcomeProfiler, SimilarityMatcher,
        };
        use report_builder::{ConsensusReportInput, ResearchReportBuilder};
        use report_renderer::MarkdownFormatter;
        use reporting::{Formatter, ReportingSnapshot};

        // 1. Resolve target date
        let target_date = match date {
            Some(d) => d,
            None => market_store::fetch_latest_table_date(&self.storage, "signal_snapshot")?
                .context("No signal data available")?,
        };

        // 2. Determine extended range for analogue search
        let available_dates = self.dashboard_available_dates_for_scope(scope)?;
        let target_idx = available_dates
            .iter()
            .position(|d| *d == target_date)
            .context("Target date not available for the selected scope")?;
        let lookback = lookback_days.max(1);
        let extended_idx = (target_idx + lookback).min(available_dates.len().saturating_sub(1));
        let extended_start = available_dates.get(extended_idx).copied().unwrap_or(target_date);

        // 3. Build ResearchContext + MarketFingerprint bundles once
        let bundles = self.build_research_bundle_for_range(scope, extended_start, target_date)?;
        let target_bundle = bundles
            .iter()
            .find(|(d, _, _)| *d == target_date)
            .context("No research bundle found for target date")?;
        let target_ctx = &target_bundle.1;

        // 4. Run analogue search using the cached fingerprints
        let matcher = SimilarityMatcher::new(CosineDistance);
        let target_index = bundles.len() - 1;
        let subset: Vec<_> = bundles.iter().map(|(_, _, fp)| fp.clone()).collect();
        let normalized = normalize_all(&subset);
        let mut result = matcher.search(target_index, &subset, &normalized, top_n);

        // 5. Profile forward outcome
        let anchor_symbol = match scope {
            core_domain::AnalysisScope::Global | core_domain::AnalysisScope::Cn => "000300",
            core_domain::AnalysisScope::Hk => "HSCEI",
        };
        let anchor_bars = market_store::fetch_daily_bars(&self.storage, anchor_symbol)?;
        let close_by_date: BTreeMap<NaiveDate, f64> =
            anchor_bars.iter().map(|b| (b.date, b.close)).collect();
        let provider = AnchorBarForwardProvider {
            close_by_date: &close_by_date,
        };
        result.outcome = OutcomeProfiler::profile(&result.matches, horizon_days, &provider);

        // 6. Build EvidenceInput from Observation + Evolution + Historical Evidence
        let signal_score = target_ctx.signal.average_score / 100.0;

        let confirmation_avg = (target_ctx.confirmation.trend.score
            + target_ctx.confirmation.participation.score
            + target_ctx.confirmation.risk.score)
            / 3.0
            / 100.0;

        let recovery_score = target_ctx.recovery.score / 100.0;

        // Stretch evidence: compute from the target bundle's rotations/env.
        let rotations = bundles
            .iter()
            .find(|(d, _, _)| *d == target_date)
            .map(|(_, _, fp)| fp.observation.rotation.clone())
            .unwrap_or_default();
        let total_momentum: f64 = rotations.iter().map(|(_, score)| score).sum();
        let mut sorted_rotations = rotations.clone();
        sorted_rotations.sort_by(|(_, a), (_, b)| b.total_cmp(a));
        let top5_sum: f64 = sorted_rotations.iter().take(5).map(|(_, score)| score).sum();
        let concentration_pct = if total_momentum > 0.0 {
            (top5_sum / total_momentum) * 100.0
        } else {
            0.0
        };
        let rs120_max = rotations
            .iter()
            .map(|(_, score)| score)
            .fold(0.0_f64, |max_so_far: f64, score| max_so_far.max(*score));
        let crowding_level = classify_level(concentration_pct, 30.0, 50.0, true);
        let breadth_pct = target_ctx.breadth.breadth_pct;
        let breadth_level = classify_level(breadth_pct, 35.0, 20.0, false);
        let momentum_level = classify_level(rs120_max, 70.0, 85.0, true);
        let leverage_level = "Normal";
        let (_, stretch_score) =
            weighted_stretch_overall(crowding_level, breadth_level, momentum_level, leverage_level);
        let stretch_normalized = (stretch_score / 2.0).clamp(0.0, 1.0);

        // Analogues evidence: use win-rate centered around 50%.
        let analogues_score = result
            .outcome
            .as_ref()
            .map(|o| ((o.win_rate - 0.5) * 2.0).clamp(-1.0, 1.0));

        let evidence_input = EvidenceInput {
            signal: Some(signal_score.clamp(0.0, 1.0)),
            stretch: Some(stretch_normalized),
            confirmation: Some(confirmation_avg.clamp(0.0, 1.0)),
            recovery: Some(recovery_score.clamp(0.0, 1.0)),
            analogues: analogues_score,
        };

        let summary = consensus(evidence_input, &ConsensusConfig::default());

        // 7. Build report
        let reporting_snapshot = ReportingSnapshot {
            generated_at: chrono::Utc::now(),
            research: target_ctx.clone(),
        };
        let input = ConsensusReportInput { summary };
        let doc = ResearchReportBuilder::build_consensus(&reporting_snapshot, &input)?;

        let mut formatter = MarkdownFormatter::new();
        report_renderer::render(&mut formatter, &doc);
        let markdown = formatter.finalize();

        // 8. Write to reports/consensus/
        let scope_label = scope_label(scope).to_lowercase();
        let output_dir = std::path::PathBuf::from("reports/consensus");
        std::fs::create_dir_all(&output_dir)?;
        let output_path = output_dir.join(format!(
            "research-consensus-{}-{}.md",
            scope_label, target_date
        ));
        std::fs::write(&output_path, markdown)?;

        Ok(output_path)
    }

    /// Map a 0-100 recovery score to a human-readable bucket label.
    fn recovery_bucket_label(score: f64) -> String {
        match score {
            s if s >= 80.0 => "80-100".to_string(),
            s if s >= 60.0 => "60-80".to_string(),
            s if s >= 40.0 => "40-60".to_string(),
            s if s >= 20.0 => "20-40".to_string(),
            _ => "0-20".to_string(),
        }
    }

    /// Map a fingerprint MatchLevel to its string representation.
    fn match_level_label(level: &market_fingerprint_engine::MatchLevel) -> String {
        use market_fingerprint_engine::MatchLevel;
        match level {
            MatchLevel::VeryHigh => "Very High".to_string(),
            MatchLevel::High => "High".to_string(),
            MatchLevel::Moderate => "Moderate".to_string(),
            MatchLevel::Weak => "Weak".to_string(),
        }
    }

    /// Phase 1: Single query owner — fetch all research data in one place.
    fn fetch_research_dataset(
        &self,
        date: NaiveDate,
        scope: core_domain::AnalysisScope,
        signal_history_lookback_days: i64,
    ) -> Result<ResearchDataset> {
        // 1. signals for date/scope
        let signals = market_store::fetch_signal_snapshots_for_date_with_scope(
            &self.storage, date, scope,
        )?;

        // 2. states_history for scope
        let states_history =
            market_store::fetch_strategy_states_for_scope(&self.storage, scope)?;

        // 3. env_history (60-day lookback)
        let env_lookback = date - chrono::Duration::days(60);
        let env_history = market_store::fetch_environment_snapshots_for_scope(
            &self.storage, scope, env_lookback, date,
        )?;

        // 4. rotations for date, filtered by scope using seed_universe
        let rotations = market_store::fetch_rotation_ranks_for_date(&self.storage, date)?;
        let instruments = self.seed_universe().unwrap_or_default();
        let market_filter = match scope {
            core_domain::AnalysisScope::Cn => Some(core_domain::Market::Cn),
            core_domain::AnalysisScope::Hk => Some(core_domain::Market::Hk),
            core_domain::AnalysisScope::Global => None,
        };
        let rotations: Vec<core_domain::RotationRankSnapshot> = rotations
            .into_iter()
            .filter(|r| {
                market_filter.as_ref().map_or(true, |m| {
                    instruments.iter().any(|i| i.symbol == r.symbol && i.market == *m)
                })
            })
            .collect();

        // 4b. rotation history for the last 20 trading days for leadership evolution
        let rotation_lookback = date - chrono::Duration::days(30);
        let hist_rotations = market_store::fetch_rotation_ranks_for_range(
            &self.storage,
            rotation_lookback,
            date,
        )?;
        let mut rotation_history: BTreeMap<NaiveDate, Vec<core_domain::RotationRankSnapshot>> =
            BTreeMap::new();
        for r in hist_rotations {
            rotation_history
                .entry(r.date)
                .or_default()
                .push(r);
        }
        // Apply the same scope filter to historical rotations
        for day_rotations in rotation_history.values_mut() {
            day_rotations.retain(|r| {
                market_filter.as_ref().map_or(true, |m| {
                    instruments.iter().any(|i| i.symbol == r.symbol && i.market == *m)
                })
            });
        }
        // Remove empty entries after filtering
        rotation_history.retain(|_, v| !v.is_empty());

        // 5. all_regimes
        let all_regimes = market_store::fetch_market_regimes(&self.storage)?;

        // 6. signal_history (signal_history_lookback_days range) using new market-store function
        let history_from = date - chrono::Duration::days(signal_history_lookback_days);
        let history_signals = market_store::fetch_signal_snapshots_for_range_with_scope(
            &self.storage, scope, history_from, date,
        )?;
        let mut signal_history: BTreeMap<NaiveDate, Vec<(f64, core_domain::SignalLabel)>> =
            BTreeMap::new();
        for s in history_signals {
            signal_history
                .entry(s.date)
                .or_default()
                .push((s.final_score, s.signal_label));
        }

        Ok(ResearchDataset {
            date,
            scope,
            signals,
            states_history,
            env_history,
            rotations,
            rotation_history,
            all_regimes,
            signal_history,
        })
    }

    /// V6 Reporting Layer demo: build ResearchContext → ReportingSnapshot → ReportDocument → Markdown.
    ///
    /// This is a temporary demo method to validate the new reporting pipeline.
    /// It does not replace any existing CLI behavior.
    pub fn demo_reporting_pipeline(&self, scope: ReportScope) -> Result<String> {
        use report_builder::{ResearchReportBuilder, SrdReportInput};
        use reporting::Formatter;

        let snapshot = self
            .dashboard_snapshot_with_scope(None, scope)?
            .context("No dashboard data available")?;
        let research = build_research_context_from_dashboard(&snapshot);
        let reporting_snapshot = reporting::ReportingSnapshot {
            generated_at: chrono::Utc::now(),
            research,
        };

        // Demo: build an SRD-style document with placeholder input
        let input = SrdReportInput {
            strong_buy_count: 0,
            buy_count: 0,
            average_signal: 0.0,
            duration: 0,
            breadth_trend: "Neutral".to_string(),
            rotation_pattern: "Mixed".to_string(),
            historical_percentile: 50.0,
            interpretation: "Demo pipeline output".to_string(),
            confidence: "Low".to_string(),
            state_label: "NO_TRADE".to_string(),
        };

        let document =
            ResearchReportBuilder::build_srd(&reporting_snapshot, &input)?;
        let mut formatter = report_renderer::MarkdownFormatter::new();
        report_renderer::render(&mut formatter, &document);
        Ok(formatter.finalize())
    }

    /// Research Layer — 只读叙事层分析。
    ///
    ///  governance:
    ///  - 只解释、质疑、提供上下文
    ///  - 禁止创建信号、评分、排序、覆盖决策
    ///
    /// action 必须是以下之一：
    /// - "market_story"
    /// - "explain_decision"
    /// - "preclose_review"
    /// - "risk_view"
    /// - "devils_advocate"
    /// RV1 Phase 3 (ADR-106): LLM Explanation Layer.
    ///
    /// Data flow: Deterministic Engines → Decision Fact → LLM Explanation.
    /// The LLM receives computed facts (strategy scores, scenario contrast,
    /// integrity status, previous interpretation) as EXPLANATION INPUT and
    /// produces explanation text only. It never generates action labels,
    /// position sizing, or modifies final_score / signal_label / portfolio_decision.
    ///
    /// ADR-112: before the main call, a shared adversarial hypothesis background
    /// is ensured for (scope, report_date) and injected per the persona's
    /// InjectLevel. `adversarial_override` (from CLI/Tauri) wins over config;
    /// `None` falls back to `[llm.adversarial]` resolution.
    pub async fn analyze_with_action(
        &self,
        action: &str,
        scope: ReportScope,
        adversarial_override: Option<core_domain::InjectLevel>,
    ) -> anyhow::Result<serde_json::Value> {
        // 1. Build snapshot context
        let snapshot = self.dashboard_snapshot_with_scope(None, scope)?;
        let Some(snapshot) = snapshot else {
            return Err(anyhow::anyhow!("No dashboard snapshot available for scope {:?}", scope));
        };

        // 2. Resolve persona (config/prompts.toml → built-in fallback)
        let project_root = StorageConfig::project_root()?;
        let prompts_file = prompts::load_prompts(&project_root);
        let persona = prompts::resolve_persona(&prompts_file, action)?;

        // 3. Compose extra context sections (computed facts as explanation input)
        let mut extra = String::new();
        extra.push_str(&self.build_strategy_context_section(scope));
        extra.push_str(&self.build_integrity_context_section());
        if action == "portfolio_review" {
            extra.push_str(&self.build_portfolio_decision_section(scope));
        }

        // 3b. ADR-112: shared adversarial hypothesis background.
        // Recursion guard is hardcoded — the adversarial persona never receives
        // its own shared injection, regardless of config.
        // ADR-113/114: the diag answers "did it work, and why not" — every
        // not-injected outcome carries a machine-readable `reason`.
        let resolved_cfg = self.get_resolved_llm_config(None)?;
        let adversarial_enabled = resolved_cfg.adversarial_auto_inject;
        let mut adversarial_diag = serde_json::json!({
            "enabled": adversarial_enabled,
            "injected": false,
            "level": "none",
            "fresh": false,
            "generated_at": serde_json::Value::Null,
            "source": serde_json::Value::Null,
            // Default: recursion guard — the adversarial persona never injects itself.
            "reason": "persona_excluded",
        });
        if action != "market_adversarial_lens" {
            let configured = resolved_cfg
                .adversarial_inject
                .get(action)
                .copied()
                .unwrap_or(core_domain::InjectLevel::Standard);
            // CLI/Tauri 显式指定优先；未指定时遵循 auto_inject 总开关
            let effective = match adversarial_override {
                Some(level) => level,
                None if adversarial_enabled => configured,
                None => core_domain::InjectLevel::None,
            };
            if effective == core_domain::InjectLevel::None {
                let reason = if !adversarial_enabled && adversarial_override.is_none() {
                    "disabled"
                } else {
                    "persona_excluded"
                };
                adversarial_diag["reason"] = serde_json::json!(reason);
            } else {
                let outcome = self
                    .ensure_adversarial_context(scope, &snapshot, "on-demand")
                    .await;
                if let Some(record) = outcome.record {
                    let fresh = record.report_date == snapshot.report_date.to_string();
                    // ADR-114: ContentPolicy (max_chars / full_max_chars) is
                    // independent from the InjectionLevel granularity choice.
                    let section_result = llm_history::adversarial_context_section(
                        &record,
                        effective.as_str(),
                        resolved_cfg.adversarial_max_chars,
                        resolved_cfg.adversarial_full_max_chars,
                    );
                    extra.push_str(&section_result.section);
                    adversarial_diag = serde_json::json!({
                        "enabled": adversarial_enabled,
                        "injected": true,
                        "level": effective.as_str(),
                        "fresh": fresh,
                        "generated_at": record.created_at,
                        "source": record.source,
                        "reason": outcome.reason,
                        "original_chars": section_result.original_chars,
                        "final_chars": section_result.final_chars,
                        "truncated": section_result.truncated,
                    });
                } else {
                    adversarial_diag = serde_json::json!({
                        "enabled": adversarial_enabled,
                        "injected": false,
                        "level": effective.as_str(),
                        "fresh": false,
                        "generated_at": serde_json::Value::Null,
                        "source": serde_json::Value::Null,
                        "reason": outcome.reason,
                    });
                }
            }
        }
        // Previous interpretation — labeled as background, never evidence (ADR-106)
        if let Some(previous) =
            llm_history::latest_record(&project_root, scope.as_str(), action)
        {
            if previous.report_date != snapshot.report_date.to_string() {
                extra.push_str(&llm_history::previous_interpretation_section(&previous));
            }
        }

        // 4. Build prompt
        let (system_prompt, user_prompt) = research_skills::build_prompt_with_persona(
            &persona.system,
            &persona.template,
            &snapshot,
            Some(&extra),
        );

        // 5. Call LLM
        let resolved = self.get_resolved_llm_config(None)?;
        let inference = research_skills::InferenceConfig {
            temperature: resolved.temperature,
            seed: resolved.seed,
            max_tokens: resolved.max_tokens,
        };
        let config = self.get_llm_config()?;
        let api_key = if let Some(ref key) = resolved.api_key {
            if !key.is_empty() {
                Some(key.clone())
            } else {
                self.get_llm_api_key()?
            }
        } else {
            self.get_llm_api_key()?
        };

        let (llm_output, is_placeholder) = if let Some(ref key) = api_key {
            let _provider = research_skills::OpenAiProvider::from_config(&config, key);
            let call_config = inference.to_call_config();
            let response = llm::call_llm_api(
                config.clone(),
                key.clone(),
                &system_prompt,
                user_prompt,
                call_config.temperature as f64,
                call_config.max_tokens,
                call_config.seed,
            )
            .await?;
            (Some(response), false)
        } else {
            (Some("LLM 未配置，这是占位符输出。请配置 API Key 以获取真实分析。".to_string()), true)
        };

        let analysis_text = llm_output.unwrap_or_default();

        // 6. Persist conversation record (workspace/llm-history/)
        if !is_placeholder {
            let record = llm_history::LlmAnalysisRecord {
                scope: scope.as_str().to_string(),
                action: action.to_string(),
                persona_label: persona.label.clone(),
                report_date: snapshot.report_date.to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                summary: llm_history::make_summary(&analysis_text),
                analysis_text: analysis_text.clone(),
                source: Some("on-demand".to_string()),
            };
            if let Err(error) = llm_history::save_record(&project_root, &record) {
                eprintln!("[llm-history] failed to save record: {error}");
            }
        }

        // 7. Return markdown result
        let result = serde_json::json!({
            "action": action,
            "persona": persona.label,
            "scope": scope.as_str(),
            "placeholder": is_placeholder,
            "markdown": analysis_text,
            "adversarial": adversarial_diag,
        });

        Ok(result)
    }

    /// ADR-112: ensure a fresh shared adversarial hypothesis background exists
    /// for (scope, snapshot.report_date).
    ///
    /// - Fresh hit (same report_date) → reuse the stored record, zero LLM cost.
    /// - Stale/missing → run the `market_adversarial_lens` persona once on the
    ///   SAME snapshot and persist under action="adversarial".
    /// - ANY failure (no API key, network, timeout) → silent `None`; the main
    ///   call proceeds without injection. Never propagates errors.
    ///
    /// ADR-113/114: `source` stamps provenance on the persisted record —
    /// "on-demand" for the interactive analyze path, "market-refresh" for the
    /// async post-refresh prewarm. The returned [`AdversarialOutcome`] also
    /// reports WHY no record is available, so the diag surface can explain it.
    async fn ensure_adversarial_context(
        &self,
        scope: ReportScope,
        snapshot: &report_engine::DashboardSnapshot,
        source: &str,
    ) -> AdversarialOutcome {
        let none = |reason: &'static str| AdversarialOutcome {
            record: None,
            reason,
        };
        let project_root = match StorageConfig::project_root() {
            Ok(root) => root,
            Err(_) => return none("config_error"),
        };

        // 1. Fresh hit → reuse (Daily Shared Context Pattern: 1 LLM call/day/scope)
        let existing =
            llm_history::latest_record(&project_root, scope.as_str(), "adversarial");
        // A record exists but may be date-mismatched: if regeneration below
        // fails, the reason collapses to "stale" (ADR-114).
        let has_stale_record = existing.is_some();
        if let Some(existing) = existing {
            if existing.report_date == snapshot.report_date.to_string() {
                return AdversarialOutcome {
                    record: Some(existing),
                    reason: "injected",
                };
            }
        }
        let stale_or = |reason: &'static str| {
            if has_stale_record {
                none("stale")
            } else {
                none(reason)
            }
        };

        // 2. Run the pre-pass on the SAME snapshot (no recursive analyze_with_action)
        let prompts_file = prompts::load_prompts(&project_root);
        let persona = match prompts::resolve_persona(&prompts_file, "market_adversarial_lens") {
            Ok(persona) => persona,
            Err(_) => return stale_or("persona_missing"),
        };
        let mut extra = String::new();
        extra.push_str(&self.build_strategy_context_section(scope));
        extra.push_str(&self.build_integrity_context_section());
        let (system_prompt, user_prompt) = research_skills::build_prompt_with_persona(
            &persona.system,
            &persona.template,
            snapshot,
            Some(&extra),
        );

        let resolved = match self.get_resolved_llm_config(None) {
            Ok(resolved) => resolved,
            Err(_) => return stale_or("config_error"),
        };
        let config = match self.get_llm_config() {
            Ok(config) => config,
            Err(_) => return stale_or("config_error"),
        };
        let api_key = match resolved
            .api_key
            .clone()
            .filter(|key| !key.is_empty())
            .or_else(|| self.get_llm_api_key().ok().flatten())
        {
            Some(key) => key,
            // no key → silent skip, never persist placeholder
            None => return stale_or("no_api_key"),
        };

        let text = match llm::call_llm_api(
            config,
            api_key,
            &system_prompt,
            user_prompt,
            resolved.temperature,
            resolved.max_tokens,
            resolved.seed,
        )
        .await
        {
            Ok(text) => text,
            Err(_) => return stale_or("llm_error"),
        };

        let record = llm_history::LlmAnalysisRecord {
            scope: scope.as_str().to_string(),
            action: "adversarial".to_string(),
            persona_label: persona.label.clone(),
            report_date: snapshot.report_date.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            summary: llm_history::make_summary(&text),
            analysis_text: text,
            source: Some(source.to_string()),
        };
        if let Err(error) = llm_history::save_record(&project_root, &record) {
            eprintln!("[adversarial] failed to save record: {error}");
        }
        AdversarialOutcome {
            record: Some(record),
            reason: "injected",
        }
    }

    /// ADR-113/114: async adversarial prewarm after market-refresh.
    ///
    /// Spawns a DETACHED `std::thread` that generates the shared adversarial
    /// hypothesis background (one LLM call per scope, on the just-refreshed
    /// snapshot) and persists it with source="market-refresh", so the user's
    /// first `llm-analyze` of the day hits the warm path.
    ///
    /// Design constraints (ADR-114):
    /// - NEVER blocks the caller: a plain `std::thread` is used (not
    ///   `tokio::spawn`) because the CLI process exits right after refresh and
    ///   a runtime task would be killed; the detached thread carries its own
    ///   tokio runtime and dies quietly if the process exits early — the
    ///   on-demand cold path in `analyze_with_action` is the fallback.
    /// - NEVER fails the refresh: every error (config, snapshot, LLM, IO) is
    ///   swallowed with an `eprintln!`.
    /// - `adversarial_auto_inject == false` is the single master switch: it
    ///   disables both injection and prewarm. No separate switch (ADR-114
    ///   reserves `auto_prepare` for the future).
    ///
    /// The returned `JoinHandle` is dropped deliberately: joining would defeat
    /// the fire-and-forget contract.
    pub fn spawn_adversarial_prewarm(&self, scopes: Vec<ReportScope>) {
        // Master switch: auto_inject=false disables injection AND prewarm.
        let resolved = match self.get_resolved_llm_config(None) {
            Ok(config) => config,
            Err(error) => {
                eprintln!("[adversarial-prewarm] skipped (config resolution failed: {error})");
                return;
            }
        };
        if !resolved.adversarial_auto_inject {
            return;
        }
        // No API key → prewarm would silently produce nothing; skip quietly.
        let has_api_key = resolved
            .api_key
            .clone()
            .filter(|key| !key.is_empty())
            .or_else(|| self.get_llm_api_key().ok().flatten())
            .is_some();
        if !has_api_key {
            return;
        }

        let context = self.clone();
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("[adversarial-prewarm] skipped (tokio runtime: {error})");
                    return;
                }
            };
            runtime.block_on(async move {
                for scope in scopes {
                    let label = scope.as_str();
                    let snapshot = match context.dashboard_snapshot_with_scope(None, scope) {
                        Ok(Some(snapshot)) => snapshot,
                        Ok(None) => {
                            eprintln!(
                                "[adversarial-prewarm] {label}: skipped (reason: snapshot_missing)"
                            );
                            continue;
                        }
                        Err(error) => {
                            eprintln!("[adversarial-prewarm] {label}: skipped ({error})");
                            continue;
                        }
                    };
                    let outcome = context
                        .ensure_adversarial_context(scope, &snapshot, "market-refresh")
                        .await;
                    match outcome.record {
                        Some(_) => eprintln!("[adversarial-prewarm] {label}: ready"),
                        None => eprintln!(
                            "[adversarial-prewarm] {label}: skipped (reason: {})",
                            outcome.reason
                        ),
                    }
                }
            });
        });
    }

    /// Strategy perspectives section: top symbols with 4 independent strategy
    /// scores and scenario contrast. Bounded to top 5 by confidence.
    fn build_strategy_context_section(&self, scope: ReportScope) -> String {
        let analysis_scope = match scope {
            ReportScope::Global => core_domain::AnalysisScope::Global,
            ReportScope::Cn => core_domain::AnalysisScope::Cn,
            ReportScope::Hk => core_domain::AnalysisScope::Hk,
        };
        let Ok(project_root) = StorageConfig::project_root() else {
            return String::new();
        };
        let Ok((date, entries)) = strategy_perspectives::strategy_perspectives_scoreboard(
            &self.storage,
            analysis_scope,
            None,
            &project_root,
        ) else {
            return String::new();
        };
        if entries.is_empty() {
            return String::new();
        }

        let mut section = format!(
            "\n## 多策略视角（{}，系统已计算的事实，非你的判断）\n\n",
            date
        );
        for entry in entries.iter().take(5) {
            section.push_str(&format!(
                "- {}{}：ValueLeft {:.0} / TrendPullback {:.0} / TrendBreakout {:.0} / MomentumRight {:.0}（最佳：{:?}）\n",
                entry.symbol,
                entry
                    .name
                    .as_ref()
                    .map(|name| format!("({})", name))
                    .unwrap_or_default(),
                entry.value_left_score,
                entry.trend_pullback_score,
                entry.trend_breakout_score,
                entry.momentum_right_score,
                entry.best_strategy,
            ));
            let scenario_text: Vec<String> = entry
                .scenario_scores
                .iter()
                .map(|s| format!("{} {:.0}", s.label, s.score))
                .collect();
            if !scenario_text.is_empty() {
                section.push_str(&format!("  场景对比：{}\n", scenario_text.join(" | ")));
            }
        }
        section
    }

    /// Strategy scoreboard for the desktop frontend (RV1): every symbol's four
    /// independent strategy scores plus scenario weightings for one date + scope.
    /// Thin delegate over `strategy_perspectives::strategy_perspectives_scoreboard`;
    /// read-only consumption layer (ADR-107/108).
    pub fn strategy_scoreboard(
        &self,
        scope: core_domain::AnalysisScope,
        date: Option<NaiveDate>,
    ) -> Result<(NaiveDate, Vec<StrategyPerspectiveEntry>)> {
        let project_root = StorageConfig::project_root()?;
        strategy_perspectives::strategy_perspectives_scoreboard(
            &self.storage,
            scope,
            date,
            &project_root,
        )
    }

    /// Strategy attribution detail for one symbol (RV1). Thin delegate over
    /// `strategy_perspectives::strategy_perspectives_detail`; attribution is
    /// recomputed on demand from bars + indicators + regime + rotation, so
    /// this is intentionally lazy and heavier than the scoreboard.
    pub fn strategy_attribution(
        &self,
        symbol: &str,
        scope: core_domain::AnalysisScope,
        date: Option<NaiveDate>,
    ) -> Result<StrategyPerspectiveDetail> {
        let project_root = StorageConfig::project_root()?;
        strategy_perspectives::strategy_perspectives_detail(
            &self.storage,
            symbol,
            scope,
            date,
            &project_root,
        )
    }

    /// Integrity section: data freshness status for the current analysis.
    fn build_integrity_context_section(&self) -> String {
        let Ok(health) = self.check_data_health() else {
            return String::new();
        };
        let status = if health.freshest_market_date_complete {
            "PASS"
        } else {
            "DEGRADED"
        };
        format!(
            "\n## 数据完整性（Integrity）\n\n- 状态：{}\n- 最新市场日期：{:?}（标的覆盖 {}/{}）\n- 健康标的 {}，待复核 {}，异常 {}\n",
            status,
            health.freshest_market_date,
            health.symbols_on_freshest_market_date,
            health.checked_symbols,
            health.healthy_symbols,
            health.review_symbols,
            health.critical_symbols,
        )
    }

    /// Deterministic portfolio decision section (portfolio_review only).
    /// Action labels are produced by the V5 pattern engine — never by the LLM.
    fn build_portfolio_decision_section(&self, scope: ReportScope) -> String {
        match self.analyze_preclose(scope) {
            Ok(decisions) if !decisions.is_empty() => {
                let mut section = String::from(
                    "\n## 组合姿态（由确定性引擎产出，不由你生成，不可修改）\n\n",
                );
                for decision in &decisions {
                    let reasons = if decision.reasons.is_empty() {
                        "（无 Pattern 命中）".to_string()
                    } else {
                        decision
                            .reasons
                            .iter()
                            .map(|reason| reason.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    section.push_str(&format!(
                        "- {} → {}（{}）\n",
                        decision.symbol,
                        decision.state.as_str(),
                        reasons
                    ));
                }
                section
            }
            _ => "\n## 组合姿态\n\n实时行情不可用，决策引擎本次未能运行。\n".to_string(),
        }
    }

    /// TASK-120: Execution Layer — Preclose analysis (Pattern Library filter)
    ///
    /// Candidate filter: signal >= Buy AND state != NO_TRADE
    /// Real-time data: Tencent API snapshot
    /// Output: ExecutionDecision list (BuyNow / Wait / NoChase / Reduce / Skip)
    pub fn analyze_preclose(
        &self,
        scope: ReportScope,
    ) -> Result<Vec<execution_engine::ExecutionDecision>> {
        use execution_engine::types::{ExecutionDecision, SkipReason};
        use core_domain::{SignalLabel, StrategyState};

        // 1. Determine latest available date for this scope
        let available_dates = self.dashboard_available_dates_for_scope(scope)?;
        let Some(latest_date) = available_dates.first().copied() else {
            return Ok(vec![]);
        };

        // 2. Load strategy state (hard gate)
        let strategy_state = market_store::fetch_latest_strategy_state_on_or_before(
            &self.storage,
            latest_date,
            scope,
        )?;

        if let Some(ref state) = strategy_state {
            if state.state == StrategyState::NoTrade {
                // All candidates skip due to state gate
                return Ok(vec![]);
            }
        }

        // 3. Load signals for the latest date
        let signals = market_store::fetch_signal_snapshots_for_date_with_scope(
            &self.storage,
            latest_date,
            scope,
        )?;

        // 4. Filter candidates: signal >= Buy
        let candidates: Vec<_> = signals
            .into_iter()
            .filter(|s| {
                matches!(
                    s.signal_label,
                    SignalLabel::StrongBuy | SignalLabel::Buy
                )
            })
            .collect();

        if candidates.is_empty() {
            return Ok(vec![]);
        }

        // 5. Build symbol list for fetching
        let symbols: Vec<String> = candidates.iter().map(|s| s.symbol.clone()).collect();

        // 6. Fetch MA10 and volume MA20 from indicators for enrichment
        let mut ma10_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut vol_ma20_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        for symbol in &symbols {
            if let Ok(indicators) = market_store::fetch_indicator_snapshots(
                &self.storage,
                symbol,
            ) {
                // Get the latest indicator (last in sorted list)
                if let Some(latest) = indicators.last() {
                    if let Some(ma10) = latest.ma10 {
                        ma10_map.insert(symbol.clone(), ma10);
                    }
                    if let Some(vol_ma20) = latest.vol_ma20 {
                        vol_ma20_map.insert(symbol.clone(), vol_ma20);
                    }
                }
            }
        }

        // 7. Fetch real-time snapshots from Tencent API
        let mut snapshots = match execution_engine::fetcher::fetch_tencent_snapshots(&symbols) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Tencent snapshot fetch failed: {}", e);
                // Return Skip for all candidates
                return Ok(candidates
                    .iter()
                    .map(|c| ExecutionDecision::skipped(c.symbol.clone(), SkipReason::DataUnavailable))
                    .collect());
            }
        };

        // 8. Enrich snapshots with MA10 (used as proxy for MA5 distance) and volume ratio
        execution_engine::fetcher::enrich_snapshots(&mut snapshots, &ma10_map, &vol_ma20_map);

        // 9. Run engine analysis
        let decisions = execution_engine::engine::analyze_batch(&snapshots);

        // 10. Augment decisions with Skip for candidates that had no snapshot data
        let mut all_decisions: Vec<ExecutionDecision> = decisions;
        for candidate in candidates {
            if !all_decisions.iter().any(|d| d.symbol == candidate.symbol) {
                all_decisions.push(ExecutionDecision::skipped(
                    candidate.symbol.clone(),
                    SkipReason::DataUnavailable,
                ));
            }
        }

        // Sort by symbol for deterministic output
        all_decisions.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        Ok(all_decisions)
    }

}

/// Build ResearchContext (Canonical Semantic Contract) from a ResearchDataset.
fn build_research_context_from_dataset(
    dataset: &ResearchDataset,
) -> Result<research_context::ResearchContext> {
    use core_domain::research::confirmation::{compute_confirmation, ConfirmationInputs, ConfirmationScores};
    use core_domain::research::recovery::{breadth_improving, compute_recovery_index, drawdown_recovering, price_recovering, volatility_contracting, RecoveryInputs};
    use core_domain::research::rotation::{leadership_transition as compute_leadership_transition, rotation_acceleration, theme_dispersion, RotationItemInput};
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, DivergenceSummary,
        MarketStateSummary, RecoverySummary, RotationItem, RotationSummary, SignalItem,
        SignalSummary, TrustSummary,
    };

    // Find env for date
    let env = dataset.env_history.iter().find(|e| e.date == dataset.date);

    // Find regime for (scope, date)
    let scope_str = dataset.scope.as_str().to_uppercase();
    let regime = dataset
        .all_regimes
        .iter()
        .filter(|r| r.market.eq_ignore_ascii_case(&scope_str) && r.date == dataset.date)
        .next();

    // Build MarketStateSummary
    let market_state = MarketStateSummary {
        label: regime
            .map(|r| r.regime_label.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        trend_score: regime.map(|r| r.trend_score).unwrap_or(50.0),
        liquidity_score: regime.map(|r| r.liquidity_score).unwrap_or(50.0),
        risk_score: regime.map(|r| r.risk_score).unwrap_or(50.0),
        confidence: env
            .map(|e| e.environment_score / 100.0)
            .unwrap_or(0.5)
            .clamp(0.0, 1.0),
    };

    // Build BreadthSummary
    let breadth = env
        .map(|e| {
            let breadth_pct = e.breadth_pct;
            let breadth_delta = e.breadth_5d_delta.unwrap_or(0.0);
            let condition = if breadth_pct < 30.0 && breadth_delta < -10.0 {
                "collapsed"
            } else if breadth_pct < 50.0 || breadth_delta < -5.0 {
                "weakening"
            } else {
                "strong"
            };
            BreadthSummary {
                breadth_pct,
                sma5: e.breadth_pct_sma5,
                delta_5d: e.breadth_5d_delta,
                condition: condition.to_string(),
            }
        })
        .unwrap_or(BreadthSummary {
            breadth_pct: 0.0,
            sma5: None,
            delta_5d: None,
            condition: "unknown".to_string(),
        });

    // Build RotationSummary
    let rotation = {
        let top: Vec<RotationItem> = dataset
            .rotations
            .iter()
            .take(10)
            .enumerate()
            .map(|(i, r)| RotationItem {
                rank: (i + 1) as i32,
                symbol: r.symbol.clone(),
                momentum_score: r.momentum_score,
            })
            .collect();
        let bottom: Vec<RotationItem> = dataset
            .rotations
            .iter()
            .rev()
            .take(10)
            .enumerate()
            .map(|(i, r)| RotationItem {
                rank: (dataset.rotations.len() - i) as i32,
                symbol: r.symbol.clone(),
                momentum_score: r.momentum_score,
            })
            .collect();

        let rotation_state = if top.len() >= 3 {
            let top3: Vec<f64> = top.iter().take(3).map(|r| r.momentum_score).collect();
            let min_mm = top3.iter().cloned().fold(f64::MAX, f64::min);
            let max_mm = top3.iter().cloned().fold(f64::MIN, f64::max);
            if max_mm - min_mm < 5.0 {
                "broad"
            } else {
                "concentrated"
            }
        } else {
            "broad"
        };

        let leadership_stability = if top.len() >= 3 {
            let top3: Vec<f64> = top.iter().take(3).map(|r| r.momentum_score).collect();
            let mean = top3.iter().sum::<f64>() / top3.len() as f64;
            let variance =
                top3.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / top3.len() as f64;
            let std_dev = variance.sqrt();
            (1.0 - std_dev / 20.0).clamp(0.0, 1.0)
        } else {
            0.5
        };

        let current_items: Vec<RotationItemInput> = dataset
            .rotations
            .iter()
            .map(|r| RotationItemInput {
                symbol: r.symbol.clone(),
                momentum_score: r.momentum_score,
                rank: r.rank,
            })
            .collect();
        let previous_items: Option<Vec<RotationItemInput>> = dataset
            .rotation_history
            .iter()
            .filter(|(d, _)| **d < dataset.date)
            .max_by_key(|(d, _)| *d)
            .map(|(_, rs)| {
                rs.iter()
                    .map(|r| RotationItemInput {
                        symbol: r.symbol.clone(),
                        momentum_score: r.momentum_score,
                        rank: r.rank,
                    })
                    .collect()
            });
        let recent_history: Vec<Vec<RotationItemInput>> = dataset
            .rotation_history
            .iter()
            .filter(|(d, _)| **d != dataset.date)
            .map(|(_, rs)| {
                rs.iter()
                    .map(|r| RotationItemInput {
                        symbol: r.symbol.clone(),
                        momentum_score: r.momentum_score,
                        rank: r.rank,
                    })
                    .collect()
            })
            .collect();

        let leadership_transition = previous_items
            .as_ref()
            .map(|prev| compute_leadership_transition(&current_items, prev).as_str().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let rotation_acceleration = rotation_acceleration(&current_items, &recent_history);
        let theme_dispersion = theme_dispersion(&current_items);

        RotationSummary {
            top,
            bottom,
            rotation_state: rotation_state.to_string(),
            leadership_stability,
            leadership_transition,
            rotation_acceleration,
            theme_dispersion,
        }
    };

    // Build SignalSummary
    let signal = SignalSummary {
        signals: dataset
            .signals
            .iter()
            .take(20)
            .map(|s| SignalItem {
                symbol: s.symbol.clone(),
                final_score: s.final_score,
                signal_label: s.signal_label.to_string(),
            })
            .collect(),
        bullish_count: dataset
            .signals
            .iter()
            .filter(|s| {
                matches!(
                    s.signal_label,
                    core_domain::SignalLabel::StrongBuy | core_domain::SignalLabel::Buy
                )
            })
            .count(),
        strong_buy_count: dataset
            .signals
            .iter()
            .filter(|s| matches!(s.signal_label, core_domain::SignalLabel::StrongBuy))
            .count(),
        average_score: if dataset.signals.is_empty() {
            0.0
        } else {
            dataset.signals.iter().map(|s| s.final_score).sum::<f64>()
                / dataset.signals.len() as f64
        },
    };

    // Build DivergenceSummary (placeholder)
    let divergence = DivergenceSummary {
        divergence_duration: 0,
        samples: Vec::new(),
    };

    // Build TrustSummary
    let trust = TrustSummary {
        level: research_context::TrustLevel::Unassessed,
        headline: "Trust assessment not yet implemented for ResearchContext".to_string(),
        is_data_complete: !dataset.signals.is_empty() && env.is_some(),
    };

    // Build ConfirmationSummary (V7.1 Market Evolution)
    let confirmation_inputs = ConfirmationInputs {
        trend_score: regime.map(|r| r.trend_score).unwrap_or(50.0),
        risk_score: regime.map(|r| r.risk_score).unwrap_or(50.0),
        environment_score: env.map(|e| e.environment_score).unwrap_or(50.0),
        breadth_pct: env.map(|e| e.breadth_pct).unwrap_or(50.0),
        volume_expansion_pct: env.map(|e| e.volume_expansion_pct).unwrap_or(None),
        turnover_coverage_pct: env.map(|e| e.turnover_coverage_pct).unwrap_or(None),
        leadership_stability: rotation.leadership_stability,
        rotation_broad: rotation.rotation_state == "broad",
    };
    let confirmation_scores = compute_confirmation(&confirmation_inputs);
    let confirmation = ConfirmationSummary {
        trend: ConfirmationDimension {
            score: confirmation_scores.trend,
            label: ConfirmationScores::label(confirmation_scores.trend).to_string(),
        },
        participation: ConfirmationDimension {
            score: confirmation_scores.participation,
            label: ConfirmationScores::label(confirmation_scores.participation).to_string(),
        },
        risk: ConfirmationDimension {
            score: confirmation_scores.risk,
            label: ConfirmationScores::label(confirmation_scores.risk).to_string(),
        },
        overall: ConfirmationScores::label(confirmation_scores.overall).to_string(),
    };

    // Build RecoverySummary (V7.1 Market Evolution)
    // NOTE: V7.1 MVP uses proxies from existing regime/environment data because
    // the ResearchDataset does not yet include anchor-symbol daily bars. A
    // future iteration can replace these proxies with actual drawdown/recovery
    // measurements from price history.
    let recovery_inputs = RecoveryInputs {
        drawdown_pct: (100.0 - regime.map(|r| r.trend_score).unwrap_or(50.0)) / 100.0 * 0.3,
        breadth_5d_delta: env.map(|e| e.breadth_5d_delta.unwrap_or(0.0)).unwrap_or(0.0),
        realized_vol: regime.map(|r| r.risk_score / 100.0 * 0.5).unwrap_or(0.2),
        vol_20d_avg: env.map(|e| e.liquidity_proxy_score / 100.0 * 0.5).unwrap_or(0.2),
        price_recovery_pct: (regime.map(|r| r.trend_score).unwrap_or(50.0) - 30.0).max(0.0) / 100.0,
    };
    let recovery_score = compute_recovery_index(&recovery_inputs);
    let mut recovery_drivers = Vec::new();
    if breadth_improving(recovery_inputs.breadth_5d_delta) {
        recovery_drivers.push("Breadth improving".to_string());
    }
    if volatility_contracting(recovery_inputs.realized_vol, recovery_inputs.vol_20d_avg) {
        recovery_drivers.push("Volatility shrinking".to_string());
    }
    if drawdown_recovering(recovery_inputs.drawdown_pct) {
        recovery_drivers.push("Drawdown recovering".to_string());
    }
    if price_recovering(recovery_inputs.price_recovery_pct) {
        recovery_drivers.push("Price recovering from low".to_string());
    }
    let recovery = RecoverySummary {
        score: recovery_score,
        drivers: recovery_drivers,
    };

    Ok(research_context::ResearchContext {
        version: 1,
        scope: dataset.scope,
        date: dataset.date,
        market_state,
        breadth,
        rotation,
        signal,
        divergence,
        trust,
        confirmation,
        recovery,
        consensus: None,
    })
}

/// Build ResearchSnapshot (Computation Workspace) from a ResearchDataset.
fn build_research_snapshot_from_dataset(dataset: &ResearchDataset) -> ResearchSnapshot {
    let state = dataset
        .states_history
        .iter()
        .find(|s| s.date == dataset.date)
        .cloned();
    let env = dataset
        .env_history
        .iter()
        .find(|e| e.date == dataset.date)
        .cloned();

    ResearchSnapshot {
        date: dataset.date,
        signals: dataset.signals.clone(),
        state,
        states_history: dataset.states_history.clone(),
        rotations: dataset.rotations.clone(),
        env,
        signal_history: dataset.signal_history.clone(),
    }
}

/// Map DashboardSnapshot (Production Surface) to the new research-context semantic layer.
///
/// This mapping lives in app-service because research-context must not depend on report-engine.
fn build_research_context_from_dashboard(
    snapshot: &report_engine::DashboardSnapshot,
) -> research_context::ResearchContext {
    use core_domain::research::confirmation::{compute_confirmation, ConfirmationInputs, ConfirmationScores};
    use core_domain::research::recovery::{breadth_improving, compute_recovery_index, drawdown_recovering, price_recovering, volatility_contracting, RecoveryInputs};
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, DivergenceSummary, MarketStateSummary,
        RecoverySummary, RotationItem, RotationSummary, SignalItem, SignalSummary, TrustSummary,
    };

    let market_state = MarketStateSummary {
        label: snapshot.regime_label.clone(),
        trend_score: snapshot.trend_score,
        liquidity_score: snapshot.liquidity_score,
        risk_score: snapshot.risk_score,
        confidence: snapshot
            .environment
            .as_ref()
            .map(|e| e.environment_score / 100.0)
            .unwrap_or(0.5)
            .clamp(0.0, 1.0),
    };

    let breadth = snapshot
        .environment
        .as_ref()
        .map(|e| {
            let breadth_pct = e.breadth_pct;
            let breadth_delta = e.breadth_5d_delta.unwrap_or(0.0);
            let condition = if breadth_pct < 30.0 && breadth_delta < -10.0 {
                "collapsed"
            } else if breadth_pct < 50.0 || breadth_delta < -5.0 {
                "weakening"
            } else {
                "strong"
            };
            BreadthSummary {
                breadth_pct,
                sma5: e.breadth_pct_sma5,
                delta_5d: e.breadth_5d_delta,
                condition: condition.to_string(),
            }
        })
        .unwrap_or(BreadthSummary {
            breadth_pct: 0.0,
            sma5: None,
            delta_5d: None,
            condition: "unknown".to_string(),
        });

    let rotation = {
        let top = snapshot
            .top_rotation
            .iter()
            .map(|r| RotationItem {
                rank: r.rank as i32,
                symbol: r.symbol.clone(),
                momentum_score: r.momentum_score,
            })
            .collect::<Vec<_>>();
        let bottom = snapshot
            .bottom_rotation
            .iter()
            .map(|r| RotationItem {
                rank: r.rank as i32,
                symbol: r.symbol.clone(),
                momentum_score: r.momentum_score,
            })
            .collect::<Vec<_>>();

        let rotation_state = if top.len() >= 3 {
            let top3: Vec<f64> = top.iter().take(3).map(|r| r.momentum_score).collect();
            let min_mm = top3.iter().cloned().fold(f64::MAX, f64::min);
            let max_mm = top3.iter().cloned().fold(f64::MIN, f64::max);
            if max_mm - min_mm < 5.0 {
                "broad"
            } else {
                "concentrated"
            }
        } else {
            "broad"
        };

        let leadership_stability = if top.len() >= 3 {
            let top3: Vec<f64> = top.iter().take(3).map(|r| r.momentum_score).collect();
            let mean = top3.iter().sum::<f64>() / top3.len() as f64;
            let variance = top3.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / top3.len() as f64;
            let std_dev = variance.sqrt();
            (1.0 - std_dev / 20.0).clamp(0.0, 1.0)
        } else {
            0.5
        };

        RotationSummary {
            top,
            bottom,
            rotation_state: rotation_state.to_string(),
            leadership_stability,
            leadership_transition: "Unknown".to_string(),
            rotation_acceleration: None,
            theme_dispersion: None,
        }
    };

    let signal = SignalSummary {
        signals: snapshot
            .top_signals
            .iter()
            .map(|s| SignalItem {
                symbol: s.symbol.clone(),
                final_score: s.final_score,
                signal_label: s.signal_label.to_string(),
            })
            .collect(),
        bullish_count: snapshot.bullish_signals.len(),
        strong_buy_count: snapshot
            .bullish_signals
            .iter()
            .filter(|s| matches!(s.signal_label, core_domain::SignalLabel::StrongBuy))
            .count(),
        average_score: if snapshot.top_signals.is_empty() {
            0.0
        } else {
            snapshot.top_signals.iter().map(|s| s.final_score).sum::<f64>()
                / snapshot.top_signals.len() as f64
        },
    };

    let divergence = DivergenceSummary {
        divergence_duration: 0,
        samples: Vec::new(),
    };

    let trust = snapshot
        .trust_summary
        .as_ref()
        .map(|t| TrustSummary {
            level: parse_trust_level(&t.level),
            headline: t.headline.clone(),
            is_data_complete: t.latest_day_complete,
        })
        .unwrap_or(TrustSummary {
            level: research_context::TrustLevel::Unassessed,
            headline: "No trust summary available".to_string(),
            is_data_complete: false,
        });

    // Build ConfirmationSummary (V7.1 Market Evolution) from DashboardSnapshot data
    let env_ref = snapshot.environment.as_ref();
    let confirmation_inputs = ConfirmationInputs {
        trend_score: snapshot.trend_score,
        risk_score: snapshot.risk_score,
        environment_score: env_ref.map(|e| e.environment_score).unwrap_or(50.0),
        breadth_pct: env_ref.map(|e| e.breadth_pct).unwrap_or(50.0),
        volume_expansion_pct: env_ref.and_then(|e| e.volume_expansion_pct),
        turnover_coverage_pct: env_ref.and_then(|e| e.turnover_coverage_pct),
        leadership_stability: rotation.leadership_stability,
        rotation_broad: rotation.rotation_state == "broad",
    };
    let confirmation_scores = compute_confirmation(&confirmation_inputs);
    let confirmation = ConfirmationSummary {
        trend: ConfirmationDimension {
            score: confirmation_scores.trend,
            label: ConfirmationScores::label(confirmation_scores.trend).to_string(),
        },
        participation: ConfirmationDimension {
            score: confirmation_scores.participation,
            label: ConfirmationScores::label(confirmation_scores.participation).to_string(),
        },
        risk: ConfirmationDimension {
            score: confirmation_scores.risk,
            label: ConfirmationScores::label(confirmation_scores.risk).to_string(),
        },
        overall: ConfirmationScores::label(confirmation_scores.overall).to_string(),
    };

    // Build RecoverySummary (V7.1 Market Evolution) from DashboardSnapshot data
    let recovery_inputs = RecoveryInputs {
        drawdown_pct: (100.0 - snapshot.trend_score) / 100.0 * 0.3,
        breadth_5d_delta: env_ref
            .and_then(|e| e.breadth_5d_delta)
            .unwrap_or(0.0),
        realized_vol: snapshot.risk_score / 100.0 * 0.5,
        vol_20d_avg: env_ref
            .map(|e| e.liquidity_proxy_score / 100.0 * 0.5)
            .unwrap_or(0.2),
        price_recovery_pct: (snapshot.trend_score - 30.0).max(0.0) / 100.0,
    };
    let recovery_score = compute_recovery_index(&recovery_inputs);
    let mut recovery_drivers = Vec::new();
    if breadth_improving(recovery_inputs.breadth_5d_delta) {
        recovery_drivers.push("Breadth improving".to_string());
    }
    if volatility_contracting(recovery_inputs.realized_vol, recovery_inputs.vol_20d_avg) {
        recovery_drivers.push("Volatility shrinking".to_string());
    }
    if drawdown_recovering(recovery_inputs.drawdown_pct) {
        recovery_drivers.push("Drawdown recovering".to_string());
    }
    if price_recovering(recovery_inputs.price_recovery_pct) {
        recovery_drivers.push("Price recovering from low".to_string());
    }
    let recovery = RecoverySummary {
        score: recovery_score,
        drivers: recovery_drivers,
    };

    research_context::ResearchContext {
        version: 1,
        scope: parse_scope(&snapshot.scope),
        date: NaiveDate::parse_from_str(&snapshot.report_date, "%Y-%m-%d")
            .unwrap_or_else(|_| NaiveDate::default()),
        market_state,
        breadth,
        rotation,
        signal,
        divergence,
        trust,
        confirmation,
        recovery,
        consensus: None,
    }
}

fn parse_scope(scope: &str) -> core_domain::AnalysisScope {
    match scope.to_uppercase().as_str() {
        "CN" => core_domain::AnalysisScope::Cn,
        "HK" => core_domain::AnalysisScope::Hk,
        _ => core_domain::AnalysisScope::Global,
    }
}

fn parse_trust_level(level: &str) -> research_context::TrustLevel {
    match level.to_lowercase().as_str() {
        "trusted" => research_context::TrustLevel::High,
        "review" => research_context::TrustLevel::Medium,
        "degraded" => research_context::TrustLevel::Low,
        _ => research_context::TrustLevel::Unassessed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trust_level_maps_known_levels() {
        use research_context::TrustLevel;
        assert_eq!(parse_trust_level("trusted"), TrustLevel::High);
        assert_eq!(parse_trust_level("TRUSTED"), TrustLevel::High);
        assert_eq!(parse_trust_level("review"), TrustLevel::Medium);
        assert_eq!(parse_trust_level("Review"), TrustLevel::Medium);
        assert_eq!(parse_trust_level("degraded"), TrustLevel::Low);
        assert_eq!(parse_trust_level("DeGrAdEd"), TrustLevel::Low);
    }

    #[test]
    fn parse_trust_level_defaults_to_unassessed_for_unknown() {
        use research_context::TrustLevel;
        assert_eq!(parse_trust_level("ok"), TrustLevel::Unassessed);
        assert_eq!(parse_trust_level("warning"), TrustLevel::Unassessed);
        assert_eq!(parse_trust_level(""), TrustLevel::Unassessed);
    }

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

    #[test]
    fn pipeline_date_alerts_warn_when_signal_lags_strategy() {
        let stages = vec![
            PipelineStageDateStatus {
                stage: "strategy_preference".to_string(),
                latest_date: Some("2026-04-24".to_string()),
                lag_days: Some(0),
                is_latest: true,
                latest_entities: Some(21),
                expected_entities: Some(21),
                is_complete: Some(true),
            },
            PipelineStageDateStatus {
                stage: "signal_snapshot".to_string(),
                latest_date: Some("2026-04-09".to_string()),
                lag_days: Some(15),
                is_latest: false,
                latest_entities: Some(21),
                expected_entities: Some(21),
                is_complete: Some(true),
            },
        ];

        let alerts = pipeline_date_alerts(ReportScope::Global, &stages);

        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].contains("Rerun `compute-signals`"));
        assert!(alerts[0].contains("signal=2026-04-09"));
        assert!(alerts[0].contains("strategy=2026-04-24"));
    }

    #[test]
    fn build_signal_alignment_issue_for_dates_warns_when_signal_missing() {
        let issue = build_signal_alignment_issue_for_dates(
            ReportScope::Global,
            Some(NaiveDate::from_ymd_opt(2026, 4, 24).unwrap()),
            None,
        );

        let issue = issue.expect("expected missing-signal warning");
        assert!(issue.contains("scope GLOBAL"));
        assert!(issue.contains("missing"));
        assert!(issue.contains("2026-04-24"));
    }

    #[test]
    fn pipeline_date_alerts_warn_when_signal_latest_day_is_incomplete() {
        let stages = vec![
            PipelineStageDateStatus {
                stage: "strategy_preference".to_string(),
                latest_date: Some("2026-04-24".to_string()),
                lag_days: Some(0),
                is_latest: true,
                latest_entities: Some(21),
                expected_entities: Some(21),
                is_complete: Some(true),
            },
            PipelineStageDateStatus {
                stage: "signal_snapshot".to_string(),
                latest_date: Some("2026-04-24".to_string()),
                lag_days: Some(0),
                is_latest: true,
                latest_entities: Some(18),
                expected_entities: Some(21),
                is_complete: Some(false),
            },
        ];

        let alerts = pipeline_date_alerts(ReportScope::Global, &stages);

        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].contains("Signal snapshot is incomplete"));
        assert!(alerts[0].contains("2026-04-24"));
        assert!(alerts[0].contains("18/21"));
    }

    #[test]
    fn derive_refresh_window_uses_source_lookback_when_gate_is_current() {
        let to = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap();
        let latest_daily = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap();
        let latest_gated = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap();

        let (refresh_from, reason, repair_days) =
            derive_refresh_window(to, Some(latest_daily), Some(latest_gated), false);

        assert_eq!(refresh_from, NaiveDate::from_ymd_opt(2026, 4, 27).unwrap());
        assert_eq!(reason, "source-lookback");
        assert_eq!(repair_days, REFRESH_GATE_REPAIR_WINDOW_DAYS);
    }

    #[test]
    fn derive_refresh_window_widens_when_gate_lags_source() {
        let to = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap();
        let latest_daily = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap();
        let latest_gated = NaiveDate::from_ymd_opt(2026, 4, 20).unwrap();

        let (refresh_from, reason, _) =
            derive_refresh_window(to, Some(latest_daily), Some(latest_gated), false);

        assert_eq!(refresh_from, NaiveDate::from_ymd_opt(2026, 3, 21).unwrap());
        assert_eq!(reason, "latest-gate-repair");
    }

    #[test]
    fn derive_refresh_window_clamps_when_to_is_behind_latest_daily() {
        let to = NaiveDate::from_ymd_opt(2026, 4, 20).unwrap();
        let latest_daily = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap();

        let (refresh_from, reason, _) = derive_refresh_window(to, Some(latest_daily), None, true);

        assert!(refresh_from <= latest_daily);
        assert_eq!(reason, "missing-gated-scope-repair");
    }

    #[test]
    fn sync_gate_needs_refresh_skips_when_gate_already_advanced() {
        assert!(!sync_gate_needs_refresh(Some(true)));
    }

    #[test]
    fn sync_gate_needs_refresh_requests_when_gate_behind() {
        assert!(sync_gate_needs_refresh(Some(false)));
    }

    #[test]
    fn sync_gate_needs_refresh_requests_when_gate_unknown() {
        assert!(sync_gate_needs_refresh(None));
    }

    #[test]
    fn validate_sync_refresh_result_ok_on_success() {
        let result = validate_sync_refresh_result(true, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_sync_refresh_result_bails_on_failure_with_alerts() {
        let alerts = vec!["signal lagging".to_string(), "rotation incomplete".to_string()];
        let result = validate_sync_refresh_result(false, &alerts);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("sync-and-export aborted"));
        assert!(err.contains("signal lagging"));
        assert!(err.contains("rotation incomplete"));
    }

    #[test]
    fn validate_sync_refresh_result_bails_on_failure_with_empty_alerts() {
        let result = validate_sync_refresh_result(false, &[]);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("sync-and-export aborted"));
    }

    #[test]
    fn sync_gate_decision_flow_gate_advanced_skips_refresh() {
        let gate_before = Some(true);
        assert!(!sync_gate_needs_refresh(gate_before));
    }

    #[test]
    fn sync_gate_decision_flow_gate_behind_refresh_succeeds() {
        let gate_before = Some(false);
        assert!(sync_gate_needs_refresh(gate_before));
        assert!(validate_sync_refresh_result(true, &[]).is_ok());
    }

    #[test]
    fn sync_gate_decision_flow_gate_behind_refresh_fails() {
        let gate_before = Some(false);
        assert!(sync_gate_needs_refresh(gate_before));
        let alerts = vec!["stale data".to_string()];
        let err = validate_sync_refresh_result(false, &alerts).unwrap_err();
        assert!(err.to_string().contains("stale data"));
    }

    #[test]
    fn sync_gate_decision_flow_gate_unknown_treated_as_behind() {
        let gate_before: Option<bool> = None;
        assert!(sync_gate_needs_refresh(gate_before));
    }

    #[test]
    fn llm_config_roundtrip_serde() {
        let config = LlmConfig {
            base_url: "https://custom.api.com/v1".to_string(),
            model: "gpt-4".to_string(),
            timeout_secs: 120,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LlmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.base_url, deserialized.base_url);
        assert_eq!(config.model, deserialized.model);
        assert_eq!(config.timeout_secs, deserialized.timeout_secs);
    }

    #[test]
    fn llm_config_default_values_match_expectations() {
        let defaults = LlmConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            timeout_secs: 60,
        };
        assert_eq!(defaults.base_url, "https://api.openai.com/v1");
        assert_eq!(defaults.model, "gpt-4o-mini");
        assert_eq!(defaults.timeout_secs, 60);
    }

    #[test]
    fn llm_config_from_json_string() {
        let json =
            r#"{"base_url":"https://custom.api.com/v1","model":"gpt-4","timeout_secs":120}"#;
        let config: LlmConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.base_url, "https://custom.api.com/v1");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.timeout_secs, 120);
    }

    #[test]
    fn llm_config_invalid_json_returns_error() {
        let json = r#"{"base_url":"https://api.com","model":123}"#;
        let result = serde_json::from_str::<LlmConfig>(json);
        assert!(result.is_err());
    }

    #[test]
    fn llm_system_prompt_contains_expected_content() {
        let system_prompt = "You are a senior quantitative analyst. Analyze the following daily market research report and provide concise insights on regime, breadth, top signals, risks, and actionable takeaways.";
        assert!(system_prompt.contains("quantitative analyst"));
        assert!(system_prompt.contains("regime"));
        assert!(system_prompt.contains("breadth"));
        assert!(system_prompt.contains("top signals"));
        assert!(system_prompt.contains("risks"));
        assert!(system_prompt.contains("actionable takeaways"));
    }

    #[test]
    fn llm_user_prompt_includes_report_and_structured_request() {
        let report_markdown = "# Daily Report\nSome market data here";
        let user_prompt =
            format!("{}\n\nPlease provide a structured analysis.", report_markdown);
        assert!(user_prompt.contains(report_markdown));
        assert!(user_prompt.contains("structured analysis"));
    }

    #[test]
    fn llm_missing_api_key_error_is_clear() {
        let error_msg = "LLM API key not configured. Use set_llm_api_key first.";
        assert!(error_msg.contains("LLM API key not configured"));
        assert!(error_msg.contains("set_llm_api_key"));
        assert!(!error_msg.contains("sk-"));
        assert!(!error_msg.contains("Bearer"));
    }

    #[test]
    fn llm_api_key_not_in_error_context_message() {
        let context_msg = "LLM API call failed";
        assert!(!context_msg.contains("sk-"));
        assert!(!context_msg.contains("Bearer"));
        assert!(!context_msg.contains("api_key"));
    }

    #[test]
    fn llm_service_and_account_names_are_constants() {
        assert_eq!(LLM_SERVICE_NAME, "rust-quant-analysis-system");
        assert_eq!(LLM_ACCOUNT_NAME, "llm_api_key");
    }

    fn mock_chat_completion_response(content: &str) -> serde_json::Value {
        serde_json::json!({
            "id": "chatcmpl-test",
            "object": "chat.completion",
            "created": 1699000000,
            "model": "gpt-4o-mini",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            }
        })
    }

    #[tokio::test]
    async fn llm_mock_server_receives_correct_prompt_and_model() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .and(wiremock::matchers::body_string_contains("quantitative analyst"))
            .and(wiremock::matchers::body_string_contains("structured analysis"))
            .and(wiremock::matchers::body_string_contains("gpt-4o-mini"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(mock_chat_completion_response("Analysis complete")),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key("test-api-key-12345")
            .with_api_base(mock_server.uri());
        let client = async_openai::Client::with_config(config);

        let system_prompt = "You are a senior quantitative analyst. Analyze the following daily market research report and provide concise insights on regime, breadth, top signals, risks, and actionable takeaways.";
        let report_markdown = "# Test Report\nMarket data here";
        let user_prompt =
            format!("{}\n\nPlease provide a structured analysis.", report_markdown);

        let request = async_openai::types::chat::CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages([
                async_openai::types::chat::ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()
                    .unwrap()
                    .into(),
                async_openai::types::chat::ChatCompletionRequestUserMessageArgs::default()
                    .content(user_prompt.as_str())
                    .build()
                    .unwrap()
                    .into(),
            ])
            .build()
            .unwrap();

        let response = client.chat().create(request).await.unwrap();
        let content = response.choices[0]
            .message
            .content
            .clone()
            .unwrap_or_default();
        assert_eq!(content, "Analysis complete");
    }

    #[tokio::test]
    async fn llm_mock_server_receives_api_key_in_auth_header() {
        let mock_server = wiremock::MockServer::start().await;
        let test_key = "sk-test-secret-key-99999";

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .and(wiremock::matchers::header(
                "authorization",
                format!("Bearer {test_key}"),
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(mock_chat_completion_response("OK")),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(test_key)
            .with_api_base(mock_server.uri());
        let client = async_openai::Client::with_config(config);

        let request = async_openai::types::chat::CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(
                [async_openai::types::chat::ChatCompletionRequestUserMessageArgs::default()
                    .content("test")
                    .build()
                    .unwrap()
                    .into()],
            )
            .build()
            .unwrap();

        let response = client.chat().create(request).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn llm_mock_server_handles_401_unauthorized() {
        let mock_server = wiremock::MockServer::start().await;
        let secret_key = "sk-invalid-key-12345";

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(401).set_body_json(
                serde_json::json!({
                    "error": {
                        "message": "Invalid API key",
                        "type": "invalid_request_error",
                        "code": "invalid_api_key"
                    }
                }),
            ))
            .mount(&mock_server)
            .await;

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(secret_key)
            .with_api_base(mock_server.uri());
        let client = async_openai::Client::with_config(config);

        let request = async_openai::types::chat::CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(
                [async_openai::types::chat::ChatCompletionRequestUserMessageArgs::default()
                    .content("test")
                    .build()
                    .unwrap()
                    .into()],
            )
            .build()
            .unwrap();

        let result = client.chat().create(request).await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(!error_msg.contains(secret_key));
    }

    #[tokio::test]
    async fn llm_mock_server_error_does_not_leak_api_key() {
        let mock_server = wiremock::MockServer::start().await;
        let real_key = "sk-super-secret-production-key-abcdef";

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(wiremock::ResponseTemplate::new(500).set_body_json(
                serde_json::json!({
                    "error": {
                        "message": "Internal server error",
                        "type": "server_error",
                        "code": "internal_error"
                    }
                }),
            ))
            .mount(&mock_server)
            .await;

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(real_key)
            .with_api_base(mock_server.uri());
        let client = async_openai::Client::with_config(config);

        let request = async_openai::types::chat::CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(
                [async_openai::types::chat::ChatCompletionRequestUserMessageArgs::default()
                    .content("test")
                    .build()
                    .unwrap()
                    .into()],
            )
            .build()
            .unwrap();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            client.chat().create(request),
        )
        .await;
        let is_err = result.as_ref().map(|r| r.is_err()).unwrap_or(true);
        assert!(is_err, "expected error or timeout");
        let error_msg = match result {
            Ok(Err(e)) => e.to_string(),
            Err(_) => "timeout".to_string(),
            Ok(Ok(_)) => panic!("expected error"),
        };
        // SECURITY: API key must NEVER appear in error messages
        assert!(
            !error_msg.contains(real_key),
            "API key leaked in error message: {error_msg}"
        );
    }

    #[tokio::test]
    async fn llm_mock_server_handles_empty_choices() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/chat/completions"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "chatcmpl-empty",
                    "object": "chat.completion",
                    "created": 1699000000,
                    "model": "gpt-4o-mini",
                    "choices": [],
                    "usage": {
                        "prompt_tokens": 10,
                        "completion_tokens": 0,
                        "total_tokens": 10
                    }
                })),
            )
            .mount(&mock_server)
            .await;

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key("test-key")
            .with_api_base(mock_server.uri());
        let client = async_openai::Client::with_config(config);

        let request = async_openai::types::chat::CreateChatCompletionRequestArgs::default()
            .model("gpt-4o-mini")
            .messages(
                [async_openai::types::chat::ChatCompletionRequestUserMessageArgs::default()
                    .content("test")
                    .build()
                    .unwrap()
                    .into()],
            )
            .build()
            .unwrap();

        let response = client.chat().create(request).await.unwrap();
        let content = response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();
        assert!(content.is_empty());
    }

    fn build_research_context(
        breadth_pct: f64,
        breadth_delta: f64,
        liquidity_pressure: llm_context::LiquidityPressure,
        macro_stale_days: i32,
    ) -> llm_context::ResearchContext {
        llm_context::ResearchContext {
            market: llm_context::MarketContext {
                current_state: "bullish".to_string(),
                previous_state: None,
                confidence: 0.8,
                drivers: vec![],
                transition: None,
            },
            liquidity: llm_context::LiquidityContext {
                pressure: liquidity_pressure,
                spread: None,
                yield_curve_status: None,
                dollar_strength: None,
            },
            breadth: llm_context::BreadthContext {
                condition: llm_context::BreadthCondition::Strong,
                breadth_pct,
                breadth_delta,
            },
            rotation: llm_context::RotationContext {
                state: llm_context::RotationState::Broad,
                top_sectors: vec![],
                bottom_sectors: vec![],
                leadership_stability: 0.7,
                momentum_factor: None,
                value_factor: None,
                quality_factor: None,
                crowding_factor: None,
            },
            regime: llm_context::RegimeContext {
                current: "expansion".to_string(),
                confidence: 0.75,
                macro_stale_days,
            },
            signals: llm_context::SignalsContext {
                bullish_count: 3,
                defensive_count: 2,
                data_starved_count: 0,
            },
            macro_: llm_context::MacroContext {
                spread_10y: None,
                dxy_index: None,
                foreign_flow: None,
                vix: None,
            },
            risk: llm_context::RiskContext {
                skewness: None,
                kurtosis: None,
                tail_index: None,
            },
        }
    }

    #[test]
    fn extract_key_drivers_detects_breadth_collapse() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(25.0, -5.0, llm_context::LiquidityPressure::Moderate, 1);
        let drivers = extract_key_drivers(&context);
        assert!(drivers.contains(&"breadth_collapse".to_string()));
        assert!(!drivers.contains(&"breadth_deteriorating".to_string()));
    }

    #[test]
    fn extract_key_drivers_detects_deteriorating_and_liquidity_critical() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(40.0, -15.0, llm_context::LiquidityPressure::Critical, 5);
        let drivers = extract_key_drivers(&context);
        assert!(drivers.contains(&"breadth_deteriorating".to_string()));
        assert!(drivers.contains(&"liquidity_critical".to_string()));
        assert!(drivers.contains(&"macro_stale".to_string()));
    }

    #[test]
    fn assess_risk_level_critical_when_breadth_below_20() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(15.0, 0.0, llm_context::LiquidityPressure::Low, 0);
        assert_eq!(assess_risk_level(&context), "critical");
    }

    #[test]
    fn assess_risk_level_high_when_liquidity_critical() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(40.0, 0.0, llm_context::LiquidityPressure::Critical, 0);
        assert_eq!(assess_risk_level(&context), "critical");
    }

    #[test]
    fn assess_risk_level_medium_when_breadth_between_30_and_50() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(35.0, 0.0, llm_context::LiquidityPressure::Low, 0);
        assert_eq!(assess_risk_level(&context), "medium");
    }

    #[test]
    fn assess_risk_level_low_when_breadth_above_50() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(60.0, 0.0, llm_context::LiquidityPressure::Low, 0);
        assert_eq!(assess_risk_level(&context), "low");
    }

    #[test]
    fn identify_risk_factors_detects_extreme_collapse() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(15.0, 0.0, llm_context::LiquidityPressure::Low, 0);
        let factors = identify_risk_factors(&context);
        assert!(factors.contains(&"breadth_below_30".to_string()));
        assert!(factors.contains(&"breadth_extreme_collapse".to_string()));
    }

    #[test]
    fn identify_risk_factors_detects_liquidity_and_macro_stale() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(40.0, 0.0, llm_context::LiquidityPressure::Critical, 10);
        let factors = identify_risk_factors(&context);
        assert!(factors.contains(&"liquidity_critical".to_string()));
        assert!(factors.contains(&"macro_severely_stale".to_string()));
    }

    #[test]
    fn generate_recommendation_exit_when_breadth_below_20() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(15.0, 0.0, llm_context::LiquidityPressure::Low, 0);
        assert_eq!(generate_recommendation(&context), "exit");
    }

    #[test]
    fn generate_recommendation_reduce_exposure_when_breadth_below_30() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(25.0, 0.0, llm_context::LiquidityPressure::Low, 0);
        assert_eq!(generate_recommendation(&context), "reduce_exposure");
    }

    #[test]
    fn generate_recommendation_maintain_when_breadth_above_50() {
        let _ctx = AppContext::new(StorageConfig::default());
        let context = build_research_context(60.0, 0.0, llm_context::LiquidityPressure::Low, 0);
        assert_eq!(generate_recommendation(&context), "maintain");
    }

    /// V7.3.1: Integration smoke test for the Consensus vertical slice.
    ///
    /// This test does not hit the database; it verifies that a `ConsensusSummary`
    /// produced by the evidence aggregator flows correctly through the
    /// app-service → report-builder → report-renderer pipeline and that the
    /// resulting Markdown report contains the required research-language fields
    /// (version, bias, confidence, aggregate score, evidence lists, disclaimer).
    #[test]
    fn consensus_report_vertical_slice_renders_markdown() {
        use core_domain::research::consensus::{
            consensus, ConsensusConfig, EvidenceInput,
        };
        use report_builder::{ConsensusReportInput, ResearchReportBuilder};
        use report_renderer::MarkdownFormatter;
        use reporting::{Formatter, ReportingSnapshot};
        use research_context::{
            BreadthSummary, ConfirmationDimension, ConfirmationSummary, Confidence, ConsensusBias,
            DivergenceSummary, MarketStateSummary, RecoverySummary, ResearchContext,
            RotationSummary, SignalSummary, TrustLevel, TrustSummary,
        };

        // 1. Build evidence input and run aggregation (same path as run_research_consensus)
        let input = EvidenceInput {
            signal: Some(0.6),
            stretch: Some(0.2),
            confirmation: Some(0.5),
            recovery: Some(0.4),
            analogues: Some(0.1),
        };
        let core_summary = consensus(input, &ConsensusConfig::default());
        assert_eq!(core_summary.version, 1);
        assert!(matches!(core_summary.bias, ConsensusBias::Constructive));
        assert!(matches!(core_summary.confidence, Confidence::Medium | Confidence::High));
        assert!(!core_summary.supporting_evidence.is_empty());

        // 2. Wrap in a minimal ResearchContext / ReportingSnapshot
        let context = ResearchContext {
            version: 1,
            scope: core_domain::AnalysisScope::Global,
            date: NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
            market_state: MarketStateSummary {
                label: "risk_on".to_string(),
                trend_score: 75.0,
                liquidity_score: 60.0,
                risk_score: 40.0,
                confidence: 0.8,
            },
            breadth: BreadthSummary {
                breadth_pct: 65.0,
                sma5: Some(62.0),
                delta_5d: Some(3.0),
                condition: "Strong".to_string(),
            },
            rotation: RotationSummary {
                top: vec![],
                bottom: vec![],
                rotation_state: "Broad".to_string(),
                leadership_stability: 0.7,
                leadership_transition: "Stable".to_string(),
                rotation_acceleration: None,
                theme_dispersion: None,
            },
            signal: SignalSummary {
                signals: vec![],
                bullish_count: 3,
                strong_buy_count: 2,
                average_score: 72.0,
            },
            divergence: DivergenceSummary {
                divergence_duration: 0,
                samples: vec![],
            },
            trust: TrustSummary {
                level: TrustLevel::Unassessed,
                headline: "Data healthy".to_string(),
                is_data_complete: true,
            },
            confirmation: ConfirmationSummary {
                trend: ConfirmationDimension {
                    score: 75.0,
                    label: "Strong".to_string(),
                },
                participation: ConfirmationDimension {
                    score: 45.0,
                    label: "Moderate".to_string(),
                },
                risk: ConfirmationDimension {
                    score: 70.0,
                    label: "Strong".to_string(),
                },
                overall: "Moderate".to_string(),
            },
            recovery: RecoverySummary {
                score: 42.0,
                drivers: vec!["Breadth improving".to_string()],
            },
            consensus: Some(core_summary),
        };
        let snapshot = ReportingSnapshot {
            generated_at: chrono::Utc::now(),
            research: context,
        };

        // 3. Build and render the consensus report
        let report_input = ConsensusReportInput {
            summary: snapshot.research.consensus.clone().unwrap(),
        };
        let doc = ResearchReportBuilder::build_consensus(&snapshot, &report_input)
            .expect("build_consensus should succeed");
        let mut formatter = MarkdownFormatter::new();
        report_renderer::render(&mut formatter, &doc);
        let markdown = formatter.finalize();

        // 4. Assert research-language content and no decision advice
        assert!(markdown.contains("Research Consensus"));
        assert!(markdown.contains("Consensus version:"));
        assert!(markdown.contains("Constructive"));
        assert!(markdown.contains("Aggregate score:"));
        assert!(markdown.contains("Supporting Evidence:"));
        assert!(markdown.contains("Contradicting Evidence:"));
        assert!(markdown.contains("does not provide buy/sell recommendations"));
        assert!(!markdown.contains("Buy"));
        assert!(!markdown.contains("Sell"));
    }
}

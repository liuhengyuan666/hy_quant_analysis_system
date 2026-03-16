use anyhow::{Context, Result};
use backtest_engine::{run_signal_backtest, BacktestConfig};
use chrono::{Duration, NaiveDate, Utc};
use core_domain::{Instrument, InstrumentType};
use data_ingestion::{
    fetch_daily_bars, fetch_eastmoney_daily_bars, fetch_fred_series, fetch_fred_series_with_status,
    fetch_tencent_daily_bars, load_universe,
};
use indicator_engine::build_indicator_snapshots;
use macro_engine::{build_macro_snapshots, build_market_regimes};
use market_store::StorageConfig;
use report_engine::{
    build_dashboard_snapshot, collect_dashboard_dates, render_data_health_report,
    render_markdown_report, DashboardSnapshot, DataHealthMacroSourceSummary, DataHealthSummary,
    DataHealthSymbolSummary,
};
use rotation_engine::build_rotation_ranks;
use serde::Serialize;
use signal_engine::build_signal_snapshots;
use std::collections::BTreeMap;
use std::fs;
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
    primary_provider_ok: bool,
    fallback_provider_ok: Option<bool>,
    gap_count: usize,
    suspicious_jump_count: usize,
    missing_turnover_rows: usize,
) -> String {
    if rows == 0 || (!primary_provider_ok && fallback_provider_ok != Some(true)) {
        "critical".to_string()
    } else if !primary_provider_ok
        || gap_count > 0
        || suspicious_jump_count > 0
        || missing_turnover_rows > 0
    {
        "review".to_string()
    } else {
        "healthy".to_string()
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
        let regimes = market_store::fetch_market_regimes(&self.storage)?;
        let rotations = market_store::fetch_rotation_ranks(&self.storage)?;
        let signals = market_store::fetch_signal_snapshots(&self.storage)?;
        let latest_backtest = market_store::fetch_latest_backtest_run(&self.storage)?;
        Ok(build_dashboard_snapshot(
            &regimes,
            &rotations,
            &signals,
            latest_backtest,
            report_date,
        ))
    }

    pub fn dashboard_available_dates(&self) -> Result<Vec<String>> {
        let regimes = market_store::fetch_market_regimes(&self.storage)?;
        let rotations = market_store::fetch_rotation_ranks(&self.storage)?;
        let signals = market_store::fetch_signal_snapshots(&self.storage)?;
        Ok(collect_dashboard_dates(&regimes, &rotations, &signals))
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
                primary_provider_ok,
                fallback_provider_ok,
                gap_count,
                suspicious_jump_count,
                missing_turnover_rows,
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

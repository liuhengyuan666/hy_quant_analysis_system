use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use backtest_engine::{BacktestEquityPoint, BacktestSummary, BacktestTrade};
use chrono::NaiveDate;
use core_domain::{
    AnalysisScope, DailyBar, EnvironmentSnapshot, IndicatorSnapshot, Instrument, MacroSnapshot,
    MarketRegimeSnapshot, RegimeReason, RotationRankSnapshot, RotationReason, SignalReason,
    SignalSnapshot, StrategyKind, StrategyPreferenceSnapshot, StrategyStateSnapshot,
};
use reqwest::blocking::Client;
use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub clickhouse_url: String,
    pub clickhouse_database: String,
    pub clickhouse_user: String,
    pub clickhouse_password: String,
    pub sqlite_path: String,
    pub universe_path: String,
    pub profile: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            clickhouse_url: "http://127.0.0.1:18123".to_string(),
            clickhouse_database: "quant".to_string(),
            clickhouse_user: "quant_user".to_string(),
            clickhouse_password: "quant_pass".to_string(),
            sqlite_path: "data/app_state.db".to_string(),
            universe_path: "config/universe.json".to_string(),
            profile: "local".to_string(),
        }
    }
}

impl StorageConfig {
    pub fn project_root() -> Result<PathBuf> {
        let mut current = std::env::current_dir().context("failed to get current directory")?;
        loop {
            if current.join("Cargo.toml").exists() && current.join("crates").exists() {
                return Ok(current);
            }
            if !current.pop() {
                anyhow::bail!("failed to locate project root from current directory")
            }
        }
    }

    pub fn sqlite_abspath(&self) -> Result<PathBuf> {
        Ok(Self::project_root()?.join(&self.sqlite_path))
    }

    pub fn universe_abspath(&self) -> Result<PathBuf> {
        Ok(Self::project_root()?.join(&self.universe_path))
    }
}

fn read_sql_file(relative_path: &str) -> Result<String> {
    let root = StorageConfig::project_root()?;
    let path = root.join(relative_path);
    fs::read_to_string(&path)
        .with_context(|| format!("failed to read SQL file: {}", path.display()))
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(str::trim)
        .filter(|stmt| !stmt.is_empty())
        .map(|stmt| format!("{stmt};"))
        .collect()
}

fn escape_sql_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

pub fn init_sqlite(config: &StorageConfig) -> Result<()> {
    let sqlite_path = config.sqlite_abspath()?;
    if let Some(parent) = sqlite_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create sqlite directory: {}", parent.display()))?;
    }
    let schema = read_sql_file("sql/sqlite/001_init.sql")?;
    let connection = Connection::open(&sqlite_path)
        .with_context(|| format!("failed to open sqlite database: {}", sqlite_path.display()))?;
    connection
        .execute_batch(&schema)
        .context("failed to initialize sqlite schema")?;
    Ok(())
}

fn clickhouse_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(Client::new)
}

fn parse_json_each_row<T: DeserializeOwned>(body: &str, row_context: &str) -> Result<Vec<T>> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<T>(line).with_context(|| row_context.to_string()))
        .collect()
}

fn encode_symbol_list(symbols: &[String]) -> String {
    symbols
        .iter()
        .map(|symbol| format!("'{}'", escape_sql_string(symbol)))
        .collect::<Vec<_>>()
        .join(",")
}

fn ensure_environment_snapshot_table(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "CREATE TABLE IF NOT EXISTS quant.environment_snapshot (date Date,scope LowCardinality(String),regime_as_of_date Date,breadth_as_of_date Date,stress_as_of_date Date,breadth_eligible_count UInt32,breadth_above_count UInt32,breadth_pct Float64,breadth_pct_sma5 Nullable(Float64),breadth_5d_delta Nullable(Float64),breadth_state LowCardinality(String),volume_expansion_pct Nullable(Float64),turnover_coverage_pct Nullable(Float64),liquidity_proxy_score Float64,stress_proxy_score Float64,environment_score Float64,environment_label LowCardinality(String),updated_at DateTime DEFAULT now()) ENGINE = MergeTree PARTITION BY toYYYYMM(date) ORDER BY (scope, date)",
    )
}

fn ensure_strategy_state_table(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "CREATE TABLE IF NOT EXISTS quant.strategy_state (date Date,scope LowCardinality(String),state LowCardinality(String),state_score Float64,transition_reason String,recommended_position_pct Float64,updated_at DateTime DEFAULT now()) ENGINE = MergeTree PARTITION BY toYYYYMM(date) ORDER BY (scope, date)",
    )
}

fn ensure_strategy_preference_scope_columns(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.strategy_preference ADD COLUMN IF NOT EXISTS analysis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.strategy_preference ADD COLUMN IF NOT EXISTS regime_basis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )
}

fn ensure_signal_snapshot_provenance_columns(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.signal_snapshot ADD COLUMN IF NOT EXISTS analysis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.signal_snapshot ADD COLUMN IF NOT EXISTS regime_basis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )
}

fn ensure_backtest_run_provenance_columns(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS analysis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS signal_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS regime_basis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS signal_start_date Nullable(Date)",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS signal_end_date Nullable(Date)",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS config_summary String DEFAULT ''",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS drawdown_events UInt64 DEFAULT 0",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS state_trajectory_json String DEFAULT ''",
    )
}

fn fetch_max_date_for_table_with_filter(
    config: &StorageConfig,
    table_name: &str,
    filter_column: &str,
    filter_value: &str,
) -> Result<Option<NaiveDate>> {
    let query = format!(
        "SELECT count() AS row_count, max(date) AS max_date FROM quant.{table_name} WHERE {filter_column} = '{}' FORMAT JSONEachRow",
        escape_sql_string(filter_value)
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse scoped max date row")?;
    if json_u64(row.get("row_count")).unwrap_or(0) == 0 {
        return Ok(None);
    }
    let Some(text) = row.get("max_date").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    Ok(Some(NaiveDate::parse_from_str(text, "%Y-%m-%d")?))
}

pub fn fetch_distinct_entity_count_for_date_with_filter(
    config: &StorageConfig,
    table_name: &str,
    entity_column: &str,
    filter_column: &str,
    filter_value: &str,
    date: NaiveDate,
) -> Result<usize> {
    let query = format!(
        "SELECT count(DISTINCT {entity_column}) AS entities FROM quant.{table_name} WHERE date = '{date}' AND {filter_column} = '{}' FORMAT JSONEachRow",
        escape_sql_string(filter_value)
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(0);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse filtered distinct entity row")?;
    Ok(json_u64(row.get("entities")).unwrap_or(0) as usize)
}

fn fetch_max_date_for_table(config: &StorageConfig, table_name: &str) -> Result<Option<NaiveDate>> {
    let query = format!(
        "SELECT count() AS row_count, max(date) AS max_date FROM quant.{} FORMAT JSONEachRow",
        table_name
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse max date row")?;
    if json_u64(row.get("row_count")).unwrap_or(0) == 0 {
        return Ok(None);
    }
    let Some(text) = row.get("max_date").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    Ok(Some(NaiveDate::parse_from_str(text, "%Y-%m-%d")?))
}

fn decode_signal_snapshot_row(mut row: serde_json::Value) -> Result<SignalSnapshot> {
    if let Some(signal_label) = row.get("signal_label").and_then(|value| value.as_str()) {
        row["signal_label"] = match signal_label {
            "STRONG_BUY" => serde_json::json!("StrongBuy"),
            "BUY" => serde_json::json!("Buy"),
            "WATCH" => serde_json::json!("Watch"),
            "HOLD" => serde_json::json!("Hold"),
            "REDUCE" => serde_json::json!("Reduce"),
            "SELL" => serde_json::json!("Sell"),
            _ => serde_json::json!("Hold"),
        };
    }
    if row.get("analysis_scope").is_none() {
        row["analysis_scope"] = serde_json::json!("GLOBAL");
    }
    if row.get("regime_basis_scope").is_none() {
        row["regime_basis_scope"] = serde_json::json!("GLOBAL");
    }
    let explanation = row
        .get("explanation")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    row["reason"] = match serde_json::from_str::<SignalReason>(&explanation) {
        Ok(reason) => serde_json::to_value(reason)?,
        Err(_) => serde_json::to_value(fallback_signal_reason(&row, &explanation))?,
    };
    if let Some(object) = row.as_object_mut() {
        object.remove("explanation");
    }
    serde_json::from_value::<SignalSnapshot>(row).context("failed to decode signal snapshot")
}

fn fallback_signal_reason(row: &serde_json::Value, summary: &str) -> SignalReason {
    let final_score = row
        .get("final_score")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let label = row
        .get("signal_label")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or(core_domain::SignalLabel::Hold);
    SignalReason {
        best_strategy: StrategyKind::ValueLeft,
        strategy_score: 0.0,
        strategy_contribution: 0.0,
        alignment: 0,
        aligned_strategies: Vec::new(),
        alignment_contribution: 0.0,
        regime: RegimeReason {
            trend_score: 50.0,
            risk_score: 50.0,
            combined_score: 50.0,
            contribution: 10.0,
        },
        rotation: RotationReason {
            momentum_score: 40.0,
            rank: None,
            combined_score: 40.0,
            contribution: 8.0,
        },
        final_score,
        label,
        summary: summary.to_string(),
    }
}

fn parse_state_trajectory(value: Option<&serde_json::Value>) -> Vec<(NaiveDate, String)> {
    value
        .and_then(|value| value.as_str())
        .filter(|text| !text.trim().is_empty())
        .and_then(|text| serde_json::from_str::<Vec<(NaiveDate, String)>>(text).ok())
        .unwrap_or_default()
}

fn fetch_clickhouse_text(config: &StorageConfig, query: &str) -> Result<String> {
    let encoded = urlencoding::encode(query);
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url, config.clickhouse_database, encoded
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(String::new())
        .send()
        .context("failed to fetch ClickHouse text response")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "ClickHouse text query failed with status {}",
            response.status()
        );
    }
    response
        .text()
        .context("failed to read ClickHouse text response")
}

fn json_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    match value {
        Some(serde_json::Value::Number(number)) => number.as_u64(),
        Some(serde_json::Value::String(text)) => text.parse::<u64>().ok(),
        _ => None,
    }
}

pub fn execute_clickhouse_query(config: &StorageConfig, query: &str) -> Result<()> {
    let upper = query.to_ascii_uppercase();
    let effective_query = if upper.starts_with("ALTER TABLE")
        && upper.contains(" DELETE ")
        && !upper.contains("MUTATIONS_SYNC")
    {
        format!("{} SETTINGS mutations_sync = 1", query)
    } else {
        query.to_string()
    };
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(&effective_query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(String::new())
        .send()
        .context("failed to execute ClickHouse query")?;
    if !response.status().is_success() {
        anyhow::bail!("ClickHouse query failed with status {}", response.status());
    }
    Ok(())
}

pub fn init_clickhouse(config: &StorageConfig) -> Result<()> {
    let sql = read_sql_file("sql/clickhouse/001_init.sql")?;
    for statement in split_sql_statements(&sql) {
        execute_clickhouse_query(config, &statement)?;
    }
    ensure_environment_snapshot_table(config)?;
    ensure_strategy_preference_scope_columns(config)?;
    ensure_signal_snapshot_provenance_columns(config)?;
    ensure_backtest_run_provenance_columns(config)?;
    Ok(())
}

pub fn init_storage(config: &StorageConfig) -> Result<()> {
    init_sqlite(config)?;
    init_clickhouse(config)?;
    Ok(())
}

pub fn insert_instruments(config: &StorageConfig, instruments: &[Instrument]) -> Result<()> {
    if instruments.is_empty() {
        return Ok(());
    }
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.instrument ADD COLUMN IF NOT EXISTS display_symbol Nullable(String) AFTER name",
    )?;
    execute_clickhouse_query(config, "TRUNCATE TABLE quant.instrument")?;

    let payload = instruments
        .iter()
        .map(|instrument| {
            serde_json::json!({
                "symbol": instrument.symbol,
                "name": instrument.name,
                "display_symbol": instrument.display_symbol,
                "instrument_type": match instrument.instrument_type { core_domain::InstrumentType::Index => "INDEX", core_domain::InstrumentType::Etf => "ETF" },
                "market": match instrument.market { core_domain::Market::Cn => "CN", core_domain::Market::Hk => "HK" },
                "category": instrument.category,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.instrument FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert instruments")?;
    if !response.status().is_success() {
        anyhow::bail!("instrument insert failed with status {}", response.status());
    }
    Ok(())
}

pub fn insert_daily_bars(config: &StorageConfig, symbol: &str, bars: &[DailyBar]) -> Result<()> {
    if bars.is_empty() {
        return Ok(());
    }
    let min_date = bars
        .iter()
        .map(|bar| bar.date)
        .min()
        .context("missing min date")?;
    let max_date = bars
        .iter()
        .map(|bar| bar.date)
        .max()
        .context("missing max date")?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.daily_bar DELETE WHERE symbol = '{}' AND date BETWEEN '{}' AND '{}'",
            escape_sql_string(symbol),
            min_date,
            max_date
        ),
    )?;

    let payload = bars
        .iter()
        .map(|bar| {
            serde_json::json!({
                "date": bar.date.to_string(),
                "symbol": bar.symbol,
                "open": bar.open,
                "high": bar.high,
                "low": bar.low,
                "close": bar.close,
                "volume": bar.volume,
                "turnover": bar.turnover,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.daily_bar FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert daily bars")?;
    if !response.status().is_success() {
        anyhow::bail!("daily bar insert failed with status {}", response.status());
    }
    Ok(())
}

pub fn fetch_daily_bars(config: &StorageConfig, symbol: &str) -> Result<Vec<DailyBar>> {
    let query = format!(
        "SELECT date,symbol,open,high,low,close,volume,turnover FROM quant.daily_bar WHERE symbol = '{}' ORDER BY date FORMAT JSONEachRow",
        escape_sql_string(symbol)
    );
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(&query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(String::new())
        .send()
        .context("failed to fetch daily bars")?;
    if !response.status().is_success() {
        anyhow::bail!("daily bar fetch failed with status {}", response.status());
    }
    let body = response
        .text()
        .context("failed to read daily bar response")?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        rows.push(serde_json::from_str::<DailyBar>(line).context("failed to parse daily bar row")?);
    }
    Ok(rows)
}

pub fn fetch_daily_bars_for_symbols_in_range(
    config: &StorageConfig,
    symbols: &[String],
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<DailyBar>> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let query = format!(
        "SELECT date,symbol,open,high,low,close,volume,turnover FROM quant.daily_bar WHERE symbol IN ({}) AND date BETWEEN '{}' AND '{}' ORDER BY symbol,date FORMAT JSONEachRow",
        encode_symbol_list(symbols),
        from,
        to
    );
    let body = fetch_clickhouse_text(config, &query)?;
    parse_json_each_row(&body, "failed to parse daily bar row")
}

pub fn fetch_latest_daily_bar_date(config: &StorageConfig) -> Result<Option<NaiveDate>> {
    let body = fetch_clickhouse_text(
        config,
        "SELECT max(date) AS max_date FROM quant.daily_bar FORMAT JSONEachRow",
    )?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse latest daily bar date row")?;
    let Some(text) = row.get("max_date").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    Ok(Some(NaiveDate::parse_from_str(text, "%Y-%m-%d")?))
}

pub fn insert_indicator_snapshots(
    config: &StorageConfig,
    symbol: &str,
    snapshots: &[IndicatorSnapshot],
) -> Result<()> {
    if snapshots.is_empty() {
        return Ok(());
    }
    let min_date = snapshots
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min indicator date")?;
    let max_date = snapshots
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max indicator date")?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.indicator_snapshot DELETE WHERE symbol = '{}' AND date BETWEEN '{}' AND '{}'",
            escape_sql_string(symbol),
            min_date,
            max_date
        ),
    )?;

    let payload = snapshots
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "symbol": row.symbol,
                "ma10": row.ma10,
                "ma20": row.ma20,
                "ma30": row.ma30,
                "ma60": row.ma60,
                "ma120": row.ma120,
                "ema12": row.ema12,
                "ema26": row.ema26,
                "macd": row.macd,
                "macd_signal": row.macd_signal,
                "macd_hist": row.macd_hist,
                "rsi14": row.rsi14,
                "atr14": row.atr14,
                "vol_ma20": row.vol_ma20,
                "vol_ma60": row.vol_ma60,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.indicator_snapshot FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert indicator snapshots")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "indicator snapshot insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn insert_macro_snapshots(config: &StorageConfig, rows: &[MacroSnapshot]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min macro date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max macro date")?;
    let factors = rows
        .iter()
        .map(|row| row.factor_name.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|factor| format!("'{}'", escape_sql_string(&factor)))
        .collect::<Vec<_>>()
        .join(",");
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.macro_snapshot DELETE WHERE factor_name IN ({}) AND date BETWEEN '{}' AND '{}'",
            factors, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "factor_name": row.factor_name,
                "factor_value": row.factor_value,
                "factor_score": row.factor_score,
                "factor_source": row.factor_source,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.macro_snapshot FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert macro snapshots")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "macro snapshot insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn fetch_macro_snapshots_in_range(
    config: &StorageConfig,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<MacroSnapshot>> {
    let query = format!(
        "SELECT date,factor_name,factor_value,factor_score,factor_source FROM quant.macro_snapshot WHERE date BETWEEN '{}' AND '{}' ORDER BY factor_name,date FORMAT JSONEachRow",
        from, to
    );
    let body = fetch_clickhouse_text(config, &query)?;
    parse_json_each_row(&body, "failed to parse macro snapshot row")
}

pub fn insert_market_regimes(config: &StorageConfig, rows: &[MarketRegimeSnapshot]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.market_regime ADD COLUMN IF NOT EXISTS macro_as_of_date Date DEFAULT date AFTER date",
    )?;
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min regime date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max regime date")?;
    let scopes = rows
        .iter()
        .map(|row| row.market.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|scope| format!("'{}'", escape_sql_string(&scope)))
        .collect::<Vec<_>>()
        .join(",");
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.market_regime DELETE WHERE market IN ({}) AND date BETWEEN '{}' AND '{}'",
            scopes, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "macro_as_of_date": row.macro_as_of_date.to_string(),
                "market": row.market,
                "trend_score": row.trend_score,
                "liquidity_score": row.liquidity_score,
                "risk_score": row.risk_score,
                "regime_label": row.regime_label,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.market_regime FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert market regimes")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "market regime insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn insert_environment_snapshots(
    config: &StorageConfig,
    rows: &[EnvironmentSnapshot],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    ensure_environment_snapshot_table(config)?;
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min environment date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max environment date")?;
    let scopes = rows
        .iter()
        .map(|row| row.scope.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|scope| format!("'{}'", escape_sql_string(&scope)))
        .collect::<Vec<_>>()
        .join(",");
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.environment_snapshot DELETE WHERE scope IN ({}) AND date BETWEEN '{}' AND '{}'",
            scopes, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "scope": row.scope,
                "regime_as_of_date": row.regime_as_of_date.to_string(),
                "breadth_as_of_date": row.breadth_as_of_date.to_string(),
                "stress_as_of_date": row.stress_as_of_date.to_string(),
                "breadth_eligible_count": row.breadth_eligible_count,
                "breadth_above_count": row.breadth_above_count,
                "breadth_pct": row.breadth_pct,
                "breadth_pct_sma5": row.breadth_pct_sma5,
                "breadth_5d_delta": row.breadth_5d_delta,
                "breadth_state": row.breadth_state,
                "volume_expansion_pct": row.volume_expansion_pct,
                "turnover_coverage_pct": row.turnover_coverage_pct,
                "liquidity_proxy_score": row.liquidity_proxy_score,
                "stress_proxy_score": row.stress_proxy_score,
                "environment_score": row.environment_score,
                "environment_label": row.environment_label,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.environment_snapshot FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert environment snapshots")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "environment snapshot insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn insert_strategy_states(
    config: &StorageConfig,
    rows: &[StrategyStateSnapshot],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    ensure_strategy_state_table(config)?;
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min strategy state date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max strategy state date")?;
    let scopes = rows
        .iter()
        .map(|row| row.scope.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|scope| format!("'{}'", escape_sql_string(&scope)))
        .collect::<Vec<_>>()
        .join(",");
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.strategy_state DELETE WHERE scope IN ({}) AND date BETWEEN '{}' AND '{}'",
            scopes, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "scope": row.scope,
                "state": row.state.as_str(),
                "state_score": row.state_score,
                "transition_reason": row.transition_reason,
                "recommended_position_pct": row.recommended_position_pct,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.strategy_state FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert strategy states")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "strategy state insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn fetch_latest_strategy_state_on_or_before(
    config: &StorageConfig,
    report_date: NaiveDate,
    scope: AnalysisScope,
) -> Result<Option<StrategyStateSnapshot>> {
    ensure_strategy_state_table(config)?;
    let query = format!(
        "SELECT date,scope,state,state_score,transition_reason,recommended_position_pct FROM quant.strategy_state WHERE scope = '{}' AND date <= '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
        scope.as_str(), report_date
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    Ok(Some(
        serde_json::from_str::<StrategyStateSnapshot>(line)
            .context("failed to parse strategy state row")?,
    ))
}

pub fn fetch_strategy_states_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Vec<StrategyStateSnapshot>> {
    ensure_strategy_state_table(config)?;
    let query = format!(
        "SELECT date,scope,state,state_score,transition_reason,recommended_position_pct FROM quant.strategy_state WHERE scope = '{}' ORDER BY date FORMAT JSONEachRow",
        escape_sql_string(scope.as_str())
    );
    let body = fetch_clickhouse_text(config, &query)?;
    parse_json_each_row::<StrategyStateSnapshot>(&body, "failed to parse strategy state row")
}

pub fn fetch_latest_strategy_state_date_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<NaiveDate>> {
    fetch_max_date_for_table_with_filter(config, "strategy_state", "scope", scope.as_str())
}

pub fn insert_rotation_ranks(config: &StorageConfig, rows: &[RotationRankSnapshot]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min rotation date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max rotation date")?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.rotation_rank DELETE WHERE date BETWEEN '{}' AND '{}'",
            min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "symbol": row.symbol,
                "rs_20": row.rs_20,
                "rs_60": row.rs_60,
                "rs_120": row.rs_120,
                "momentum_score": row.momentum_score,
                "rank": row.rank,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.rotation_rank FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert rotation ranks")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "rotation rank insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn fetch_indicator_snapshots(
    config: &StorageConfig,
    symbol: &str,
) -> Result<Vec<IndicatorSnapshot>> {
    let query = format!(
        "SELECT date,symbol,ma10,ma20,ma30,ma60,ma120,ema12,ema26,macd,macd_signal,macd_hist,rsi14,atr14,vol_ma20,vol_ma60 FROM quant.indicator_snapshot WHERE symbol = '{}' ORDER BY date FORMAT JSONEachRow",
        escape_sql_string(symbol)
    );
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(&query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(String::new())
        .send()
        .context("failed to fetch indicator snapshots")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "indicator snapshot fetch failed with status {}",
            response.status()
        );
    }
    let body = response
        .text()
        .context("failed to read indicator snapshot response")?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        rows.push(
            serde_json::from_str::<IndicatorSnapshot>(line)
                .context("failed to parse indicator snapshot row")?,
        );
    }
    Ok(rows)
}

pub fn fetch_indicator_snapshots_for_symbols_in_range(
    config: &StorageConfig,
    symbols: &[String],
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<IndicatorSnapshot>> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let query = format!(
        "SELECT date,symbol,ma10,ma20,ma30,ma60,ma120,ema12,ema26,macd,macd_signal,macd_hist,rsi14,atr14,vol_ma20,vol_ma60 FROM quant.indicator_snapshot WHERE symbol IN ({}) AND date BETWEEN '{}' AND '{}' ORDER BY symbol,date FORMAT JSONEachRow",
        encode_symbol_list(symbols),
        from,
        to
    );
    let body = fetch_clickhouse_text(config, &query)?;
    parse_json_each_row(&body, "failed to parse indicator snapshot row")
}

pub fn fetch_market_regimes(config: &StorageConfig) -> Result<Vec<MarketRegimeSnapshot>> {
    let query = "SELECT date,macro_as_of_date,market,trend_score,liquidity_score,risk_score,regime_label FROM quant.market_regime WHERE market = 'GLOBAL' ORDER BY date FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(String::new())
        .send()
        .context("failed to fetch market regimes")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "market regime fetch failed with status {}",
            response.status()
        );
    }
    let body = response
        .text()
        .context("failed to read market regime response")?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        rows.push(
            serde_json::from_str::<MarketRegimeSnapshot>(line)
                .context("failed to parse market regime row")?,
        );
    }
    Ok(rows)
}

pub fn fetch_latest_market_regime_on_or_before(
    config: &StorageConfig,
    report_date: NaiveDate,
    scope: AnalysisScope,
) -> Result<Option<MarketRegimeSnapshot>> {
    let query = format!(
        "SELECT date,macro_as_of_date,market,trend_score,liquidity_score,risk_score,regime_label FROM quant.market_regime WHERE market = '{}' AND date <= '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
        scope.as_str(), report_date
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    Ok(Some(
        serde_json::from_str::<MarketRegimeSnapshot>(line)
            .context("failed to parse market regime row")?,
    ))
}

pub fn fetch_latest_market_regime_date_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<NaiveDate>> {
    fetch_max_date_for_table_with_filter(config, "market_regime", "market", scope.as_str())
}

pub fn fetch_latest_environment_on_or_before(
    config: &StorageConfig,
    report_date: NaiveDate,
    scope: AnalysisScope,
) -> Result<Option<EnvironmentSnapshot>> {
    ensure_environment_snapshot_table(config)?;
    let query = format!(
        "SELECT date,scope,regime_as_of_date,breadth_as_of_date,stress_as_of_date,breadth_eligible_count,breadth_above_count,breadth_pct,breadth_pct_sma5,breadth_5d_delta,breadth_state,volume_expansion_pct,turnover_coverage_pct,liquidity_proxy_score,stress_proxy_score,environment_score,environment_label FROM quant.environment_snapshot WHERE scope = '{}' AND date <= '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
        scope.as_str(), report_date
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    Ok(Some(
        serde_json::from_str::<EnvironmentSnapshot>(line)
            .context("failed to parse environment snapshot row")?,
    ))
}

pub fn fetch_latest_environment_date_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<NaiveDate>> {
    ensure_environment_snapshot_table(config)?;
    fetch_max_date_for_table_with_filter(config, "environment_snapshot", "scope", scope.as_str())
}

pub fn fetch_dashboard_available_dates(config: &StorageConfig) -> Result<Vec<NaiveDate>> {
    ensure_environment_snapshot_table(config)?;
    let query = "SELECT DISTINCT date FROM quant.signal_snapshot WHERE date IN (SELECT DISTINCT date FROM quant.rotation_rank) AND date >= greatest((SELECT min(date) FROM quant.market_regime WHERE market = 'GLOBAL'), (SELECT min(date) FROM quant.environment_snapshot WHERE scope = 'GLOBAL')) ORDER BY date DESC FORMAT JSONEachRow";
    let body = fetch_clickhouse_text(config, query)?;
    let mut dates = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse dashboard date row")?;
        if let Some(text) = row.get("date").and_then(|value| value.as_str()) {
            dates.push(NaiveDate::parse_from_str(text, "%Y-%m-%d")?);
        }
    }
    Ok(dates)
}

pub fn fetch_latest_table_date(
    config: &StorageConfig,
    table_name: &str,
) -> Result<Option<NaiveDate>> {
    fetch_max_date_for_table(config, table_name)
}

pub fn fetch_distinct_entity_count_for_date(
    config: &StorageConfig,
    table_name: &str,
    entity_column: &str,
    date: NaiveDate,
) -> Result<usize> {
    let query = format!(
        "SELECT count(DISTINCT {entity_column}) AS entities FROM quant.{table_name} WHERE date = '{date}' FORMAT JSONEachRow"
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(0);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse distinct entity count row")?;
    Ok(json_u64(row.get("entities")).unwrap_or(0) as usize)
}

pub fn fetch_distinct_entity_count_for_date_in_symbols(
    config: &StorageConfig,
    table_name: &str,
    entity_column: &str,
    symbols: &[String],
    date: NaiveDate,
) -> Result<usize> {
    if symbols.is_empty() {
        return Ok(0);
    }
    let query = format!(
        "SELECT count(DISTINCT {entity_column}) AS entities FROM quant.{table_name} WHERE date = '{date}' AND symbol IN ({}) FORMAT JSONEachRow",
        encode_symbol_list(symbols)
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(0);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse scoped distinct entity count row")?;
    Ok(json_u64(row.get("entities")).unwrap_or(0) as usize)
}

pub fn fetch_rotation_ranks(config: &StorageConfig) -> Result<Vec<RotationRankSnapshot>> {
    let query = "SELECT date,symbol,rs_20,rs_60,rs_120,momentum_score,rank FROM quant.rotation_rank ORDER BY date,symbol FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(String::new())
        .send()
        .context("failed to fetch rotation ranks")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "rotation rank fetch failed with status {}",
            response.status()
        );
    }
    let body = response
        .text()
        .context("failed to read rotation rank response")?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        rows.push(
            serde_json::from_str::<RotationRankSnapshot>(line)
                .context("failed to parse rotation rank row")?,
        );
    }
    Ok(rows)
}

pub fn fetch_rotation_ranks_for_date(
    config: &StorageConfig,
    report_date: NaiveDate,
) -> Result<Vec<RotationRankSnapshot>> {
    let query = format!(
        "SELECT date,symbol,rs_20,rs_60,rs_120,momentum_score,rank FROM quant.rotation_rank WHERE date = '{}' ORDER BY rank,symbol FORMAT JSONEachRow",
        report_date
    );
    let body = fetch_clickhouse_text(config, &query)?;
    parse_json_each_row(&body, "failed to parse rotation rank row")
}

pub fn insert_strategy_preferences(
    config: &StorageConfig,
    rows: &[StrategyPreferenceSnapshot],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    ensure_strategy_preference_scope_columns(config)?;
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min strategy date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max strategy date")?;
    let scopes = rows
        .iter()
        .map(|row| row.analysis_scope.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|scope| format!("'{}'", escape_sql_string(&scope)))
        .collect::<Vec<_>>()
        .join(",");
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.strategy_preference DELETE WHERE analysis_scope IN ({}) AND date BETWEEN '{}' AND '{}'",
            scopes, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "symbol": row.symbol,
                "analysis_scope": row.analysis_scope,
                "regime_basis_scope": row.regime_basis_scope,
                "value_left_score": row.value_left_score,
                "trend_pullback_score": row.trend_pullback_score,
                "trend_breakout_score": row.trend_breakout_score,
                "momentum_right_score": row.momentum_right_score,
                "best_strategy": match row.best_strategy {
                    core_domain::StrategyKind::ValueLeft => "VALUE_LEFT",
                    core_domain::StrategyKind::TrendPullback => "TREND_PULLBACK",
                    core_domain::StrategyKind::TrendBreakout => "TREND_BREAKOUT",
                    core_domain::StrategyKind::MomentumRight => "MOMENTUM_RIGHT",
                },
                "confidence": row.confidence,
                "alignment": row.alignment,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.strategy_preference FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert strategy preferences")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "strategy preference insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn fetch_latest_backtest_run(
    config: &StorageConfig,
) -> Result<Option<backtest_engine::BacktestSummary>> {
    ensure_backtest_run_provenance_columns(config)?;
    let query = "SELECT run_id,strategy_name,analysis_scope,signal_scope,regime_basis_scope,signal_start_date,signal_end_date,config_summary,drawdown_events,state_trajectory_json,cagr,max_drawdown,sharpe FROM quant.backtest_run ORDER BY started_at DESC LIMIT 1 FORMAT JSONEachRow";
    let body = fetch_clickhouse_text(config, query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse latest backtest run row")?;
    let run_id = row
        .get("run_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let final_equity = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT equity FROM quant.backtest_equity_curve WHERE run_id = '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json.get("equity").and_then(|value| value.as_f64()))
    .unwrap_or(0.0);
    let trades = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT count() AS trades FROM quant.backtest_trade WHERE run_id = '{}' FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json_u64(json.get("trades")))
    .unwrap_or(0) as usize;
    let trading_days = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT count() AS points FROM quant.backtest_equity_curve WHERE run_id = '{}' FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json_u64(json.get("points")))
    .unwrap_or(0)
    .saturating_sub(1) as usize;

    Ok(Some(backtest_engine::BacktestSummary {
        run_id,
        strategy_name: row
            .get("strategy_name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        analysis_scope: row
            .get("analysis_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        signal_scope: row
            .get("signal_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        regime_basis_scope: row
            .get("regime_basis_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        signal_start_date: row
            .get("signal_start_date")
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        signal_end_date: row
            .get("signal_end_date")
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        config_summary: row
            .get("config_summary")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        cagr: row
            .get("cagr")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        max_drawdown: row
            .get("max_drawdown")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        sharpe: row
            .get("sharpe")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        final_equity,
        trades,
        trading_days,
        drawdown_events: json_u64(row.get("drawdown_events")).unwrap_or(0) as usize,
        state_trajectory: parse_state_trajectory(row.get("state_trajectory_json")),
    }))
}

pub fn fetch_latest_backtest_run_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<backtest_engine::BacktestSummary>> {
    ensure_backtest_run_provenance_columns(config)?;
    let query = format!(
        "SELECT run_id,strategy_name,analysis_scope,signal_scope,regime_basis_scope,signal_start_date,signal_end_date,config_summary,drawdown_events,state_trajectory_json,cagr,max_drawdown,sharpe FROM quant.backtest_run WHERE analysis_scope = '{}' ORDER BY started_at DESC LIMIT 1 FORMAT JSONEachRow",
        scope.as_str()
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse scoped latest backtest run row")?;
    let run_id = row
        .get("run_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let final_equity = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT equity FROM quant.backtest_equity_curve WHERE run_id = '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json.get("equity").and_then(|value| value.as_f64()))
    .unwrap_or(0.0);
    let trades = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT count() AS trades FROM quant.backtest_trade WHERE run_id = '{}' FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json_u64(json.get("trades")))
    .unwrap_or(0) as usize;
    let trading_days = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT count() AS points FROM quant.backtest_equity_curve WHERE run_id = '{}' FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json_u64(json.get("points")))
    .unwrap_or(0)
    .saturating_sub(1) as usize;

    Ok(Some(backtest_engine::BacktestSummary {
        run_id,
        strategy_name: row
            .get("strategy_name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        analysis_scope: row
            .get("analysis_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        signal_scope: row
            .get("signal_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        regime_basis_scope: row
            .get("regime_basis_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        signal_start_date: row
            .get("signal_start_date")
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        signal_end_date: row
            .get("signal_end_date")
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        config_summary: row
            .get("config_summary")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        cagr: row
            .get("cagr")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        max_drawdown: row
            .get("max_drawdown")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        sharpe: row
            .get("sharpe")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        final_equity,
        trades,
        trading_days,
        drawdown_events: json_u64(row.get("drawdown_events")).unwrap_or(0) as usize,
        state_trajectory: parse_state_trajectory(row.get("state_trajectory_json")),
    }))
}

pub fn fetch_latest_strategy_preference_date_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<NaiveDate>> {
    ensure_strategy_preference_scope_columns(config)?;
    fetch_max_date_for_table_with_filter(
        config,
        "strategy_preference",
        "analysis_scope",
        scope.as_str(),
    )
}

pub fn fetch_latest_signal_snapshot_date_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<NaiveDate>> {
    ensure_signal_snapshot_provenance_columns(config)?;
    fetch_max_date_for_table_with_filter(
        config,
        "signal_snapshot",
        "analysis_scope",
        scope.as_str(),
    )
}

pub fn insert_report_snapshot(
    config: &StorageConfig,
    report_date: &str,
    report_type: &str,
    artifact_path: &str,
) -> Result<()> {
    let payload = serde_json::to_string(&serde_json::json!({
        "report_date": report_date,
        "report_type": report_type,
        "artifact_path": artifact_path,
    }))?;
    let query = "INSERT INTO quant.report_snapshot FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert report snapshot")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "report snapshot insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn fetch_recent_report_snapshots(
    config: &StorageConfig,
    limit: usize,
) -> Result<Vec<(String, String, String)>> {
    let query = format!(
        "SELECT report_type,report_date,artifact_path FROM quant.report_snapshot ORDER BY generated_at DESC LIMIT {} FORMAT JSONEachRow",
        limit
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse report snapshot row")?;
        rows.push((
            row.get("report_type")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            row.get("report_date")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            row.get("artifact_path")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
        ));
    }
    Ok(rows)
}

pub fn fetch_strategy_preferences(
    config: &StorageConfig,
) -> Result<Vec<StrategyPreferenceSnapshot>> {
    ensure_strategy_preference_scope_columns(config)?;
    let query = "SELECT date,symbol,analysis_scope,regime_basis_scope,value_left_score,trend_pullback_score,trend_breakout_score,momentum_right_score,best_strategy,confidence,alignment FROM quant.strategy_preference ORDER BY analysis_scope,date,symbol FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(String::new())
        .send()
        .context("failed to fetch strategy preferences")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "strategy preference fetch failed with status {}",
            response.status()
        );
    }
    let body = response
        .text()
        .context("failed to read strategy preference response")?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let mut row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse strategy preference row")?;
        if let Some(best_strategy) = row.get("best_strategy").and_then(|value| value.as_str()) {
            row["best_strategy"] = match best_strategy {
                "VALUE_LEFT" => serde_json::json!("ValueLeft"),
                "TREND_PULLBACK" => serde_json::json!("TrendPullback"),
                "TREND_BREAKOUT" => serde_json::json!("TrendBreakout"),
                "MOMENTUM_RIGHT" => serde_json::json!("MomentumRight"),
                _ => serde_json::json!("ValueLeft"),
            };
        }
        if row.get("analysis_scope").is_none() {
            row["analysis_scope"] = serde_json::json!("GLOBAL");
        }
        if row.get("regime_basis_scope").is_none() {
            row["regime_basis_scope"] = serde_json::json!("GLOBAL");
        }
        rows.push(
            serde_json::from_value::<StrategyPreferenceSnapshot>(row)
                .context("failed to decode strategy preference snapshot")?,
        );
    }
    Ok(rows)
}

pub fn insert_signal_snapshots(config: &StorageConfig, rows: &[SignalSnapshot]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    ensure_signal_snapshot_provenance_columns(config)?;
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min signal date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max signal date")?;
    let scopes = rows
        .iter()
        .map(|row| row.analysis_scope.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|scope| format!("'{}'", escape_sql_string(&scope)))
        .collect::<Vec<_>>()
        .join(",");
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.signal_snapshot DELETE WHERE analysis_scope IN ({}) AND date BETWEEN '{}' AND '{}'",
            scopes, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| -> Result<String> {
            let reason_json = serde_json::to_string(&row.reason)?;
            Ok(serde_json::to_string(&serde_json::json!({
                "date": row.date.to_string(),
                "symbol": row.symbol,
                "final_score": row.final_score,
                "signal_label": match row.signal_label {
                    core_domain::SignalLabel::StrongBuy => "STRONG_BUY",
                    core_domain::SignalLabel::Buy => "BUY",
                    core_domain::SignalLabel::Watch => "WATCH",
                    core_domain::SignalLabel::Hold => "HOLD",
                    core_domain::SignalLabel::Reduce => "REDUCE",
                    core_domain::SignalLabel::Sell => "SELL",
                },
                "analysis_scope": row.analysis_scope,
                "regime_basis_scope": row.regime_basis_scope,
                "explanation": reason_json,
            }))?)
        })
        .collect::<Result<Vec<_>>>()?
        .join("\n");

    let query = "INSERT INTO quant.signal_snapshot FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(payload)
        .send()
        .context("failed to insert signal snapshots")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "signal snapshot insert failed with status {}",
            response.status()
        );
    }
    Ok(())
}

pub fn fetch_signal_snapshots(config: &StorageConfig) -> Result<Vec<SignalSnapshot>> {
    ensure_signal_snapshot_provenance_columns(config)?;
    let query = "SELECT date,symbol,final_score,signal_label,analysis_scope,regime_basis_scope,explanation FROM quant.signal_snapshot ORDER BY date,symbol FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let response = clickhouse_client()
        .post(url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(String::new())
        .send()
        .context("failed to fetch signal snapshots")?;
    if !response.status().is_success() {
        anyhow::bail!(
            "signal snapshot fetch failed with status {}",
            response.status()
        );
    }
    let body = response
        .text()
        .context("failed to read signal snapshot response")?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse signal snapshot row")?;
        rows.push(decode_signal_snapshot_row(row)?);
    }
    Ok(rows)
}

pub fn fetch_signal_snapshots_with_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Vec<SignalSnapshot>> {
    ensure_signal_snapshot_provenance_columns(config)?;
    let query = format!(
        "SELECT date,symbol,final_score,signal_label,analysis_scope,regime_basis_scope,explanation FROM quant.signal_snapshot WHERE analysis_scope = '{}' ORDER BY date,symbol FORMAT JSONEachRow",
        scope.as_str()
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse scoped signal snapshot row")?;
        rows.push(decode_signal_snapshot_row(row)?);
    }
    Ok(rows)
}

pub fn fetch_signal_snapshots_for_date(
    config: &StorageConfig,
    report_date: NaiveDate,
) -> Result<Vec<SignalSnapshot>> {
    ensure_signal_snapshot_provenance_columns(config)?;
    let query = format!(
        "SELECT date,symbol,final_score,signal_label,analysis_scope,regime_basis_scope,explanation FROM quant.signal_snapshot WHERE date = '{}' ORDER BY final_score DESC,symbol FORMAT JSONEachRow",
        report_date
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse signal snapshot row")?;
        rows.push(decode_signal_snapshot_row(row)?);
    }
    Ok(rows)
}

pub fn fetch_signal_snapshots_for_date_with_scope(
    config: &StorageConfig,
    report_date: NaiveDate,
    scope: AnalysisScope,
) -> Result<Vec<SignalSnapshot>> {
    ensure_signal_snapshot_provenance_columns(config)?;
    let query = format!(
        "SELECT date,symbol,final_score,signal_label,analysis_scope,regime_basis_scope,explanation FROM quant.signal_snapshot WHERE date = '{}' AND analysis_scope = '{}' ORDER BY final_score DESC,symbol FORMAT JSONEachRow",
        report_date,
        scope.as_str()
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse scoped signal snapshot row")?;
        rows.push(decode_signal_snapshot_row(row)?);
    }
    Ok(rows)
}

pub fn fetch_signal_snapshot_for_symbol(
    config: &StorageConfig,
    date: NaiveDate,
    symbol: &str,
    scope: AnalysisScope,
) -> Result<Option<SignalSnapshot>> {
    ensure_signal_snapshot_provenance_columns(config)?;
    let query = format!(
        "SELECT date,symbol,final_score,signal_label,analysis_scope,regime_basis_scope,explanation FROM quant.signal_snapshot WHERE date = '{}' AND symbol = '{}' AND analysis_scope = '{}' LIMIT 1 FORMAT JSONEachRow",
        date,
        escape_sql_string(symbol),
        scope.as_str()
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse signal snapshot row")?;
    Ok(Some(decode_signal_snapshot_row(row)?))
}

pub fn insert_backtest_result(
    config: &StorageConfig,
    summary: &BacktestSummary,
    trades: &[BacktestTrade],
    equity_curve: &[BacktestEquityPoint],
) -> Result<()> {
    ensure_backtest_run_provenance_columns(config)?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.backtest_run DELETE WHERE run_id = '{}'",
            escape_sql_string(&summary.run_id)
        ),
    )?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.backtest_trade DELETE WHERE run_id = '{}'",
            escape_sql_string(&summary.run_id)
        ),
    )?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.backtest_equity_curve DELETE WHERE run_id = '{}'",
            escape_sql_string(&summary.run_id)
        ),
    )?;

    let state_trajectory_json = serde_json::to_string(&summary.state_trajectory)?;
    let run_payload = serde_json::to_string(&serde_json::json!({
        "run_id": summary.run_id,
        "strategy_name": summary.strategy_name,
        "analysis_scope": summary.analysis_scope,
        "signal_scope": summary.signal_scope,
        "regime_basis_scope": summary.regime_basis_scope,
        "signal_start_date": summary.signal_start_date.map(|date| date.to_string()),
        "signal_end_date": summary.signal_end_date.map(|date| date.to_string()),
        "config_summary": summary.config_summary,
        "started_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "finished_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "drawdown_events": summary.drawdown_events,
        "state_trajectory_json": state_trajectory_json,
        "cagr": summary.cagr,
        "max_drawdown": summary.max_drawdown,
        "sharpe": summary.sharpe,
    }))?;
    let run_query = "INSERT INTO quant.backtest_run FORMAT JSONEachRow";
    let run_url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(run_query)
    );
    let run_response = clickhouse_client()
        .post(run_url)
        .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
        .body(run_payload)
        .send()
        .context("failed to insert backtest run")?;
    if !run_response.status().is_success() {
        anyhow::bail!(
            "backtest run insert failed with status {}",
            run_response.status()
        );
    }

    if !trades.is_empty() {
        let payload = trades
            .iter()
            .map(|row| {
                serde_json::json!({
                    "run_id": row.run_id,
                    "trade_date": row.trade_date.to_string(),
                    "symbol": row.symbol,
                    "action": row.action,
                    "price": row.price,
                    "quantity": row.quantity,
                    "trade_value": row.trade_value,
                })
            })
            .map(|row| serde_json::to_string(&row))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n");
        let query = "INSERT INTO quant.backtest_trade FORMAT JSONEachRow";
        let url = format!(
            "{}?database={}&query={}",
            config.clickhouse_url,
            config.clickhouse_database,
            urlencoding::encode(query)
        );
        let response = clickhouse_client()
            .post(url)
            .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
            .body(payload)
            .send()
            .context("failed to insert backtest trades")?;
        if !response.status().is_success() {
            anyhow::bail!(
                "backtest trade insert failed with status {}",
                response.status()
            );
        }
    }

    if !equity_curve.is_empty() {
        let payload = equity_curve
            .iter()
            .map(|row| {
                serde_json::json!({
                    "run_id": row.run_id,
                    "date": row.date.to_string(),
                    "equity": row.equity,
                    "drawdown": row.drawdown,
                })
            })
            .map(|row| serde_json::to_string(&row))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n");
        let query = "INSERT INTO quant.backtest_equity_curve FORMAT JSONEachRow";
        let url = format!(
            "{}?database={}&query={}",
            config.clickhouse_url,
            config.clickhouse_database,
            urlencoding::encode(query)
        );
        let response = clickhouse_client()
            .post(url)
            .basic_auth(&config.clickhouse_user, Some(&config.clickhouse_password))
            .body(payload)
            .send()
            .context("failed to insert backtest equity curve")?;
        if !response.status().is_success() {
            anyhow::bail!(
                "backtest equity insert failed with status {}",
                response.status()
            );
        }
    }

    Ok(())
}

pub fn ping_clickhouse(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(config, "SELECT 1")
}

pub fn date_bounds(bars: &[DailyBar]) -> Option<(NaiveDate, NaiveDate)> {
    Some((
        bars.iter().map(|bar| bar.date).min()?,
        bars.iter().map(|bar| bar.date).max()?,
    ))
}

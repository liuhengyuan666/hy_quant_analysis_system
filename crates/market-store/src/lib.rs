use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use backtest_engine::{BacktestEquityPoint, BacktestSummary, BacktestTrade};
use chrono::NaiveDate;
use core_domain::{
    DailyBar, IndicatorSnapshot, Instrument, MacroSnapshot, MarketRegimeSnapshot,
    RotationRankSnapshot, SignalSnapshot, StrategyPreferenceSnapshot,
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

fn fetch_max_date_for_table(config: &StorageConfig, table_name: &str) -> Result<Option<NaiveDate>> {
    let query = format!(
        "SELECT max(date) AS max_date FROM quant.{} FORMAT JSONEachRow",
        table_name
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse max date row")?;
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
    serde_json::from_value::<SignalSnapshot>(row).context("failed to decode signal snapshot")
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
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.macro_snapshot DELETE WHERE date BETWEEN '{}' AND '{}'",
            min_date, max_date
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
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.market_regime DELETE WHERE market = 'GLOBAL' AND date BETWEEN '{}' AND '{}'",
            min_date, max_date
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
    let query = "SELECT date,macro_as_of_date,market,trend_score,liquidity_score,risk_score,regime_label FROM quant.market_regime ORDER BY date FORMAT JSONEachRow";
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
) -> Result<Option<MarketRegimeSnapshot>> {
    let query = format!(
        "SELECT date,macro_as_of_date,market,trend_score,liquidity_score,risk_score,regime_label FROM quant.market_regime WHERE date <= '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
        report_date
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

pub fn fetch_dashboard_available_dates(config: &StorageConfig) -> Result<Vec<NaiveDate>> {
    let query = "SELECT DISTINCT date FROM quant.signal_snapshot WHERE date IN (SELECT DISTINCT date FROM quant.rotation_rank) AND date >= (SELECT min(date) FROM quant.market_regime) ORDER BY date DESC FORMAT JSONEachRow";
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
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.strategy_preference DELETE WHERE date BETWEEN '{}' AND '{}'",
            min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "symbol": row.symbol,
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
    let query = "SELECT run_id,strategy_name,cagr,max_drawdown,sharpe FROM quant.backtest_run ORDER BY started_at DESC LIMIT 1 FORMAT JSONEachRow";
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
    }))
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
    let query = "SELECT date,symbol,value_left_score,trend_pullback_score,trend_breakout_score,momentum_right_score,best_strategy,confidence,alignment FROM quant.strategy_preference ORDER BY date,symbol FORMAT JSONEachRow";
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
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.signal_snapshot DELETE WHERE date BETWEEN '{}' AND '{}'",
            min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
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
                "explanation": row.explanation,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
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
    let query = "SELECT date,symbol,final_score,signal_label,explanation FROM quant.signal_snapshot ORDER BY date,symbol FORMAT JSONEachRow";
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

pub fn fetch_signal_snapshots_for_date(
    config: &StorageConfig,
    report_date: NaiveDate,
) -> Result<Vec<SignalSnapshot>> {
    let query = format!(
        "SELECT date,symbol,final_score,signal_label,explanation FROM quant.signal_snapshot WHERE date = '{}' ORDER BY final_score DESC,symbol FORMAT JSONEachRow",
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

pub fn insert_backtest_result(
    config: &StorageConfig,
    summary: &BacktestSummary,
    trades: &[BacktestTrade],
    equity_curve: &[BacktestEquityPoint],
) -> Result<()> {
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

    let run_payload = serde_json::to_string(&serde_json::json!({
        "run_id": summary.run_id,
        "strategy_name": summary.strategy_name,
        "started_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "finished_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
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

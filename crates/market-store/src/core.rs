use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::DailyBar;
use ureq::Agent;
use base64::Engine;
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

pub fn escape_sql_string(value: &str) -> String {
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

pub fn sqlite_connection(config: &StorageConfig) -> Result<Connection> {
    let sqlite_path = config.sqlite_abspath()?;
    if let Some(parent) = sqlite_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create sqlite directory: {}", parent.display()))?;
    }
    let connection = Connection::open(&sqlite_path)
        .with_context(|| format!("failed to open sqlite database: {}", sqlite_path.display()))?;
    ensure_refresh_jobs_table(&connection)?;
    ensure_user_preferences_table(&connection)?;
    ensure_app_config_table(&connection)?;
    ensure_credential_store_table(&connection)?;
    Ok(connection)
}

pub fn ensure_refresh_jobs_table(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS refresh_jobs (
                id TEXT PRIMARY KEY,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                status TEXT NOT NULL,
                stages_json TEXT NOT NULL,
                last_successful_stage TEXT,
                error TEXT,
                refresh_from TEXT,
                refresh_to TEXT
            );",
        )
        .context("failed to ensure refresh_jobs table")?;
    Ok(())
}

pub fn ensure_user_preferences_table(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS user_preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .context("failed to ensure user_preferences table")?;
    Ok(())
}

pub fn ensure_app_config_table(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS app_config (
                config_key TEXT PRIMARY KEY,
                config_value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .context("failed to ensure app_config table")?;
    Ok(())
}

pub fn ensure_credential_store_table(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS credential_store (
                credential_key TEXT PRIMARY KEY,
                credential_value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .context("failed to ensure credential_store table")?;
    Ok(())
}

pub fn clickhouse_client() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_read(std::time::Duration::from_secs(30))
            .timeout_write(std::time::Duration::from_secs(30))
            .build()
    })
}

pub fn read_body(response: ureq::Response) -> Result<String> {
    let mut body = String::new();
    response
        .into_reader()
        .read_to_string(&mut body)
        .context("failed to read response body")?;
    Ok(body)
}

pub fn clickhouse_auth_header(user: &str, password: &str) -> String {
    let credentials = format!("{}:{}", user, password);
    format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(credentials))
}

pub fn parse_json_each_row<T: DeserializeOwned>(body: &str, row_context: &str) -> Result<Vec<T>> {
    body.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<T>(line).with_context(|| row_context.to_string()))
        .collect()
}

pub fn encode_symbol_list(symbols: &[String]) -> String {
    symbols
        .iter()
        .map(|symbol| format!("'{}'", escape_sql_string(symbol)))
        .collect::<Vec<_>>()
        .join(",")
}

pub fn ensure_environment_snapshot_table(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "CREATE TABLE IF NOT EXISTS quant.environment_snapshot (date Date,scope LowCardinality(String),regime_as_of_date Date,breadth_as_of_date Date,stress_as_of_date Date,breadth_eligible_count UInt32,breadth_above_count UInt32,breadth_pct Float64,breadth_pct_sma5 Nullable(Float64),breadth_5d_delta Nullable(Float64),breadth_state LowCardinality(String),volume_expansion_pct Nullable(Float64),turnover_coverage_pct Nullable(Float64),liquidity_proxy_score Float64,stress_proxy_score Float64,environment_score Float64,environment_label LowCardinality(String),updated_at DateTime DEFAULT now()) ENGINE = MergeTree PARTITION BY toYYYYMM(date) ORDER BY (scope, date)",
    )
}

pub fn ensure_strategy_state_table(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "CREATE TABLE IF NOT EXISTS quant.strategy_state (date Date,scope LowCardinality(String),state LowCardinality(String),state_score Float64,transition_reason String,recommended_position_pct Float64,updated_at DateTime DEFAULT now()) ENGINE = MergeTree PARTITION BY toYYYYMM(date) ORDER BY (scope, date)",
    )
}

pub fn ensure_strategy_preference_scope_columns(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.strategy_preference ADD COLUMN IF NOT EXISTS analysis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.strategy_preference ADD COLUMN IF NOT EXISTS regime_basis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )
}

pub fn ensure_signal_snapshot_provenance_columns(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.signal_snapshot ADD COLUMN IF NOT EXISTS analysis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.signal_snapshot ADD COLUMN IF NOT EXISTS regime_basis_scope LowCardinality(String) DEFAULT 'GLOBAL'",
    )
}

pub fn ensure_backtest_run_provenance_columns(config: &StorageConfig) -> Result<()> {
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
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS run_version String DEFAULT 'legacy'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS git_commit String DEFAULT 'unknown'",
    )?;
    execute_clickhouse_query(
        config,
        "ALTER TABLE quant.backtest_run ADD COLUMN IF NOT EXISTS generated_at String DEFAULT ''",
    )
}

pub fn fetch_max_date_for_table_with_filter(
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

pub fn fetch_max_date_for_table(config: &StorageConfig, table_name: &str) -> Result<Option<NaiveDate>> {
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

pub fn fetch_clickhouse_text(config: &StorageConfig, query: &str) -> Result<String> {
    let encoded = urlencoding::encode(query);
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url, config.clickhouse_database, encoded
    );
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let response = clickhouse_client()
        .post(&url)
        .set("Authorization", &auth)
        .send_string("")
        .context("failed to fetch ClickHouse text response")?;
    let status = response.status();
    if status >= 400 {
        let mut body = String::new();
        let _ = response.into_reader()
            .read_to_string(&mut body);
        if body.is_empty() {
            body = format!("HTTP {}", status);
        }
        anyhow::bail!("ClickHouse text query failed with status {}: {}", status, body);
    }
    let mut body = String::new();
    response.into_reader()
        .read_to_string(&mut body)
        .context("failed to read ClickHouse text response")?;
    Ok(body)
}

pub fn json_u64(value: Option<&serde_json::Value>) -> Option<u64> {
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
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let response = clickhouse_client()
        .post(&url)
        .set("Authorization", &auth)
        .send_string("")
        .context("failed to execute ClickHouse query")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!("ClickHouse query failed with status {}: {}", status, body);
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

pub fn ping_clickhouse(config: &StorageConfig) -> Result<()> {
    execute_clickhouse_query(config, "SELECT 1")
}

pub fn date_bounds(bars: &[DailyBar]) -> Option<(NaiveDate, NaiveDate)> {
    Some((
        bars.iter().map(|bar| bar.date).min()?,
        bars.iter().map(|bar| bar.date).max()?,
    ))
}

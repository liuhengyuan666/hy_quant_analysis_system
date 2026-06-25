use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::{AnalysisScope, MarketRegimeSnapshot};

use crate::core::*;

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

    let query = "INSERT INTO quant.market_regime SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let response = clickhouse_client()
        .post(&url)
        .set("Authorization", &auth)
        .send_string(&payload)
        .context("failed to insert market regimes")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "market regime insert failed with status {}: {}",
            status, body
        );
    }
    Ok(())
}

pub fn fetch_market_regimes(config: &StorageConfig) -> Result<Vec<MarketRegimeSnapshot>> {
    let query = "SELECT date,macro_as_of_date,market,trend_score,liquidity_score,risk_score,regime_label FROM quant.market_regime ORDER BY date FORMAT JSONEachRow";
    let url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(query)
    );
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let response = clickhouse_client()
        .post(&url)
        .set("Authorization", &auth)
        .send_string("")
        .context("failed to fetch market regimes")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "market regime fetch failed with status {}: {}",
            status, body
        );
    }
    let body = read_body(response)
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

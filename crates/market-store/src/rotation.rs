use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::RotationRankSnapshot;

use crate::core::*;

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

    let query = "INSERT INTO quant.rotation_rank SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert rotation ranks")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "rotation rank insert failed with status {}: {}",
            status, body
        );
    }
    Ok(())
}

pub fn fetch_rotation_ranks(config: &StorageConfig) -> Result<Vec<RotationRankSnapshot>> {
    let query = "SELECT date,symbol,rs_20,rs_60,rs_120,momentum_score,rank FROM quant.rotation_rank ORDER BY date,symbol FORMAT JSONEachRow";
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
        .context("failed to fetch rotation ranks")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "rotation rank fetch failed with status {}: {}",
            status, body
        );
    }
    let body = read_body(response)
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

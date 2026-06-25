use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::MacroSnapshot;

use crate::core::*;

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

    let query = "INSERT INTO quant.macro_snapshot SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert macro snapshots")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "macro snapshot insert failed with status {}: {}",
            status, body
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

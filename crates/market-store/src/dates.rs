use anyhow::{Context, Result};
use chrono::NaiveDate;

use crate::core::*;

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

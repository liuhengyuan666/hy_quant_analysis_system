use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::IndicatorSnapshot;

use crate::core::*;

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

    let query = "INSERT INTO quant.indicator_snapshot SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert indicator snapshots")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "indicator snapshot insert failed with status {}: {}",
            status, body
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
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let response = clickhouse_client()
        .post(&url)
        .set("Authorization", &auth)
        .send_string("")
        .context("failed to fetch indicator snapshots")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "indicator snapshot fetch failed with status {}: {}",
            status, body
        );
    }
    let body = read_body(response)
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

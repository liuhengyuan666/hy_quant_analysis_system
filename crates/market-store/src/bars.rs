use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::DailyBar;

use crate::core::*;

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

    let query = "INSERT INTO quant.daily_bar SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert daily bars")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!("daily bar insert failed with status {}: {}", status, body);
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
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let response = clickhouse_client()
        .post(&url)
        .set("Authorization", &auth)
        .send_string("")
        .context("failed to fetch daily bars")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!("daily bar fetch failed with status {}: {}", status, body);
    }
    let body = read_body(response)
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

use anyhow::{Context, Result};
use core_domain::Instrument;

use crate::core::*;

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
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let response = clickhouse_client()
        .post(&url)
        .set("Authorization", &auth)
        .send_string(&payload)
        .context("failed to insert instruments")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!("instrument insert failed with status {}: {}", status, body);
    }
    Ok(())
}

use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::{
    AnalysisScope, RegimeReason, RotationReason, SignalReason, SignalSnapshot, StrategyKind,
};

use crate::core::*;

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
    let trimmed_explanation = explanation.trim_start();
    row["reason"] = match serde_json::from_str::<SignalReason>(&explanation) {
        Ok(reason) => serde_json::to_value(reason)?,
        Err(error) if trimmed_explanation.starts_with('{') => {
            anyhow::bail!("failed to parse structured signal reason JSON: {error}")
        }
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

    let query = "INSERT INTO quant.signal_snapshot SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert signal snapshots")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "signal snapshot insert failed with status {}: {}",
            status, body
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
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let response = clickhouse_client()
        .post(&url)
        .set("Authorization", &auth)
        .send_string("")
        .context("failed to fetch signal snapshots")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "signal snapshot fetch failed with status {}: {}",
            status, body
        );
    }
    let body = read_body(response)
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

pub fn fetch_signal_snapshots_for_range_with_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<SignalSnapshot>> {
    ensure_signal_snapshot_provenance_columns(config)?;
    let query = format!(
        "SELECT date,symbol,final_score,signal_label,analysis_scope,regime_basis_scope,explanation FROM quant.signal_snapshot WHERE date BETWEEN '{}' AND '{}' AND analysis_scope = '{}' ORDER BY date, final_score DESC, symbol FORMAT JSONEachRow",
        from,
        to,
        scope.as_str()
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse ranged scoped signal snapshot row")?;
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

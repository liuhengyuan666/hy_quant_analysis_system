use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::{AnalysisScope, EnvironmentSnapshot};

use crate::core::*;

pub fn insert_environment_snapshots(
    config: &StorageConfig,
    rows: &[EnvironmentSnapshot],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    ensure_environment_snapshot_table(config)?;
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min environment date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max environment date")?;
    let scopes = rows
        .iter()
        .map(|row| row.scope.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|scope| format!("'{}'", escape_sql_string(&scope)))
        .collect::<Vec<_>>()
        .join(",");
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.environment_snapshot DELETE WHERE scope IN ({}) AND date BETWEEN '{}' AND '{}'",
            scopes, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "scope": row.scope,
                "regime_as_of_date": row.regime_as_of_date.to_string(),
                "breadth_as_of_date": row.breadth_as_of_date.to_string(),
                "stress_as_of_date": row.stress_as_of_date.to_string(),
                "breadth_eligible_count": row.breadth_eligible_count,
                "breadth_above_count": row.breadth_above_count,
                "breadth_pct": row.breadth_pct,
                "breadth_pct_sma5": row.breadth_pct_sma5,
                "breadth_5d_delta": row.breadth_5d_delta,
                "breadth_state": row.breadth_state,
                "volume_expansion_pct": row.volume_expansion_pct,
                "turnover_coverage_pct": row.turnover_coverage_pct,
                "liquidity_proxy_score": row.liquidity_proxy_score,
                "stress_proxy_score": row.stress_proxy_score,
                "environment_score": row.environment_score,
                "environment_label": row.environment_label,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.environment_snapshot SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert environment snapshots")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "environment snapshot insert failed with status {}: {}",
            status, body
        );
    }
    Ok(())
}

pub fn fetch_latest_environment_on_or_before(
    config: &StorageConfig,
    report_date: NaiveDate,
    scope: AnalysisScope,
) -> Result<Option<EnvironmentSnapshot>> {
    ensure_environment_snapshot_table(config)?;
    let query = format!(
        "SELECT date,scope,regime_as_of_date,breadth_as_of_date,stress_as_of_date,breadth_eligible_count,breadth_above_count,breadth_pct,breadth_pct_sma5,breadth_5d_delta,breadth_state,volume_expansion_pct,turnover_coverage_pct,liquidity_proxy_score,stress_proxy_score,environment_score,environment_label FROM quant.environment_snapshot WHERE scope = '{}' AND date <= '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
        scope.as_str(), report_date
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    Ok(Some(
        serde_json::from_str::<EnvironmentSnapshot>(line)
            .context("failed to parse environment snapshot row")?,
    ))
}

pub fn fetch_latest_environment_date_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<NaiveDate>> {
    ensure_environment_snapshot_table(config)?;
    fetch_max_date_for_table_with_filter(config, "environment_snapshot", "scope", scope.as_str())
}

/// Fetch all environment snapshots for a scope within a date range.
pub fn fetch_environment_snapshots_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<EnvironmentSnapshot>> {
    ensure_environment_snapshot_table(config)?;
    let query = format!(
        "SELECT date,scope,regime_as_of_date,breadth_as_of_date,stress_as_of_date,breadth_eligible_count,breadth_above_count,breadth_pct,breadth_pct_sma5,breadth_5d_delta,breadth_state,volume_expansion_pct,turnover_coverage_pct,liquidity_proxy_score,stress_proxy_score,environment_score,environment_label FROM quant.environment_snapshot WHERE scope = '{}' AND date BETWEEN '{}' AND '{}' ORDER BY date FORMAT JSONEachRow",
        scope.as_str(), from, to
    );
    let body = fetch_clickhouse_text(config, &query)?;
    parse_json_each_row(&body, "failed to parse environment snapshot row")
}

pub fn fetch_all_environment_snapshots(config: &StorageConfig) -> Result<Vec<EnvironmentSnapshot>> {
    ensure_environment_snapshot_table(config)?;
    let query = "SELECT date,scope,regime_as_of_date,breadth_as_of_date,stress_as_of_date,breadth_eligible_count,breadth_above_count,breadth_pct,breadth_pct_sma5,breadth_5d_delta,breadth_state,volume_expansion_pct,turnover_coverage_pct,liquidity_proxy_score,stress_proxy_score,environment_score,environment_label FROM quant.environment_snapshot ORDER BY date, scope FORMAT JSONEachRow";
    let body = fetch_clickhouse_text(config, query)?;
    parse_json_each_row(&body, "failed to parse environment snapshot row")
}

pub fn fetch_dashboard_available_dates(config: &StorageConfig) -> Result<Vec<NaiveDate>> {
    ensure_environment_snapshot_table(config)?;
    // 优化：使用 JOIN 替代 IN 子句，避免双表全扫描
    let query = r#"
        SELECT DISTINCT s.date
        FROM quant.signal_snapshot s
        INNER JOIN (
            SELECT DISTINCT date FROM quant.rotation_rank
        ) r ON s.date = r.date
        WHERE s.date >= greatest(
            (SELECT min(date) FROM quant.market_regime WHERE market = 'GLOBAL'),
            (SELECT min(date) FROM quant.environment_snapshot WHERE scope = 'GLOBAL')
        )
        ORDER BY s.date DESC
        FORMAT JSONEachRow
    "#;
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

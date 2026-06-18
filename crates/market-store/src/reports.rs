use anyhow::{Context, Result};

use crate::core::*;

pub fn insert_report_snapshot(
    config: &StorageConfig,
    report_date: &str,
    report_type: &str,
    artifact_path: &str,
) -> Result<()> {
    let payload = serde_json::to_string(&serde_json::json!({
        "report_date": report_date,
        "report_type": report_type,
        "artifact_path": artifact_path,
    }))?;
    let query = "INSERT INTO quant.report_snapshot SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert report snapshot")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "report snapshot insert failed with status {}: {}",
            status, body
        );
    }
    Ok(())
}

pub fn fetch_recent_report_snapshots(
    config: &StorageConfig,
    limit: usize,
) -> Result<Vec<(String, String, String)>> {
    let query = format!(
        "SELECT report_type,report_date,artifact_path FROM quant.report_snapshot ORDER BY generated_at DESC LIMIT {} FORMAT JSONEachRow",
        limit
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse report snapshot row")?;
        rows.push((
            row.get("report_type")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            row.get("report_date")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            row.get("artifact_path")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
        ));
    }
    Ok(rows)
}

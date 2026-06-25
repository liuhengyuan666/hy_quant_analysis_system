use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::{AnalysisScope, StrategyPreferenceSnapshot, StrategyStateSnapshot};

use crate::core::*;

pub fn insert_strategy_states(
    config: &StorageConfig,
    rows: &[StrategyStateSnapshot],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    ensure_strategy_state_table(config)?;
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min strategy state date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max strategy state date")?;
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
            "ALTER TABLE quant.strategy_state DELETE WHERE scope IN ({}) AND date BETWEEN '{}' AND '{}'",
            scopes, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "scope": row.scope,
                "state": row.state.as_str(),
                "state_score": row.state_score,
                "transition_reason": row.transition_reason,
                "recommended_position_pct": row.recommended_position_pct,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.strategy_state SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert strategy states")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "strategy state insert failed with status {}: {}",
            status, body
        );
    }
    Ok(())
}

pub fn fetch_latest_strategy_state_on_or_before(
    config: &StorageConfig,
    report_date: NaiveDate,
    scope: AnalysisScope,
) -> Result<Option<StrategyStateSnapshot>> {
    ensure_strategy_state_table(config)?;
    let query = format!(
        "SELECT date,scope,state,state_score,transition_reason,recommended_position_pct FROM quant.strategy_state WHERE scope = '{}' AND date <= '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
        scope.as_str(), report_date
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    Ok(Some(
        serde_json::from_str::<StrategyStateSnapshot>(line)
            .context("failed to parse strategy state row")?,
    ))
}

pub fn fetch_strategy_states_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Vec<StrategyStateSnapshot>> {
    ensure_strategy_state_table(config)?;
    let query = format!(
        "SELECT date,scope,state,state_score,transition_reason,recommended_position_pct FROM quant.strategy_state WHERE scope = '{}' ORDER BY date FORMAT JSONEachRow",
        escape_sql_string(scope.as_str())
    );
    let body = fetch_clickhouse_text(config, &query)?;
    parse_json_each_row::<StrategyStateSnapshot>(&body, "failed to parse strategy state row")
}

pub fn fetch_latest_strategy_state_date_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<NaiveDate>> {
    fetch_max_date_for_table_with_filter(config, "strategy_state", "scope", scope.as_str())
}

pub fn insert_strategy_preferences(
    config: &StorageConfig,
    rows: &[StrategyPreferenceSnapshot],
) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    ensure_strategy_preference_scope_columns(config)?;
    let min_date = rows
        .iter()
        .map(|row| row.date)
        .min()
        .context("missing min strategy date")?;
    let max_date = rows
        .iter()
        .map(|row| row.date)
        .max()
        .context("missing max strategy date")?;
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
            "ALTER TABLE quant.strategy_preference DELETE WHERE analysis_scope IN ({}) AND date BETWEEN '{}' AND '{}'",
            scopes, min_date, max_date
        ),
    )?;

    let payload = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "date": row.date.to_string(),
                "symbol": row.symbol,
                "analysis_scope": row.analysis_scope,
                "regime_basis_scope": row.regime_basis_scope,
                "value_left_score": row.value_left_score,
                "trend_pullback_score": row.trend_pullback_score,
                "trend_breakout_score": row.trend_breakout_score,
                "momentum_right_score": row.momentum_right_score,
                "best_strategy": match row.best_strategy {
                    core_domain::StrategyKind::ValueLeft => "VALUE_LEFT",
                    core_domain::StrategyKind::TrendPullback => "TREND_PULLBACK",
                    core_domain::StrategyKind::TrendBreakout => "TREND_BREAKOUT",
                    core_domain::StrategyKind::MomentumRight => "MOMENTUM_RIGHT",
                },
                "confidence": row.confidence,
                "alignment": row.alignment,
            })
        })
        .map(|row| serde_json::to_string(&row))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");

    let query = "INSERT INTO quant.strategy_preference SETTINGS max_partitions_per_insert_block=10000 FORMAT JSONEachRow";
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
        .context("failed to insert strategy preferences")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "strategy preference insert failed with status {}: {}",
            status, body
        );
    }
    Ok(())
}

pub fn fetch_strategy_preferences(
    config: &StorageConfig,
) -> Result<Vec<StrategyPreferenceSnapshot>> {
    ensure_strategy_preference_scope_columns(config)?;
    let query = "SELECT date,symbol,analysis_scope,regime_basis_scope,value_left_score,trend_pullback_score,trend_breakout_score,momentum_right_score,best_strategy,confidence,alignment FROM quant.strategy_preference ORDER BY analysis_scope,date,symbol FORMAT JSONEachRow";
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
        .context("failed to fetch strategy preferences")?;
    let status = response.status();
    if status >= 400 {
        let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
        anyhow::bail!(
            "strategy preference fetch failed with status {}: {}",
            status, body
        );
    }
    let body = read_body(response)
        .context("failed to read strategy preference response")?;
    let mut rows = Vec::new();
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        let mut row: serde_json::Value =
            serde_json::from_str(line).context("failed to parse strategy preference row")?;
        if let Some(best_strategy) = row.get("best_strategy").and_then(|value| value.as_str()) {
            row["best_strategy"] = match best_strategy {
                "VALUE_LEFT" => serde_json::json!("ValueLeft"),
                "TREND_PULLBACK" => serde_json::json!("TrendPullback"),
                "TREND_BREAKOUT" => serde_json::json!("TrendBreakout"),
                "MOMENTUM_RIGHT" => serde_json::json!("MomentumRight"),
                _ => serde_json::json!("ValueLeft"),
            };
        }
        if row.get("analysis_scope").is_none() {
            row["analysis_scope"] = serde_json::json!("GLOBAL");
        }
        if row.get("regime_basis_scope").is_none() {
            row["regime_basis_scope"] = serde_json::json!("GLOBAL");
        }
        rows.push(
            serde_json::from_value::<StrategyPreferenceSnapshot>(row)
                .context("failed to decode strategy preference snapshot")?,
        );
    }
    Ok(rows)
}

pub fn fetch_strategy_preference_for_symbol(
    config: &StorageConfig,
    date: NaiveDate,
    symbol: &str,
    scope: AnalysisScope,
) -> Result<Option<StrategyPreferenceSnapshot>> {
    ensure_strategy_preference_scope_columns(config)?;
    let query = format!(
        "SELECT date,symbol,analysis_scope,regime_basis_scope,value_left_score,trend_pullback_score,trend_breakout_score,momentum_right_score,best_strategy,confidence,alignment FROM quant.strategy_preference WHERE date = '{}' AND symbol = '{}' AND analysis_scope = '{}' LIMIT 1 FORMAT JSONEachRow",
        date,
        escape_sql_string(symbol),
        scope.as_str()
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let mut row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse strategy preference row")?;
    if let Some(best_strategy) = row.get("best_strategy").and_then(|value| value.as_str()) {
        row["best_strategy"] = match best_strategy {
            "VALUE_LEFT" => serde_json::json!("ValueLeft"),
            "TREND_PULLBACK" => serde_json::json!("TrendPullback"),
            "TREND_BREAKOUT" => serde_json::json!("TrendBreakout"),
            "MOMENTUM_RIGHT" => serde_json::json!("MomentumRight"),
            _ => serde_json::json!("ValueLeft"),
        };
    }
    if row.get("analysis_scope").is_none() {
        row["analysis_scope"] = serde_json::json!("GLOBAL");
    }
    if row.get("regime_basis_scope").is_none() {
        row["regime_basis_scope"] = serde_json::json!("GLOBAL");
    }
    Ok(Some(
        serde_json::from_value::<StrategyPreferenceSnapshot>(row)
            .context("failed to decode strategy preference snapshot")?,
    ))
}

pub fn fetch_latest_strategy_preference_date_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<NaiveDate>> {
    ensure_strategy_preference_scope_columns(config)?;
    fetch_max_date_for_table_with_filter(
        config,
        "strategy_preference",
        "analysis_scope",
        scope.as_str(),
    )
}

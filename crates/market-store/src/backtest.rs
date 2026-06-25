use anyhow::{Context, Result};
use backtest_engine::{BacktestEquityPoint, BacktestSummary, BacktestTrade};
use chrono::NaiveDate;
use core_domain::AnalysisScope;

use crate::core::*;

fn parse_state_trajectory(value: Option<&serde_json::Value>) -> Vec<(NaiveDate, String)> {
    value
        .and_then(|value| value.as_str())
        .filter(|text| !text.trim().is_empty())
        .and_then(|text| serde_json::from_str::<Vec<(NaiveDate, String)>>(text).ok())
        .unwrap_or_default()
}

pub fn fetch_latest_backtest_run(
    config: &StorageConfig,
) -> Result<Option<BacktestSummary>> {
    ensure_backtest_run_provenance_columns(config)?;
    let query = "SELECT run_id,strategy_name,analysis_scope,signal_scope,regime_basis_scope,signal_start_date,signal_end_date,config_summary,drawdown_events,state_trajectory_json,cagr,max_drawdown,sharpe,run_version,git_commit,generated_at FROM quant.backtest_run WHERE run_version = 'v1' ORDER BY started_at DESC LIMIT 1 FORMAT JSONEachRow";
    let body = fetch_clickhouse_text(config, query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse latest backtest run row")?;
    let run_id = row
        .get("run_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let final_equity = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT equity FROM quant.backtest_equity_curve WHERE run_id = '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json.get("equity").and_then(|value| value.as_f64()))
    .unwrap_or(0.0);
    let trades = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT count() AS trades FROM quant.backtest_trade WHERE run_id = '{}' FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json_u64(json.get("trades")))
    .unwrap_or(0) as usize;
    let trading_days = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT count() AS points FROM quant.backtest_equity_curve WHERE run_id = '{}' FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json_u64(json.get("points")))
    .unwrap_or(0)
    .saturating_sub(1) as usize;

    Ok(Some(BacktestSummary {
        run_id,
        strategy_name: row
            .get("strategy_name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        analysis_scope: row
            .get("analysis_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        signal_scope: row
            .get("signal_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        regime_basis_scope: row
            .get("regime_basis_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        signal_start_date: row
            .get("signal_start_date")
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        signal_end_date: row
            .get("signal_end_date")
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        config_summary: row
            .get("config_summary")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        cagr: row
            .get("cagr")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        max_drawdown: row
            .get("max_drawdown")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        sharpe: row
            .get("sharpe")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        final_equity,
        trades,
        trading_days,
        drawdown_events: json_u64(row.get("drawdown_events")).unwrap_or(0) as usize,
        state_trajectory: parse_state_trajectory(row.get("state_trajectory_json")),
        run_version: row
            .get("run_version")
            .and_then(|value| value.as_str())
            .unwrap_or("legacy")
            .to_string(),
        git_commit: row
            .get("git_commit")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        generated_at: row
            .get("generated_at")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    }))
}

pub fn fetch_latest_backtest_run_for_scope(
    config: &StorageConfig,
    scope: AnalysisScope,
) -> Result<Option<BacktestSummary>> {
    ensure_backtest_run_provenance_columns(config)?;
    let query = format!(
        "SELECT run_id,strategy_name,analysis_scope,signal_scope,regime_basis_scope,signal_start_date,signal_end_date,config_summary,drawdown_events,state_trajectory_json,cagr,max_drawdown,sharpe,run_version,git_commit,generated_at FROM quant.backtest_run WHERE analysis_scope = '{}' AND run_version = 'v1' ORDER BY started_at DESC LIMIT 1 FORMAT JSONEachRow",
        scope.as_str()
    );
    let body = fetch_clickhouse_text(config, &query)?;
    let Some(line) = body.lines().find(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let row: serde_json::Value =
        serde_json::from_str(line).context("failed to parse scoped latest backtest run row")?;
    let run_id = row
        .get("run_id")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let final_equity = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT equity FROM quant.backtest_equity_curve WHERE run_id = '{}' ORDER BY date DESC LIMIT 1 FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json.get("equity").and_then(|value| value.as_f64()))
    .unwrap_or(0.0);
    let trades = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT count() AS trades FROM quant.backtest_trade WHERE run_id = '{}' FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json_u64(json.get("trades")))
    .unwrap_or(0) as usize;
    let trading_days = fetch_clickhouse_text(
        config,
        &format!(
            "SELECT count() AS points FROM quant.backtest_equity_curve WHERE run_id = '{}' FORMAT JSONEachRow",
            escape_sql_string(&run_id)
        ),
    )?
    .lines()
    .find(|line| !line.trim().is_empty())
    .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
    .and_then(|json| json_u64(json.get("points")))
    .unwrap_or(0)
    .saturating_sub(1) as usize;

    Ok(Some(BacktestSummary {
        run_id,
        strategy_name: row
            .get("strategy_name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        analysis_scope: row
            .get("analysis_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        signal_scope: row
            .get("signal_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        regime_basis_scope: row
            .get("regime_basis_scope")
            .and_then(|value| value.as_str())
            .unwrap_or("GLOBAL")
            .to_string(),
        signal_start_date: row
            .get("signal_start_date")
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        signal_end_date: row
            .get("signal_end_date")
            .and_then(|value| value.as_str())
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        config_summary: row
            .get("config_summary")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        cagr: row
            .get("cagr")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        max_drawdown: row
            .get("max_drawdown")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        sharpe: row
            .get("sharpe")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0),
        final_equity,
        trades,
        trading_days,
        drawdown_events: json_u64(row.get("drawdown_events")).unwrap_or(0) as usize,
        state_trajectory: parse_state_trajectory(row.get("state_trajectory_json")),
        run_version: row
            .get("run_version")
            .and_then(|value| value.as_str())
            .unwrap_or("legacy")
            .to_string(),
        git_commit: row
            .get("git_commit")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        generated_at: row
            .get("generated_at")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string(),
    }))
}

pub fn insert_backtest_result(
    config: &StorageConfig,
    summary: &BacktestSummary,
    trades: &[BacktestTrade],
    equity_curve: &[BacktestEquityPoint],
) -> Result<()> {
    ensure_backtest_run_provenance_columns(config)?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.backtest_run DELETE WHERE run_id = '{}'",
            escape_sql_string(&summary.run_id)
        ),
    )?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.backtest_trade DELETE WHERE run_id = '{}'",
            escape_sql_string(&summary.run_id)
        ),
    )?;
    execute_clickhouse_query(
        config,
        &format!(
            "ALTER TABLE quant.backtest_equity_curve DELETE WHERE run_id = '{}'",
            escape_sql_string(&summary.run_id)
        ),
    )?;

    let state_trajectory_json = serde_json::to_string(&summary.state_trajectory)?;
    let run_payload = serde_json::to_string(&serde_json::json!({
        "run_id": summary.run_id,
        "strategy_name": summary.strategy_name,
        "analysis_scope": summary.analysis_scope,
        "signal_scope": summary.signal_scope,
        "regime_basis_scope": summary.regime_basis_scope,
        "signal_start_date": summary.signal_start_date.map(|date| date.to_string()),
        "signal_end_date": summary.signal_end_date.map(|date| date.to_string()),
        "config_summary": summary.config_summary,
        "started_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "finished_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "drawdown_events": summary.drawdown_events,
        "state_trajectory_json": state_trajectory_json,
        "run_version": summary.run_version,
        "git_commit": summary.git_commit,
        "generated_at": summary.generated_at,
        "cagr": summary.cagr,
        "max_drawdown": summary.max_drawdown,
        "sharpe": summary.sharpe,
    }))?;
    let run_query = "INSERT INTO quant.backtest_run FORMAT JSONEachRow";
    let run_url = format!(
        "{}?database={}&query={}",
        config.clickhouse_url,
        config.clickhouse_database,
        urlencoding::encode(run_query)
    );
    let auth = clickhouse_auth_header(&config.clickhouse_user, &config.clickhouse_password);
    let run_response = clickhouse_client()
        .post(&run_url)
        .set("Authorization", &auth)
        .send_string(&run_payload)
        .context("failed to insert backtest run")?;
    let run_status = run_response.status();
    if run_status >= 400 {
        let body = read_body(run_response).unwrap_or_else(|_| format!("HTTP {}", run_status));
        anyhow::bail!(
            "backtest run insert failed with status {}: {}",
            run_status, body
        );
    }

    if !trades.is_empty() {
        let payload = trades
            .iter()
            .map(|row| {
                serde_json::json!({
                    "run_id": row.run_id,
                    "trade_date": row.trade_date.to_string(),
                    "symbol": row.symbol,
                    "action": row.action,
                    "price": row.price,
                    "quantity": row.quantity,
                    "trade_value": row.trade_value,
                })
            })
            .map(|row| serde_json::to_string(&row))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n");
        let query = "INSERT INTO quant.backtest_trade FORMAT JSONEachRow";
        let url = format!(
            "{}?database={}&query={}",
            config.clickhouse_url,
            config.clickhouse_database,
            urlencoding::encode(query)
        );
        let response = clickhouse_client()
            .post(&url)
            .set("Authorization", &auth)
            .send_string(&payload)
            .context("failed to insert backtest trades")?;
        let status = response.status();
        if status >= 400 {
            let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
            anyhow::bail!(
                "backtest trade insert failed with status {}: {}",
                status, body
            );
        }
    }

    if !equity_curve.is_empty() {
        let payload = equity_curve
            .iter()
            .map(|row| {
                serde_json::json!({
                    "run_id": row.run_id,
                    "date": row.date.to_string(),
                    "equity": row.equity,
                    "drawdown": row.drawdown,
                })
            })
            .map(|row| serde_json::to_string(&row))
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n");
        let query = "INSERT INTO quant.backtest_equity_curve FORMAT JSONEachRow";
        let url = format!(
            "{}?database={}&query={}&max_partitions_per_insert_block=10000",
            config.clickhouse_url,
            config.clickhouse_database,
            urlencoding::encode(query)
        );
        let response = clickhouse_client()
            .post(&url)
            .set("Authorization", &auth)
            .send_string(&payload)
            .context("failed to insert backtest equity curve")?;
        let status = response.status();
        if status >= 400 {
            let body = read_body(response).unwrap_or_else(|_| format!("HTTP {}", status));
            anyhow::bail!(
                "backtest equity insert failed with status {}: {}",
                status, body
            );
        }
    }

    Ok(())
}

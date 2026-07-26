use anyhow::Result;
use app_service::{pipeline_stages, AppContext};
use chrono::{Local, NaiveDate};
use crate::ReportScopeArg;

pub fn stage_label(stage: &str) -> String {
    let total = pipeline_stages::ALL.len();
    match pipeline_stages::ALL.iter().position(|&s| s == stage) {
        Some(idx) => format!("[{}/{}] {}", idx + 1, total, stage),
        None => stage.to_string(),
    }
}

pub fn handle_ingest_daily(context: &AppContext, from: NaiveDate, to: NaiveDate, quiet: bool) -> Result<()> {
    let progress_fn = |msg: &str| eprintln!("[ingest] {}", msg);
    let result = if quiet {
        context.ingest_daily(from, to, None)?
    } else {
        context.ingest_daily(from, to, Some(&progress_fn))?
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_compute_indicators(context: &AppContext, quiet: bool) -> Result<()> {
    let label = stage_label(pipeline_stages::STAGE_INDICATORS);
    let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
    let result = if quiet {
        context.compute_indicators(None)?
    } else {
        context.compute_indicators(Some(&progress_fn))?
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_compute_macro(context: &AppContext, from: NaiveDate, to: NaiveDate, quiet: bool) -> Result<()> {
    let label = stage_label(pipeline_stages::STAGE_MACRO);
    let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
    let result = if quiet {
        context.compute_macro_regime(from, to, None)?
    } else {
        context.compute_macro_regime(from, to, Some(&progress_fn))?
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_compute_rotation(context: &AppContext, quiet: bool) -> Result<()> {
    let label = stage_label(pipeline_stages::STAGE_ROTATION);
    let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
    let result = if quiet {
        context.compute_rotation(None)?
    } else {
        context.compute_rotation(Some(&progress_fn))?
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_compute_strategy_preferences(context: &AppContext, quiet: bool) -> Result<()> {
    let label = stage_label(pipeline_stages::STAGE_STRATEGY);
    let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
    let result = if quiet {
        context.compute_strategy_preferences(None)?
    } else {
        context.compute_strategy_preferences(Some(&progress_fn))?
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_compute_signals(context: &AppContext, quiet: bool) -> Result<()> {
    let label = stage_label(pipeline_stages::STAGE_SIGNALS);
    let progress_fn = |msg: &str| eprintln!("{}: {}", label, msg);
    let result = if quiet {
        context.compute_signals(None)?
    } else {
        context.compute_signals(Some(&progress_fn))?
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_daily_analysis(
    context: &AppContext,
    scope: crate::ReportScopeArg,
    quiet: bool,
) -> Result<()> {
    let scope: app_service::ReportScope = scope.into();
    if !quiet {
        eprintln!("[daily-analysis] Running daily analysis for scope {:?}...", scope);
    }

    // Step 0: Context Integrity Gate — check data health first
    let integrity = context.check_data_health()?;
    let integrity_status = if integrity.freshest_market_date_complete {
        "PASS"
    } else {
        "DEGRADED"
    };

    // Step 1: Dashboard snapshot for current state
    let snapshot = context.dashboard_snapshot_with_scope(None, scope)?;

    let (date, bullish_count, top_bullish) = match &snapshot {
        Some(snap) => {
            let bullish = &snap.bullish_signals;
            let top: Vec<serde_json::Value> = bullish.iter().take(5).map(|s| {
                serde_json::json!({
                    "symbol": s.symbol,
                    "final_score": s.final_score,
                    "signal_label": format!("{:?}", s.signal_label),
                })
            }).collect();
            (snap.report_date.to_string(), bullish.len(), top)
        }
        None => ("no data".to_string(), 0, vec![]),
    };

    // Step 2: Portfolio action
    let decisions = context.analyze_preclose(scope).unwrap_or_default();
    let portfolio_actions: Vec<serde_json::Value> = decisions.iter().map(|d| {
        serde_json::json!({
            "symbol": d.symbol,
            "action": d.state.as_str(),
            "reasons": d.reasons.iter().map(|r| r.as_str()).collect::<Vec<_>>(),
        })
    }).collect();

    // Output
    let output = serde_json::json!({
        "integrity": integrity_status,
        "scope": format!("{:?}", scope),
        "date": date,
        "signals": {
            "bullish_count": bullish_count,
            "top_bullish": top_bullish,
        },
        "portfolio_actions": portfolio_actions,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn handle_refresh_all(
    context: &AppContext,
    to: Option<NaiveDate>,
    scope: ReportScopeArg,
    run_backtests: bool,
    quiet: bool,
) -> Result<()> {
    let progress_callback: Option<Box<dyn Fn(&str) + Send>> = if quiet {
        None
    } else {
        Some(Box::new(|msg: &str| {
            eprintln!("[refresh] {}", msg);
        }))
    };
    let result = context.refresh_pipeline(
        to.unwrap_or_else(|| Local::now().date_naive()),
        scope.into(),
        run_backtests,
        None,
        None,
        progress_callback,
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    if !result.success {
        std::process::exit(1);
    }
    Ok(())
}

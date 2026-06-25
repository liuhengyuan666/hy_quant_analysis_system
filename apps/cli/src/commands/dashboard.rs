use anyhow::Result;
use app_service::AppContext;
use chrono::{Local, NaiveDate};
use crate::ReportScopeArg;

pub fn handle_dashboard_snapshot(context: &AppContext, date: Option<NaiveDate>, scope: ReportScopeArg) -> Result<()> {
    let result = context.dashboard_snapshot_with_scope(date, scope.into())?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_dashboard_dates(context: &AppContext, scope: ReportScopeArg) -> Result<()> {
    let result = context.dashboard_available_dates_with_scope(scope.into())?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_export_report(context: &AppContext, date: Option<NaiveDate>, scope: ReportScopeArg, concise: bool) -> Result<()> {
    let result = context.export_report_with_scope_and_format(date, scope.into(), concise)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_export_data_health_report(context: &AppContext) -> Result<()> {
    let result = context.export_data_health_report()?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_sync_and_export(
    context: &AppContext,
    date: Option<NaiveDate>,
    scope: ReportScopeArg,
    to: Option<NaiveDate>,
    run_backtests: bool,
    quiet: bool,
) -> Result<()> {
    if !quiet {
        eprintln!("[sync-and-export] Starting...");
    }
    let progress_callback: Option<Box<dyn Fn(&str) + Send>> = if quiet {
        None
    } else {
        Some(Box::new(|msg: &str| {
            eprintln!("[sync-and-export] {}", msg);
        }))
    };
    let result = context.sync_and_export(
        date,
        to.unwrap_or_else(|| Local::now().date_naive()),
        scope.into(),
        run_backtests,
        progress_callback,
    )?;
    if !quiet {
        eprintln!("[sync-and-export] Done.");
    }
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_research_context(context: &AppContext, scope: ReportScopeArg) -> Result<()> {
    let result = context.research_context(scope.into())?;
    let features = context.research_features(scope.into())?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "context": result,
        "features": features,
    }))?);
    Ok(())
}

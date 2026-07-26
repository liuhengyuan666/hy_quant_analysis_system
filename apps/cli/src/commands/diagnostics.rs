use anyhow::Result;
use app_service::AppContext;
use crate::ReportScopeArg;

pub fn handle_explain_latest_gate(context: &AppContext, scope: ReportScopeArg) -> Result<()> {
    let result = context.explain_latest_gate(scope.into())?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_pipeline_dates(context: &AppContext, scope: ReportScopeArg) -> Result<()> {
    let result = context.pipeline_date_diagnostics_with_scope(scope.into())?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// RV1: superseded by `data-health` (merged check+export); internal library only.
#[allow(dead_code)]
pub fn handle_check_data_health(context: &AppContext) -> Result<()> {
    let result = context.check_data_health()?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// V7 Workflow — Data Health: run provider/gap/jump diagnostics and export the
/// report in one step, combining the old `check-data-health` and
/// `export-data-health-report` primitives.
pub fn handle_data_health(context: &AppContext) -> Result<()> {
    let check = context.check_data_health()?;
    let export = context.export_data_health_report()?;

    let combined = serde_json::json!({
        "check": check,
        "export": export,
    });

    println!("{}", serde_json::to_string_pretty(&combined)?);
    Ok(())
}

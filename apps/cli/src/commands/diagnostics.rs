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

pub fn handle_check_data_health(context: &AppContext) -> Result<()> {
    let result = context.check_data_health()?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

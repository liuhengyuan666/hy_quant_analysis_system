use anyhow::{Context, Result};
use app_service::AppContext;
use chrono::NaiveDate;
use crate::ReportScopeArg;

pub fn handle_set_llm_config(
    context: &AppContext,
    base_url: String,
    model: String,
    timeout_secs: u64,
) -> Result<()> {
    context.set_llm_config(&base_url, &model, timeout_secs)?;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "status": "ok",
        "base_url": base_url,
        "model": model,
        "timeout_secs": timeout_secs,
    }))?);
    Ok(())
}

pub fn handle_set_llm_api_key(context: &AppContext, key: String) -> Result<()> {
    context.set_llm_api_key(&key)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "message": "LLM API key stored successfully"
        }))?
    );
    Ok(())
}

pub fn handle_analyze_with_llm(context: &AppContext, scope: ReportScopeArg, date: Option<NaiveDate>, quiet: bool) -> Result<()> {
    eprintln!("WARNING: 'analyze-with-llm' is deprecated. Use 'analyze --skill <name>' instead.");
    eprintln!("Example: cargo run -p quant-cli -- analyze --scope global --skill market-regime-reasoning");
    eprintln!();
    let report_date = match date {
        Some(d) => d,
        None => {
            let dates =
                context.dashboard_available_dates_with_scope(scope.into())?;
            let latest = dates.first().context(
                "no dashboard dates available; run refresh-all first",
            )?;
            NaiveDate::parse_from_str(latest, "%Y-%m-%d")
                .context("failed to parse latest dashboard date")?
        }
    };
    if !quiet {
        eprintln!("[analyze-with-llm] Analyzing report for {report_date}...");
    }
    let result = context.analyze_report_with_llm(report_date, scope.into())?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_show_llm_config(context: &AppContext, validate: bool) -> Result<()> {
    let resolved = context.show_llm_config()?;

    let mut output = serde_json::json!({
        "base_url": resolved.base_url,
        "model": resolved.model,
        "timeout_secs": resolved.timeout_secs,
        "temperature": resolved.temperature,
        "max_tokens": resolved.max_tokens,
        "api_key_set": resolved.api_key.is_some(),
        "source": {
            "base_url": resolved.source.base_url,
            "model": resolved.source.model,
            "api_key": resolved.source.api_key,
            "config_file": resolved.source.config_file,
        }
    });

    if let Some(seed) = resolved.seed {
        output["seed"] = serde_json::json!(seed);
    }

    if validate {
        let validation = context.validate_llm_config();
        output["validation"] = serde_json::json!({
            "file_exists": validation.file_exists,
            "file_parseable": validation.file_parseable,
            "env_vars_resolved": validation.env_vars_resolved,
            "missing_env_vars": validation.missing_env_vars,
            "url_format_valid": validation.url_format_valid,
            "api_key_set": validation.api_key_set,
        });
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn handle_set_fred_config(context: &AppContext, enabled: bool, disabled: bool, api_key: Option<String>) -> Result<()> {
    let final_enabled = if disabled { false } else { enabled };
    context.set_fred_config(final_enabled, api_key.as_deref())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "message": "FRED config updated successfully",
            "enabled": final_enabled,
            "api_key_set": api_key.is_some(),
        }))?
    );
    Ok(())
}

pub fn handle_show_fred_config(context: &AppContext) -> Result<()> {
    let resolved = context.show_fred_config()?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "enabled": resolved.enabled,
            "base_url": resolved.base_url,
            "api_key_set": resolved.api_key.is_some(),
            "config_file": resolved.config_file,
            "request_delay_ms": resolved.request_delay_ms,
            "timeout_secs": resolved.timeout_secs,
            "valid": resolved.is_valid(),
        }))?
    );
    Ok(())
}

pub fn handle_migrate_llm_config(context: &AppContext, force: bool) -> Result<()> {
    let result = context.migrate_llm_config_to_toml(force)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "message": result,
        }))?
    );
    Ok(())
}

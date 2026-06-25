use anyhow::Result;
use app_service::AppContext;
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

pub fn handle_analyze_with_llm(_context: &AppContext, scope: ReportScopeArg, action: String, quiet: bool) -> Result<()> {
    if !quiet {
        eprintln!("[analyze-with-llm] Running research action '{}'...", action);
    }
    let result = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        let context = AppContext::new(market_store::StorageConfig::default());
        runtime.block_on(context.analyze_with_action(&action, scope.into()))
    })
    .join()
    .expect("LLM analysis thread panicked")?;
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
    });

    if let Some(seed) = resolved.seed {
        output["seed"] = serde_json::json!(seed);
    }

    if validate {
        let status = if resolved.api_key.is_some() {
            "ok"
        } else {
            "warning: api_key not set"
        };
        output["validation"] = serde_json::json!(status);
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn handle_migrate_llm_config(context: &AppContext, force: bool) -> Result<()> {
    let result = context.migrate_llm_config_to_toml(force)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_set_fred_config(context: &AppContext, enabled: bool, _disabled: bool, api_key: Option<String>) -> Result<()> {
    let result = context.set_fred_config(enabled, api_key.as_deref())?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub fn handle_show_fred_config(_context: &AppContext) -> Result<()> {
    println!("show-fred-config not yet implemented");
    Ok(())
}

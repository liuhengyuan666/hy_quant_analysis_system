use anyhow::Result;
use core_domain::LlmConfig;

/// Render LLM analysis JSON as markdown report.
pub(crate) fn render_llm_analysis_markdown(analysis: &serde_json::Value) -> String {
    let mut md = String::new();

    // Title
    let skill = analysis["skill"].as_str().unwrap_or("unknown");
    let scope = analysis["scope"].as_str().unwrap_or("global");
    md.push_str(&format!("# LLM Analysis: {}\n\n", skill));
    md.push_str(&format!("**Scope**: {}\n\n", scope));

    // Triggered status
    let triggered = analysis["triggered"].as_bool().unwrap_or(false);
    md.push_str(&format!(
        "**Triggered**: {}\n\n",
        if triggered { "Yes" } else { "No" }
    ));

    // Placeholder warning
    if analysis["placeholder"].as_bool().unwrap_or(false) {
        md.push_str(
            "> **Warning**: This analysis was generated in placeholder mode. \
             No real LLM provider was configured.\n\n",
        );
    }

    // Regime Analysis
    if let Some(regime) = analysis["regime_analysis"].as_object() {
        md.push_str("## Regime Analysis\n\n");
        if let Some(state) = regime.get("current_state").and_then(|v| v.as_str()) {
            md.push_str(&format!("- **Current State**: {}\n", state));
        }
        if let Some(transition) = regime.get("transition").and_then(|v| v.as_f64()) {
            md.push_str(&format!("- **Transition Score**: {:.2}\n", transition));
        }
        if let Some(confidence) = regime.get("confidence").and_then(|v| v.as_f64()) {
            md.push_str(&format!("- **Confidence**: {:.1}%\n", confidence * 100.0));
        }
        if let Some(drivers) = regime.get("key_drivers").and_then(|v| v.as_array()) {
            if !drivers.is_empty() {
                md.push_str("- **Key Drivers**:\n");
                for d in drivers {
                    if let Some(s) = d.as_str() {
                        md.push_str(&format!("  - {}\n", s));
                    }
                }
            }
        }
        if let Some(risk) = regime.get("risk_assessment") {
            if let Some(level) = risk.get("level").and_then(|v| v.as_str()) {
                md.push_str(&format!("- **Risk Level**: {}\n", level));
            }
            if let Some(factors) = risk.get("factors").and_then(|v| v.as_array()) {
                if !factors.is_empty() {
                    md.push_str("- **Risk Factors**:\n");
                    for f in factors {
                        if let Some(s) = f.as_str() {
                            md.push_str(&format!("  - {}\n", s));
                        }
                    }
                }
            }
            if let Some(rec) = risk.get("recommendation").and_then(|v| v.as_str()) {
                md.push_str(&format!("- **Recommendation**: {}\n", rec));
            }
        }
        md.push('\n');
    }

    // LLM Analysis
    if let Some(llm) = analysis["llm_analysis"].as_str() {
        if !llm.is_empty() {
            md.push_str("## LLM Analysis\n\n");
            md.push_str(llm);
            md.push_str("\n\n");
        }
    }

    // Token Usage
    if let Some(tokens) = analysis["token_usage"].as_object() {
        md.push_str("## Token Usage\n\n");
        if let Some(input) = tokens.get("system_tokens").and_then(|v| v.as_u64()) {
            md.push_str(&format!("- **System Tokens**: {}\n", input));
        }
        if let Some(input) = tokens.get("context_tokens").and_then(|v| v.as_u64()) {
            md.push_str(&format!("- **Context Tokens**: {}\n", input));
        }
        if let Some(input) = tokens.get("reasoning_tokens").and_then(|v| v.as_u64()) {
            md.push_str(&format!("- **Reasoning Tokens**: {}\n", input));
        }
        if let Some(output) = tokens.get("output_tokens").and_then(|v| v.as_u64()) {
            md.push_str(&format!("- **Output Tokens**: {}\n", output));
        }
        md.push('\n');
    }

    md
}

pub(crate) async fn call_llm_api(
    config: LlmConfig,
    api_key: String,
    system_prompt: &'static str,
    user_prompt: String,
    temperature: f64,
    max_tokens: usize,
    seed: Option<u64>,
) -> Result<String> {
    let openai_config = async_openai::config::OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(config.base_url);
    let client = async_openai::Client::with_config(openai_config);
    let mut request_builder =
        async_openai::types::chat::CreateChatCompletionRequestArgs::default();
    request_builder
        .model(&config.model)
        .temperature(temperature as f32)
        .max_tokens(max_tokens as u32)
        .messages([
            async_openai::types::chat::ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()
                .map_err(|e| anyhow::anyhow!("failed to build system message: {e}"))?
                .into(),
            async_openai::types::chat::ChatCompletionRequestUserMessageArgs::default()
                .content(&*user_prompt)
                .build()
                .map_err(|e| anyhow::anyhow!("failed to build user message: {e}"))?
                .into(),
        ]);
    if let Some(seed_val) = seed {
        request_builder.seed(seed_val as i64);
    }
    let request = request_builder
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build chat completion request: {e}"))?;

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(config.timeout_secs),
        client.chat().create(request),
    )
    .await
    .map_err(|_| anyhow::anyhow!("LLM API call timed out after {}s", config.timeout_secs))?
    .map_err(|e| anyhow::anyhow!("LLM API call failed: {e}"))?;

    let content = response
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .unwrap_or_default();

    Ok(content)
}

/// Extract key drivers from context
#[allow(dead_code)]
pub(crate) fn extract_key_drivers(context: &research_context::ResearchContext) -> Vec<String> {
    let mut drivers = Vec::new();

    if context.breadth.breadth_pct < 30.0 {
        drivers.push("breadth_collapse".to_string());
    }
    if context.breadth.breadth_delta < -10.0 {
        drivers.push("breadth_deteriorating".to_string());
    }
    if matches!(
        context.liquidity.pressure,
        research_context::LiquidityPressure::Critical
    ) {
        drivers.push("liquidity_critical".to_string());
    }
    if context.regime.macro_stale_days > 3 {
        drivers.push("macro_stale".to_string());
    }

    drivers
}

/// Assess risk level from context
#[allow(dead_code)]
pub(crate) fn assess_risk_level(context: &research_context::ResearchContext) -> String {
    if context.breadth.breadth_pct < 20.0
        || matches!(
            context.liquidity.pressure,
            research_context::LiquidityPressure::Critical
        )
    {
        "critical".to_string()
    } else if context.breadth.breadth_pct < 30.0
        || matches!(
            context.liquidity.pressure,
            research_context::LiquidityPressure::High
        )
    {
        "high".to_string()
    } else if context.breadth.breadth_pct < 50.0 {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

/// Identify risk factors
#[allow(dead_code)]
pub(crate) fn identify_risk_factors(context: &research_context::ResearchContext) -> Vec<String> {
    let mut factors = Vec::new();

    if context.breadth.breadth_pct < 30.0 {
        factors.push("breadth_below_30".to_string());
    }
    if context.breadth.breadth_pct < 20.0 {
        factors.push("breadth_extreme_collapse".to_string());
    }
    if matches!(
        context.liquidity.pressure,
        research_context::LiquidityPressure::Critical
    ) {
        factors.push("liquidity_critical".to_string());
    }
    if context.regime.macro_stale_days > 5 {
        factors.push("macro_severely_stale".to_string());
    }

    factors
}

/// Generate recommendation
#[allow(dead_code)]
pub(crate) fn generate_recommendation(context: &research_context::ResearchContext) -> String {
    if context.breadth.breadth_pct < 20.0 {
        "exit".to_string()
    } else if context.breadth.breadth_pct < 30.0
        || matches!(
            context.liquidity.pressure,
            research_context::LiquidityPressure::Critical
        )
    {
        "reduce_exposure".to_string()
    } else if context.breadth.breadth_pct < 50.0 {
        "increase_quality".to_string()
    } else {
        "maintain".to_string()
    }
}

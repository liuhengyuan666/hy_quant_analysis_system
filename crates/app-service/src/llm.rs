use anyhow::Result;
use core_domain::LlmConfig;

/// Render LLM analysis JSON as markdown report.
/// V4.5: Expects {action, scope, placeholder, markdown} from analyze_with_action.
pub(crate) fn render_llm_analysis_markdown(analysis: &serde_json::Value) -> String {
    let mut md = String::new();

    let action = analysis["action"].as_str().unwrap_or("unknown");
    let scope = analysis["scope"].as_str().unwrap_or("global");

    md.push_str(&format!("# LLM Analysis: {}\n\n", action));
    md.push_str(&format!("**Scope**: {}\n\n", scope));

    // Placeholder warning
    if analysis["placeholder"].as_bool().unwrap_or(false) {
        md.push_str(
            "> **Warning**: This analysis was generated in placeholder mode. \
             No real LLM provider was configured.\n\n",
        );
    }

    // Markdown content (the actual LLM output)
    if let Some(content) = analysis["markdown"].as_str() {
        if !content.is_empty() {
            md.push_str(content);
            md.push_str("\n\n");
        }
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

use anyhow::{Context, Result};
use app_service::{AppContext, ReportScope};
use crate::ReportScopeArg;

/// Render an analyze_with_action result as markdown
pub fn render_action_result_md(value: &serde_json::Value) -> String {
    let mut md = String::new();

    md.push_str(&format!(
        "# Research Analysis: {}\n\n",
        value["action"].as_str().unwrap_or("unknown")
    ));

    if let Some(scope) = value["scope"].as_str() {
        md.push_str(&format!("**Scope**: {}\n\n", scope));
    }

    if value["placeholder"].as_bool().unwrap_or(false) {
        md.push_str(
            "> **Warning**: This analysis was generated in placeholder mode. \
             No real LLM provider was configured.\n\n",
        );
    }

    if let Some(content) = value["markdown"].as_str() {
        md.push_str(content);
        md.push_str("\n\n");
    }

    md
}

// ------------------------------------------------------------------
// Benchmark provider config loader
// ------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
struct ProviderConfigFile {
    #[serde(default)]
    provider: Vec<ProviderConfigEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct ProviderConfigEntry {
    name: String,
    base_url: String,
    model: String,
    api_key: String,
}

fn load_provider_config(path: &str) -> anyhow::Result<Vec<research_benchmark::ProviderConfig>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read provider config: {}", path))?;
    let file: ProviderConfigFile = toml::from_str(&content)
        .with_context(|| format!("failed to parse provider config: {}", path))?;

    let configs: Vec<research_benchmark::ProviderConfig> = file
        .provider
        .into_iter()
        .map(|entry| research_benchmark::ProviderConfig {
            name: entry.name,
            base_url: entry.base_url,
            model: entry.model,
            api_key: entry.api_key,
            timeout_secs: 60,
        })
        .collect();

    Ok(configs)
}

pub fn handle_list_actions() -> Result<()> {
    println!("Available Research Actions:");
    println!("  - market_story: 市场叙事");
    println!("  - explain_decision: 解释决策");
    println!("  - preclose_review: 收盘前复核");
    println!("  - risk_view: 风险视角");
    println!("  - devils_advocate: 唱反调");
    Ok(())
}

pub fn handle_benchmark_action(
    _context: &AppContext,
    action: String,
    provider_config: String,
    runs: usize,
    format: String,
    scope: ReportScopeArg,
    quiet: bool,
) -> Result<()> {
    if !quiet {
        eprintln!("[benchmark] Action '{}' is not yet benchmarkable after Research Layer refactor.", action);
        eprintln!("[benchmark] Scope: {:?}, Providers: {}, Runs: {}", scope, provider_config, runs);
    }
    println!("Benchmark not yet implemented for new ResearchAction architecture.");
    println!("Format: {}", format);
    Ok(())
}

pub fn handle_analyze(
    context: AppContext,
    action: String,
    scope: ReportScopeArg,
    format: String,
    _deterministic: bool,
    _seed: u64,
) -> Result<()> {
    let scope: ReportScope = scope.into();
    let result = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        runtime.block_on(context.analyze_with_action(&action, scope))
    })
    .join()
    .expect("LLM analysis thread panicked")?;

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "markdown" => {
            let md = render_action_result_md(&result);
            println!("{}", md);
        }
        _ => {
            anyhow::bail!(
                "Unsupported format: {}. Use 'json' or 'markdown'",
                format
            );
        }
    }
    Ok(())
}

use anyhow::{Context, Result};
use app_service::{AppContext, ReportScope};
use research_skills::AgentProfile;
use crate::ReportScopeArg;

/// Render an analyze_with_skill serde_json::Value result as markdown
pub fn render_skill_result_md(value: &serde_json::Value) -> String {
    let mut md = String::new();

    // Title
    md.push_str(&format!(
        "# Skill Analysis: {}\n\n",
        value["skill"].as_str().unwrap_or("unknown")
    ));

    // Triggered status
    let triggered = value["triggered"].as_bool().unwrap_or(false);
    md.push_str(&format!(
        "**Triggered**: {}\n\n",
        if triggered { "✅ Yes" } else { "❌ No" }
    ));

    if !triggered {
        if let Some(reason) = value["reason"].as_str() {
            md.push_str(&format!("**Reason**: {}\n\n", reason));
        }
    }

    // Scope
    if let Some(scope) = value["scope"].as_str() {
        md.push_str(&format!("**Scope**: {}\n\n", scope));
    }

    // Regime Analysis
    if let Some(regime) = value["regime_analysis"].as_object() {
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
    if let Some(llm) = value["llm_analysis"].as_str() {
        if !llm.is_empty() {
            md.push_str("## LLM Analysis\n\n");
            md.push_str(llm);
            md.push_str("\n\n");
        }
    }

    // Token Usage
    if let Some(tokens) = value["token_usage"].as_object() {
        md.push_str("## Token Usage\n\n");
        if let Some(input) = tokens.get("input_tokens").and_then(|v| v.as_u64()) {
            md.push_str(&format!("- **Input Tokens**: {}\n", input));
        }
        if let Some(output) = tokens.get("output_tokens").and_then(|v| v.as_u64()) {
            md.push_str(&format!("- **Output Tokens**: {}\n", output));
        }
        if let Some(total) = tokens.get("total_tokens").and_then(|v| v.as_u64()) {
            md.push_str(&format!("- **Total Tokens**: {}\n", total));
        }
        md.push('\n');
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

pub fn handle_list_skills() -> Result<()> {
    let skill_dir = std::path::PathBuf::from("crates/research-skills/skills");
    let registry = research_skills::registry::SkillRegistry::new(skill_dir)
        .map_err(|e| anyhow::anyhow!("Failed to load skills: {}", e))?;

    println!("Available Research Skills:");
    let mut names: Vec<_> = registry.list().into_iter().map(|s| s.to_string()).collect();
    names.sort();
    for name in &names {
        if let Some(skill) = registry.get(name) {
            println!(
                "  - {}: {} (priority: {})",
                name,
                skill.definition.description,
                skill.definition.priority
            );
        }
    }
    Ok(())
}

pub fn handle_benchmark_skill(
    context: &AppContext,
    skill: String,
    provider_config: String,
    runs: usize,
    format: String,
    scope: ReportScopeArg,
    quiet: bool,
) -> Result<()> {
    if !quiet {
        eprintln!("[benchmark] Loading skill '{}'...", skill);
    }
    let skill_dir = std::path::PathBuf::from("crates/research-skills/skills");
    let registry = research_skills::registry::SkillRegistry::new(skill_dir)
        .map_err(|e| anyhow::anyhow!("Failed to load skills: {}", e))?;
    let skill_obj = registry
        .get(&skill)
        .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found", skill))?;

    if !quiet {
        eprintln!("[benchmark] Building ResearchContext for scope {:?}...", scope);
    }
    let research_ctx = context.research_context(scope.into())?;

    if !quiet {
        eprintln!("[benchmark] Loading provider config from '{}'...", provider_config);
    }
    let providers = load_provider_config(&provider_config)?;
    if providers.is_empty() {
        anyhow::bail!("No providers configured in {}", provider_config);
    }

    let resolved = context.get_resolved_llm_config(None)?;
    let inference = research_skills::InferenceConfig {
        temperature: resolved.temperature,
        seed: resolved.seed,
        max_tokens: resolved.max_tokens,
    };

    // Load schema if specified by the skill
    let schema = skill_obj.definition.output_schema.as_ref().and_then(|schema_file| {
        let schema_path = std::path::PathBuf::from("crates/research-skills/skills")
            .join(&skill)
            .join(schema_file);
        match std::fs::read_to_string(&schema_path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(schema) => Some(schema),
                Err(e) => {
                    eprintln!("WARN: failed to parse schema at {}: {}", schema_path.display(), e);
                    None
                }
            },
            Err(e) => {
                eprintln!("WARN: failed to read schema at {}: {}", schema_path.display(), e);
                None
            }
        }
    });

    let suite = research_benchmark::BenchmarkSuite {
        skill: skill_obj.clone(),
        context: research_ctx,
        providers,
        runs_per_provider: runs,
        inference,
        schema,
    };

    let rt = tokio::runtime::Runtime::new()?;
    let report = rt.block_on(research_benchmark::BenchmarkHarness::run_suite(&suite))?;

    let output = if format == "markdown" {
        research_benchmark::ReportGenerator::to_markdown(&report)
    } else {
        research_benchmark::ReportGenerator::to_json(&report)?
    };
    println!("{}", output);
    Ok(())
}

pub fn handle_analyze(
    context: AppContext,
    skill: String,
    scope: ReportScopeArg,
    agent: Option<String>,
    format: String,
    deterministic: bool,
    seed: u64,
) -> Result<()> {
    let scope: ReportScope = scope.into();
    let resolved = context.get_resolved_llm_config(None)?;
    let inference_override = if deterministic {
        Some(research_skills::InferenceConfig {
            temperature: 0.0,
            seed: Some(seed),
            max_tokens: resolved.max_tokens,
        })
    } else {
        None
    };
    let profile = if let Some(agent_name) = &agent {
        let profile_path = format!("research/agents/{}.yaml", agent_name);
        let profile_yaml = std::fs::read_to_string(&profile_path)
            .with_context(|| format!("Failed to load agent profile '{}'", agent_name))?;
        let profile = AgentProfile::from_yaml(&profile_yaml)
            .with_context(|| format!("Failed to parse agent profile '{}'", agent_name))?;
        Some(profile)
    } else {
        None
    };
    let result = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        runtime.block_on(context.analyze_with_skill(&skill, scope, profile.as_ref(), inference_override))
    })
    .join()
    .expect("LLM analysis thread panicked")?;

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "markdown" => {
            // Try deserializing as ResearchAnalysis; fall back to raw value rendering
            match serde_json::from_value::<research_skills::ResearchAnalysis>(
                result.clone(),
            ) {
                Ok(analysis) => {
                    let md = research_skills::render_analysis_markdown(&analysis);
                    println!("{}", md);
                }
                Err(_) => {
                    // Fallback: render the raw analyze_with_skill result as markdown
                    let md = render_skill_result_md(&result);
                    println!("{}", md);
                }
            }
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

//! LLM persona loading from `config/prompts.toml` (RV1 Phase 3).
//!
//! ADR-106 boundary: personas carry perspective mandates only — no thresholds,
//! no scoring rules, no if/then decision logic. File-based personas override or
//! extend the six built-in research-skills personas; unknown actions fall back
//! to built-ins; truly unknown actions produce an error listing valid keys.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PromptsFile {
    #[serde(default)]
    pub prompts: HashMap<String, PersonaDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PersonaDefinition {
    pub label: String,
    /// Optional override of the built-in system prompt (required for custom personas).
    pub system: Option<String>,
    /// Optional override of the built-in template (required for custom personas).
    pub template: Option<String>,
}

/// A fully resolved persona ready for prompt building.
#[derive(Debug, Clone)]
pub struct ResolvedPersona {
    pub key: String,
    pub label: String,
    pub system: String,
    pub template: String,
}

/// Load `config/prompts.toml`. Missing or malformed file → empty map (built-ins only).
pub fn load_prompts(project_root: &Path) -> PromptsFile {
    let path = project_root.join("config").join("prompts.toml");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return PromptsFile::default();
    };
    toml::from_str::<PromptsFile>(&content).unwrap_or_default()
}

/// Resolve an action key to a concrete persona.
///
/// Resolution order:
/// 1. File persona: file.system/template take precedence; missing fields fall back
///    to the built-in persona of the same key (only for the six built-in actions).
/// 2. Built-in persona (research-skills) for known action keys.
/// 3. Error listing valid keys.
pub fn resolve_persona(file: &PromptsFile, action: &str) -> anyhow::Result<ResolvedPersona> {
    let builtin = research_skills::builtin_persona(action);

    if let Some(def) = file.prompts.get(action) {
        let system = def
            .system
            .clone()
            .or_else(|| builtin.map(|(system, _)| system.to_string()));
        let template = def
            .template
            .clone()
            .or_else(|| builtin.map(|(_, template)| template.to_string()));
        return match (system, template) {
            (Some(system), Some(template)) => Ok(ResolvedPersona {
                key: action.to_string(),
                label: def.label.clone(),
                system,
                template,
            }),
            _ => anyhow::bail!(
                "persona '{}' in config/prompts.toml is custom and must define both `system` and `template`",
                action
            ),
        };
    }

    if let Some((system, template)) = builtin {
        let label = match action {
            "market_story" => "市场叙事",
            "explain_decision" => "解释决策",
            "preclose_review" => "收盘前复核",
            "risk_view" => "风险视角",
            "devils_advocate" => "唱反调",
            "portfolio_review" => "组合决策解读",
            other => other,
        };
        return Ok(ResolvedPersona {
            key: action.to_string(),
            label: label.to_string(),
            system: system.to_string(),
            template: template.to_string(),
        });
    }

    let mut valid: Vec<String> = file.prompts.keys().cloned().collect();
    valid.sort();
    anyhow::bail!(
        "unknown action '{}'. Built-in actions: market_story, explain_decision, preclose_review, risk_view, devils_advocate, portfolio_review. File personas: [{}]",
        action,
        valid.join(", ")
    )
}

/// List all available persona keys (built-ins + file) for help output.
pub fn available_personas(file: &PromptsFile) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![
        ("market_story".into(), "市场叙事".into()),
        ("explain_decision".into(), "解释决策".into()),
        ("preclose_review".into(), "收盘前复核".into()),
        ("risk_view".into(), "风险视角".into()),
        ("devils_advocate".into(), "唱反调".into()),
        ("portfolio_review".into(), "组合决策解读".into()),
    ];
    for (key, def) in &file.prompts {
        if !out.iter().any(|(existing, _)| existing == key) {
            out.push((key.clone(), def.label.clone()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_action_resolves_without_file() {
        let file = PromptsFile::default();
        let persona = resolve_persona(&file, "market_story").unwrap();
        assert!(persona.system.contains("市场研究员"));
    }

    #[test]
    fn file_persona_overrides_builtin_system() {
        let mut file = PromptsFile::default();
        file.prompts.insert(
            "market_story".into(),
            PersonaDefinition {
                label: "自定义叙事".into(),
                system: Some("自定义系统提示".into()),
                template: None,
            },
        );
        let persona = resolve_persona(&file, "market_story").unwrap();
        assert_eq!(persona.system, "自定义系统提示");
        // template falls back to built-in
        assert!(persona.template.contains("市场叙事"));
    }

    #[test]
    fn custom_persona_requires_both_fields() {
        let mut file = PromptsFile::default();
        file.prompts.insert(
            "custom_x".into(),
            PersonaDefinition {
                label: "X".into(),
                system: Some("sys".into()),
                template: None,
            },
        );
        assert!(resolve_persona(&file, "custom_x").is_err());
    }

    #[test]
    fn unknown_action_lists_valid_keys() {
        let file = PromptsFile::default();
        let err = resolve_persona(&file, "no_such_action").unwrap_err();
        assert!(err.to_string().contains("market_story"));
    }
}

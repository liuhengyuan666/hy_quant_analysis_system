use serde::{Deserialize, Serialize};

use super::reasoning::ReasoningGraph;
use super::trigger::Trigger;

/// Skill definition parsed from SKILL.md front matter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub trigger: Trigger,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub dependencies: Vec<String>,
    pub confidence_model: Option<ConfidenceModel>,
    pub failure_modes: Vec<FailureMode>,
    pub evaluation_metrics: Vec<String>,
    pub output_schema: Option<String>,
    pub priority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceModel {
    pub base: f64,
    pub factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureMode {
    pub condition: String,
    pub action: String,
    pub message: String,
}

/// Complete Skill with definition + content
#[derive(Debug, Clone)]
pub struct Skill {
    pub definition: SkillDefinition,
    pub overview: String,
    pub reasoning: ReasoningGraph,
    pub output_format: String,
}

impl Skill {
    /// Parse a SKILL.md file content into a Skill
    pub fn from_markdown(content: &str) -> anyhow::Result<Self> {
        // Split front matter from content
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            anyhow::bail!("Invalid SKILL.md format: missing front matter");
        }

        let definition: SkillDefinition = serde_yaml::from_str(parts[1])?;
        let body = parts[2];

        // Parse body sections (Overview, Reasoning Graph, Output Format)
        let overview = Self::extract_section(body, "Overview").unwrap_or_default();
        let reasoning_yaml =
            Self::extract_section(body, "Reasoning Graph").unwrap_or_default();
        let output_format_raw =
            Self::extract_section(body, "Output Format").unwrap_or_default();

        // Strip code fences before parsing (LLM responses often wrap in ```yaml / ```json)
        let clean_reasoning = reasoning_yaml
            .trim_start_matches("```yaml")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let reasoning: ReasoningGraph = if clean_reasoning.is_empty() {
            ReasoningGraph::default()
        } else {
            serde_yaml::from_str(clean_reasoning)?
        };

        let output_format = output_format_raw
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string();

        Ok(Skill {
            definition,
            overview,
            reasoning,
            output_format,
        })
    }

    fn extract_section(body: &str, section_name: &str) -> Option<String> {
        let pattern = format!("## {}", section_name);
        let start = body.find(&pattern)?;
        let after_start = &body[start + pattern.len()..];
        let end = after_start.find("\n## ").unwrap_or(after_start.len());
        Some(after_start[..end].trim().to_string())
    }
}

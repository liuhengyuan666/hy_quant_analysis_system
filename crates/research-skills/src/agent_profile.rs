use serde::{Deserialize, Serialize};

/// Agent Profile - defines reasoning style and constraints for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub name: String,
    pub description: String,
    pub reasoning_style: Vec<String>,
    pub risk_tolerance: RiskTolerance,
    pub output_depth: OutputDepth,
    pub output_format: OutputFormat,
    pub priority: AnalysisPriority,
    pub analysis_constraints: AnalysisConstraints,
    pub skills: Vec<String>,
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskTolerance {
    Conservative,
    Moderate,
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputDepth {
    Shallow,
    Standard,
    Deep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPriority {
    pub macro_priority: f64,
    pub technical_priority: f64,
    pub sentiment_priority: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConstraints {
    pub preferred_factors: Vec<String>,
    pub emphasis: EmphasisConfig,
    pub tone: RenderingTone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmphasisConfig {
    pub regime_transition: EmphasisLevel,
    pub breadth_signal: EmphasisLevel,
    pub liquidity_signal: EmphasisLevel,
    pub rotation_signal: EmphasisLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmphasisLevel {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderingTone {
    Cautious,
    Neutral,
    Optimistic,
}

impl AgentProfile {
    /// Load profile from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Get system prompt with variable substitution
    pub fn render_system_prompt(&self) -> String {
        if let Some(prompt) = &self.system_prompt {
            prompt
                .replace("{reasoning_style}", &self.reasoning_style.join(", "))
                .replace("{risk_tolerance}", &format!("{:?}", self.risk_tolerance))
                .replace("{output_depth}", &format!("{:?}", self.output_depth))
                .replace("{tone}", &format!("{:?}", self.analysis_constraints.tone))
        } else {
            self.default_system_prompt()
        }
    }

    fn default_system_prompt(&self) -> String {
        format!(
            "You are a {} analyst. Your reasoning style is {}. Your risk tolerance is {}. \
             Output depth: {}. Tone: {}. \
             Focus on: {}.",
            self.name,
            self.reasoning_style.join(", "),
            format!("{:?}", self.risk_tolerance).to_lowercase(),
            format!("{:?}", self.output_depth).to_lowercase(),
            format!("{:?}", self.analysis_constraints.tone).to_lowercase(),
            self.analysis_constraints.preferred_factors.join(", ")
        )
    }
}

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Standard research analysis output structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchAnalysis {
    pub meta: AnalysisMeta,
    pub thesis: Thesis,
    pub evidence: Vec<Evidence>,
    pub risks: Vec<Risk>,
    pub recommendations: Vec<Action>,
    pub confidence: ConfidenceScore,
    pub reasoning_trace: ReasoningTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMeta {
    pub skill_name: String,
    pub agent_profile: String,
    pub scope: String,
    pub analysis_date: NaiveDate,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thesis {
    pub statement: String,
    pub conviction: f64,           // [0, 1]
    pub time_horizon: String,      // short / medium / long
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub source: String,
    pub data_point: String,
    pub strength: f64,             // [0, 1]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub category: String,
    pub severity: String,          // low / medium / high / critical
    pub probability: f64,          // [0, 1]
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: String,       // reduce_exposure / increase_quality / hedge / monitor
    pub target: String,
    pub urgency: String,           // immediate / near_term / watch
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceScore {
    pub overall: f64,              // [0, 1]
    pub data_quality: f64,
    pub model_fit: f64,
    pub market_clarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTrace {
    pub steps: Vec<ReasoningStep>,
    pub assumptions: Vec<String>,
    pub alternative_scenarios: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_number: usize,
    pub premise: String,
    pub conclusion: String,
    pub confidence: f64,
}

impl ResearchAnalysis {
    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

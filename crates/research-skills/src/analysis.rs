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

    /// Render analysis as markdown report
    pub fn render_markdown(&self) -> String {
        render_analysis_markdown(self)
    }
}

/// Render a ResearchAnalysis as a markdown report string
pub fn render_analysis_markdown(analysis: &ResearchAnalysis) -> String {
    let mut md = String::new();

    // Header
    md.push_str(&format!(
        "# Research Analysis: {}\n\n",
        analysis.meta.skill_name
    ));
    md.push_str(&format!(
        "**Agent**: {} | **Scope**: {} | **Date**: {}\n\n",
        analysis.meta.agent_profile,
        analysis.meta.scope,
        analysis.meta.analysis_date.format("%Y-%m-%d")
    ));
    md.push_str("---\n\n");

    // Thesis
    md.push_str("## Thesis\n\n");
    md.push_str(&format!("{}\n\n", analysis.thesis.statement));
    md.push_str(&format!(
        "- **Conviction**: {:.0}%\n",
        analysis.thesis.conviction * 100.0
    ));
    md.push_str(&format!(
        "- **Time Horizon**: {}\n\n",
        analysis.thesis.time_horizon
    ));

    // Evidence
    if !analysis.evidence.is_empty() {
        md.push_str("## Evidence\n\n");
        for (i, e) in analysis.evidence.iter().enumerate() {
            md.push_str(&format!(
                "{}. **{}**: {} (strength: {:.0}%)\n",
                i + 1,
                e.source,
                e.data_point,
                e.strength * 100.0
            ));
        }
        md.push('\n');
    }

    // Risks
    if !analysis.risks.is_empty() {
        md.push_str("## Risks\n\n");
        for risk in &analysis.risks {
            md.push_str(&format!("### {} ({} severity)\n\n", risk.category, risk.severity));
            md.push_str(&format!(
                "- **Probability**: {:.0}%\n",
                risk.probability * 100.0
            ));
            if let Some(mitigation) = &risk.mitigation {
                md.push_str(&format!("- **Mitigation**: {}\n", mitigation));
            }
            md.push('\n');
        }
    }

    // Recommendations
    if !analysis.recommendations.is_empty() {
        md.push_str("## Recommendations\n\n");
        for (i, rec) in analysis.recommendations.iter().enumerate() {
            md.push_str(&format!(
                "{}. **{}** → `{}` (urgency: {})\n",
                i + 1,
                rec.action_type,
                rec.target,
                rec.urgency
            ));
            md.push_str(&format!("   - {}\n\n", rec.rationale));
        }
    }

    // Confidence
    md.push_str("## Confidence Assessment\n\n");
    md.push_str(&format!(
        "- **Overall**: {:.0}%\n",
        analysis.confidence.overall * 100.0
    ));
    md.push_str(&format!(
        "- **Data Quality**: {:.0}%\n",
        analysis.confidence.data_quality * 100.0
    ));
    md.push_str(&format!(
        "- **Model Fit**: {:.0}%\n",
        analysis.confidence.model_fit * 100.0
    ));
    md.push_str(&format!(
        "- **Market Clarity**: {:.0}%\n\n",
        analysis.confidence.market_clarity * 100.0
    ));

    // Reasoning Trace
    md.push_str("## Reasoning Trace\n\n");
    for step in &analysis.reasoning_trace.steps {
        md.push_str(&format!(
            "**Step {}** ({:.0}% confidence)\n\n- **Premise**: {}\n- **Conclusion**: {}\n\n",
            step.step_number,
            step.confidence * 100.0,
            step.premise,
            step.conclusion
        ));
    }

    if !analysis.reasoning_trace.assumptions.is_empty() {
        md.push_str("### Assumptions\n\n");
        for a in &analysis.reasoning_trace.assumptions {
            md.push_str(&format!("- {}\n", a));
        }
        md.push('\n');
    }

    if !analysis.reasoning_trace.alternative_scenarios.is_empty() {
        md.push_str("### Alternative Scenarios\n\n");
        for s in &analysis.reasoning_trace.alternative_scenarios {
            md.push_str(&format!("- {}\n", s));
        }
        md.push('\n');
    }

    md
}

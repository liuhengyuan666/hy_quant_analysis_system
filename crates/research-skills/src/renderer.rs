use crate::analysis::ResearchAnalysis;

/// Render ResearchAnalysis to Markdown format
pub fn render_analysis_markdown(analysis: &ResearchAnalysis) -> String {
    let mut output = String::new();

    // Title
    output.push_str(&format!("# Research Analysis: {}\n\n", analysis.meta.skill_name));

    // Meta section
    output.push_str("## Analysis Metadata\n\n");
    output.push_str(&format!("- **Skill**: {}\n", analysis.meta.skill_name));
    output.push_str(&format!("- **Agent Profile**: {}\n", analysis.meta.agent_profile));
    output.push_str(&format!("- **Scope**: {}\n", analysis.meta.scope));
    output.push_str(&format!("- **Date**: {}\n", analysis.meta.analysis_date));
    output.push_str(&format!("- **Version**: {}\n\n", analysis.meta.version));

    // Thesis
    output.push_str("## Thesis\n\n");
    output.push_str(&format!("{}\n\n", analysis.thesis.statement));
    output.push_str(&format!("- **Conviction**: {:.1}%\n", analysis.thesis.conviction * 100.0));
    output.push_str(&format!("- **Time Horizon**: {}\n\n", analysis.thesis.time_horizon));

    // Evidence
    output.push_str("## Evidence\n\n");
    for (i, evidence) in analysis.evidence.iter().enumerate() {
        output.push_str(&format!("{}. **{}** ({})\n", i + 1, evidence.source, evidence.data_point));
        output.push_str(&format!("   - Strength: {:.1}%\n", evidence.strength * 100.0));
    }
    output.push_str("\n");

    // Risks
    output.push_str("## Risks\n\n");
    for risk in &analysis.risks {
        output.push_str(&format!("- **{}** ({}): {:.1}%\n", risk.category, risk.severity, risk.probability * 100.0));
        if let Some(mitigation) = &risk.mitigation {
            output.push_str(&format!("  - Mitigation: {}\n", mitigation));
        }
    }
    output.push_str("\n");

    // Recommendations
    output.push_str("## Recommendations\n\n");
    for action in &analysis.recommendations {
        output.push_str(&format!("- **{}** ({}): {}\n", action.action_type, action.urgency, action.target));
        output.push_str(&format!("  - Rationale: {}\n", action.rationale));
    }
    output.push_str("\n");

    // Confidence
    output.push_str("## Confidence\n\n");
    output.push_str(&format!("- **Overall**: {:.1}%\n", analysis.confidence.overall * 100.0));
    output.push_str(&format!("- **Data Quality**: {:.1}%\n", analysis.confidence.data_quality * 100.0));
    output.push_str(&format!("- **Model Fit**: {:.1}%\n", analysis.confidence.model_fit * 100.0));
    output.push_str(&format!("- **Market Clarity**: {:.1}%\n\n", analysis.confidence.market_clarity * 100.0));

    // Reasoning Trace
    output.push_str("## Reasoning Trace\n\n");
    for step in &analysis.reasoning_trace.steps {
        output.push_str(&format!("### Step {}\n\n", step.step_number));
        output.push_str(&format!("**Premise**: {}\n\n", step.premise));
        output.push_str(&format!("**Conclusion**: {}\n\n", step.conclusion));
        output.push_str(&format!("**Confidence**: {:.1}%\n\n", step.confidence * 100.0));
    }

    if !analysis.reasoning_trace.assumptions.is_empty() {
        output.push_str("### Assumptions\n\n");
        for assumption in &analysis.reasoning_trace.assumptions {
            output.push_str(&format!("- {}\n", assumption));
        }
        output.push_str("\n");
    }

    if !analysis.reasoning_trace.alternative_scenarios.is_empty() {
        output.push_str("### Alternative Scenarios\n\n");
        for scenario in &analysis.reasoning_trace.alternative_scenarios {
            output.push_str(&format!("- {}\n", scenario));
        }
        output.push_str("\n");
    }

    output
}

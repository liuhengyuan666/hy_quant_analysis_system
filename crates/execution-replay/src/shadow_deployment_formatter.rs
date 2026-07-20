use crate::shadow_deployment::ShadowDeploymentReport;

/// Markdown / JSON formatter for `ShadowDeploymentReport`.
pub struct ShadowDeploymentFormatter;

impl ShadowDeploymentFormatter {
    /// Renders the report as Markdown.
    pub fn markdown(report: &ShadowDeploymentReport) -> String {
        let mut lines = Vec::new();
        lines.push("# Shadow Deployment Report".into());
        lines.push(String::new());
        lines.push(format!("**Contract Version:** {}", report.contract_version));
        lines.push(format!("**Scope:** {}", report.scope));
        lines.push(format!("**Generated:** {}", report.generated_at));
        lines.push(format!(
            "**Total Days:** {} | **High Risk:** {} | **Elevated:** {} | **Normal:** {}",
            report.summary.total_days,
            report.summary.high_risk_days,
            report.summary.elevated_risk_days,
            report.summary.normal_days
        ));
        lines.push(String::new());

        lines.push("## Summary".into());
        lines.push(String::new());
        lines.push(format!(
            "- Validation Status: {:?}",
            report.summary.validation_status
        ));
        lines.push(format!(
            "- Transition Detected Days: {} ({:.1}%)",
            report.summary.transition_detected_days,
            if report.summary.total_days > 0 {
                report.summary.transition_detected_days as f64 / report.summary.total_days as f64 * 100.0
            } else {
                0.0
            }
        ));
        lines.push(format!(
            "- Avg HoldingRiskScore: {:.2}",
            report.summary.avg_holding_risk_score
        ));
        lines.push(format!(
            "- Lifecycle Events: {} | False Alarms: {}",
            report.summary.lifecycle_events, report.summary.false_alarms
        ));
        lines.push(String::new());

        lines.push("## Daily Assessment".into());
        lines.push(String::new());
        lines.push("**[RESEARCH ONLY — NOT ACTIONABLE]**".into());
        lines.push(String::new());
        lines.push("| Date | Regime | Score | Lifecycle State | Research Interpretation | LD | LP | CD |".into());
        lines.push("|------|--------|------:|-----------------|-------------------------|----|----|----|".into());
        for a in &report.assessments {
            lines.push(format!(
                "| {} | {} | {:.2} | {} | {} | {} | {} | {} |",
                a.date,
                a.regime,
                a.holding_risk_score,
                a.lifecycle_state,
                a.research_interpretation,
                if a.evidence.leadership_decay_persistence { "Y" } else { "-" },
                if a.evidence.liquidity_pressure { "Y" } else { "-" },
                if a.evidence.confirmation_decay { "Y" } else { "-" }
            ));
        }
        lines.push(String::new());

        lines.push("## Contract".into());
        lines.push(String::new());
        lines.push("```text".into());
        lines.push("Input:  real ResearchContext (via ExecutionResearchRecord)".into());
        lines.push("Output: ShadowRiskAssessment (observation-only)".into());
        lines.push("Prohibition: DecisionEngine must NOT consume ShadowRiskAssessment".into());
        lines.push("```".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the report as JSON.
    pub fn json(report: &ShadowDeploymentReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shadow_deployment::ShadowDeploymentReport;

    #[test]
    fn markdown_contains_contract() {
        let report = ShadowDeploymentReport {
            generated_at: chrono::Utc::now(),
            scope: "CN".into(),
            contract_version: "v2c.1.0".into(),
            assessments: vec![],
            summary: crate::shadow_deployment::ShadowDeploymentSummary {
                total_days: 0,
                high_risk_days: 0,
                elevated_risk_days: 0,
                normal_days: 0,
                transition_detected_days: 0,
                avg_holding_risk_score: 0.0,
                lifecycle_events: 0,
                false_alarms: 0,
                validation_status: crate::shadow_deployment::ShadowValidationStatus::Normal,
            },
        };
        let text = ShadowDeploymentFormatter::markdown(&report);
        assert!(text.contains("Shadow Deployment"));
        assert!(text.contains("Prohibition"));
    }

    #[test]
    fn json_round_trips() {
        let report = ShadowDeploymentReport {
            generated_at: chrono::Utc::now(),
            scope: "CN".into(),
            contract_version: "v2c.1.0".into(),
            assessments: vec![],
            summary: crate::shadow_deployment::ShadowDeploymentSummary {
                total_days: 0,
                high_risk_days: 0,
                elevated_risk_days: 0,
                normal_days: 0,
                transition_detected_days: 0,
                avg_holding_risk_score: 0.0,
                lifecycle_events: 0,
                false_alarms: 0,
                validation_status: crate::shadow_deployment::ShadowValidationStatus::Normal,
            },
        };
        let text = ShadowDeploymentFormatter::json(&report);
        assert!(text.contains("scope"));
    }
}

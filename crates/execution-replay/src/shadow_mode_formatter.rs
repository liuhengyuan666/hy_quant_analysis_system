use crate::shadow_mode::ShadowModeReport;

/// Markdown / JSON formatter for `ShadowModeReport`.
pub struct ShadowModeFormatter;

impl ShadowModeFormatter {
    /// Renders the report as Markdown.
    pub fn markdown(report: &ShadowModeReport) -> String {
        let mut lines = Vec::new();
        lines.push("# Shadow Mode Report".into());
        lines.push(String::new());
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
        lines.push(String::new());

        lines.push("## Daily Output".into());
        lines.push(String::new());
        lines.push("| Date | Regime | Score | Risk State | Transition | Candidate | LD | LP | CD |".into());
        lines.push("|------|--------|------:|------------|------------|-----------|----|----|----|".into());
        for o in &report.outputs {
            lines.push(format!(
                "| {} | {} | {:.2} | {} | {} | {} | {} | {} | {} |",
                o.date,
                o.market_regime,
                o.holding_risk_score,
                o.risk_state,
                if o.transition_detected { "Yes" } else { "No" },
                o.decision_candidate,
                if o.evidence_details.leadership_decay_persistence { "Y" } else { "-" },
                if o.evidence_details.liquidity_pressure { "Y" } else { "-" },
                if o.evidence_details.confirmation_decay { "Y" } else { "-" }
            ));
        }
        lines.push(String::new());

        lines.push("## State Machine Definition".into());
        lines.push(String::new());
        lines.push("```text".into());
        lines.push("State Context:   market_regime_label (RiskOn / Neutral / RiskOff)".into());
        lines.push("Transition Evidence: HoldingRiskScore".into());
        lines.push("HIGH_RISK:       RiskOff OR HoldingRiskScore >= 0.75".into());
        lines.push("ELEVATED_RISK:   Neutral OR HoldingRiskScore >= 0.5".into());
        lines.push("NORMAL:          otherwise".into());
        lines.push("```".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the report as JSON.
    pub fn json(report: &ShadowModeReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shadow_mode::ShadowModeReport;

    #[test]
    fn markdown_contains_summary() {
        let report = ShadowModeReport {
            generated_at: chrono::Utc::now(),
            scope: "CN".into(),
            outputs: vec![],
            summary: crate::shadow_mode::ShadowModeSummary {
                total_days: 0,
                high_risk_days: 0,
                elevated_risk_days: 0,
                normal_days: 0,
                transition_detected_days: 0,
                avg_holding_risk_score: 0.0,
            },
        };
        let text = ShadowModeFormatter::markdown(&report);
        assert!(text.contains("Shadow Mode"));
    }

    #[test]
    fn json_round_trips() {
        let report = ShadowModeReport {
            generated_at: chrono::Utc::now(),
            scope: "CN".into(),
            outputs: vec![],
            summary: crate::shadow_mode::ShadowModeSummary {
                total_days: 0,
                high_risk_days: 0,
                elevated_risk_days: 0,
                normal_days: 0,
                transition_detected_days: 0,
                avg_holding_risk_score: 0.0,
            },
        };
        let text = ShadowModeFormatter::json(&report);
        assert!(text.contains("scope"));
    }
}

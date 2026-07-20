use crate::state_risk_acceleration::StateRiskAccelerationAnalysis;

/// Markdown / JSON formatter for `StateRiskAccelerationAnalysis`.
pub struct StateRiskAccelerationFormatter;

impl StateRiskAccelerationFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &StateRiskAccelerationAnalysis) -> String {
        let mut lines = Vec::new();
        lines.push("# State Risk Acceleration Model".into());
        lines.push(String::new());
        lines.push(format!("**Total Records:** {}", analysis.total_records));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(analysis.verdict.clone());
        lines.push(String::new());

        lines.push("## Regime Distribution".into());
        lines.push(String::new());
        lines.push("| Regime | Count | Negative T+60 | Avg T+60 |".into());
        lines.push("|--------|------:|--------------:|---------:|".into());
        for r in &analysis.regime_distribution {
            lines.push(format!(
                "| {} | {} | {:.1}% | {:.2}% |",
                r.regime,
                r.count,
                r.negative_t60_rate * 100.0,
                r.avg_t60 * 100.0
            ));
        }
        lines.push(String::new());

        lines.push("## State Risk Score Buckets (T+60)".into());
        lines.push(String::new());
        lines.push("| Score | Count | Negative T+60 | Baseline | Lift | Precision | Avg T+60 |".into());
        lines.push("|-------|------:|--------------:|---------:|-----:|----------:|---------:|".into());
        for b in &analysis.state_risk_buckets {
            lines.push(format!(
                "| {} | {} | {:.1}% | {:.1}% | {:.2} | {:.1}% | {:.2}% |",
                b.score_label,
                b.count,
                b.negative_t60_rate * 100.0,
                b.baseline_negative_rate * 100.0,
                b.lift,
                b.precision * 100.0,
                b.avg_t60 * 100.0
            ));
        }
        lines.push(String::new());

        lines.push("## Regime Classification (High Risk: score >= 2.0)".into());
        lines.push(String::new());
        lines.push("| Regime | Count | Negative T+60 | Avg T+60 | Recall |".into());
        lines.push("|--------|------:|--------------:|---------:|-------:|".into());
        for c in &analysis.regime_classification {
            lines.push(format!(
                "| {} | {} | {:.1}% | {:.2}% | {:.1}% |",
                c.regime,
                c.count,
                c.negative_t60_rate * 100.0,
                c.avg_t60 * 100.0,
                c.recall * 100.0
            ));
        }
        lines.push(String::new());

        lines.push("## Score Formula".into());
        lines.push(String::new());
        lines.push("```text".into());
        lines.push("StateRiskAccelerationScore = ".into());
        lines.push("  DowntrendAcceleration    * 1.0  (5d return < 0 and worsening)".into());
        lines.push("+ VolatilityNegativeDrift  * 1.0  (amplitude > 70th pct + today_return < 0 + close_position < 0.3)".into());
        lines.push("+ PersistentBreadthCollapse * 1.0  (breadth_delta_5d < -5 for >= 2 days)".into());
        lines.push("+ LiquidityStress          * 1.0  (volume_ratio_delta_5d < -0.2 for >= 2 days + today_return < 0)".into());
        lines.push("```".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &StateRiskAccelerationAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_risk_acceleration::compute_state_risk_acceleration_analysis;

    #[test]
    fn markdown_contains_verdict() {
        let analysis = compute_state_risk_acceleration_analysis(&[]);
        let text = StateRiskAccelerationFormatter::markdown(&analysis);
        assert!(text.contains("State Risk Acceleration"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = compute_state_risk_acceleration_analysis(&[]);
        let text = StateRiskAccelerationFormatter::json(&analysis);
        assert!(text.contains("total_records"));
    }
}

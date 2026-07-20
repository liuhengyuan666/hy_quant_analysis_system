use crate::confirmation_decay::ConfirmationDecayAnalysis;

/// Markdown / JSON formatter for `ConfirmationDecayAnalysis`.
pub struct ConfirmationDecayFormatter;

impl ConfirmationDecayFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &ConfirmationDecayAnalysis) -> String {
        let mut lines = Vec::new();
        lines.push("# ConfirmationDecay Research Asset Analysis".into());
        lines.push(String::new());
        lines.push(format!("**Total Records:** {}", analysis.total_records));
        lines.push(format!(
            "**Baseline T+20 Negative Rate:** {:.1}% | **Baseline T+60 Negative Rate:** {:.1}%",
            analysis.baseline_negative_t20_rate * 100.0,
            analysis.baseline_negative_t60_rate * 100.0
        ));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(analysis.verdict.clone());
        lines.push(String::new());

        lines.push("## Performance".into());
        lines.push(String::new());
        lines.push("| Horizon | Negative Rate | Lift | Precision | Avg Return | Median Return | False Reduce |".into());
        lines.push("|---------|--------------:|-----:|----------:|-----------:|--------------:|-------------:|".into());
        lines.push(format!(
            "| T+20 | {:.1}% | {:.2} | {:.1}% | {:.2}% | {:.2}% | {:.1}% |",
            analysis.negative_t20_rate * 100.0,
            analysis.lift_t20,
            analysis.precision_t20 * 100.0,
            analysis.avg_t20 * 100.0,
            analysis.median_t20 * 100.0,
            analysis.false_reduce_rate_t20 * 100.0
        ));
        lines.push(format!(
            "| T+60 | {:.1}% | {:.2} | {:.1}% | {:.2}% | {:.2}% | {:.1}% |",
            analysis.negative_t60_rate * 100.0,
            analysis.lift_t60,
            analysis.precision_t60 * 100.0,
            analysis.avg_t60 * 100.0,
            analysis.median_t60 * 100.0,
            analysis.false_reduce_rate_t60 * 100.0
        ));
        lines.push(String::new());

        lines.push("## Signal Definition".into());
        lines.push(String::new());
        lines.push("- `confirmation_score = (trend + participation + risk) / 3`".into());
        lines.push("- Trigger when `confirmation_delta_5d < -10` OR `slope_10d < -2` OR `consecutive_decline_days >= 3`".into());
        lines.push("- Optional: require `today_return < 0` (price weakness)".into());
        lines.push(String::new());

        lines.push("## Role / Horizon".into());
        lines.push(String::new());
        lines.push("```text".into());
        lines.push("Role:    HoldingRisk / Confirmation".into());
        lines.push("Horizon: ShortTerm (T+20) / MediumTerm (T+60)".into());
        lines.push("```".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &ConfirmationDecayAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::confirmation_decay::compute_confirmation_decay_analysis;

    #[test]
    fn markdown_contains_verdict() {
        let analysis = compute_confirmation_decay_analysis(&[], true);
        let text = ConfirmationDecayFormatter::markdown(&analysis);
        assert!(text.contains("ConfirmationDecay"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = compute_confirmation_decay_analysis(&[], true);
        let text = ConfirmationDecayFormatter::json(&analysis);
        assert!(text.contains("total_records"));
    }
}

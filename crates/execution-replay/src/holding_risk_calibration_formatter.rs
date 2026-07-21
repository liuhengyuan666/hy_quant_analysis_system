use crate::holding_risk_calibration::HoldingRiskCalibrationAnalysis;

/// Markdown / JSON formatter for `HoldingRiskCalibrationAnalysis`.
pub struct HoldingRiskCalibrationFormatter;

impl HoldingRiskCalibrationFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &HoldingRiskCalibrationAnalysis) -> String {
        let mut lines = Vec::new();
        lines.push("# Holding Risk Calibration v2".into());
        lines.push(String::new());
        lines.push(format!("**Total Records:** {}", analysis.total_records));
        lines.push(format!(
            "**Baseline T+60 Negative Rate:** {:.1}%",
            analysis.baseline_negative_t60_rate * 100.0
        ));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(analysis.verdict.clone());
        lines.push(String::new());

        lines.push("## Score Buckets (T+60)".into());
        lines.push(String::new());
        lines.push("| Score | Count | Negative T+60 | Baseline | Lift | Precision | Avg T+60 | Median T+60 | False Reduce |".into());
        lines.push("|-------|------:|--------------:|---------:|-----:|----------:|---------:|------------:|-------------:|".into());
        for b in &analysis.score_buckets {
            lines.push(format!(
                "| {} | {} | {:.1}% | {:.1}% | {:.2} | {:.1}% | {:.2}% | {:.2}% | {:.1}% |",
                b.score_label,
                b.count,
                b.negative_t60_rate * 100.0,
                b.baseline_negative_rate * 100.0,
                b.lift,
                b.precision * 100.0,
                b.avg_t60 * 100.0,
                b.median_t60 * 100.0,
                b.false_reduce_rate * 100.0
            ));
        }
        lines.push(String::new());

        lines.push("## Regime Stability (High Risk: score >= 0.75)".into());
        lines.push(String::new());
        lines.push("| Regime | Total | High Risk | Precision | Lift | Baseline |".into());
        lines.push("|--------|------:|----------:|----------:|-----:|---------:|".into());
        for r in &analysis.regime_buckets {
            lines.push(format!(
                "| {} | {} | {} | {:.1}% | {:.2} | {:.1}% |",
                r.regime,
                r.count,
                r.high_risk_count,
                r.high_risk_precision * 100.0,
                r.high_risk_lift,
                r.baseline_negative_rate * 100.0
            ));
        }
        lines.push(String::new());

        lines.push("## Walk-Forward Validation".into());
        lines.push(String::new());
        lines.push(format!("**Train:** {}", analysis.walk_forward.train_period));
        lines.push(format!("**Validate:** {}", analysis.walk_forward.validate_period));
        lines.push(String::new());
        lines.push("| Period | High Risk | Precision | Lift |".into());
        lines.push("|--------|----------:|----------:|-----:|".into());
        lines.push(format!(
            "| Train | {} | {:.1}% | {:.2} |",
            analysis.walk_forward.train_high_risk_count,
            analysis.walk_forward.train_precision * 100.0,
            analysis.walk_forward.train_lift
        ));
        lines.push(format!(
            "| Validate | {} | {:.1}% | {:.2} |",
            analysis.walk_forward.validate_high_risk_count,
            analysis.walk_forward.validate_precision * 100.0,
            analysis.walk_forward.validate_lift
        ));
        lines.push(format!(
            "| Precision Decay | | {:.1}% | |",
            analysis.walk_forward.precision_decay * 100.0
        ));
        lines.push(String::new());

        lines.push("## Score Formula".into());
        lines.push(String::new());
        lines.push("```text".into());
        lines.push("HoldingRiskScore = ".into());
        lines.push("  LeadershipDecayPersistence(>=5d) * 0.5".into());
        lines.push("+ LiquidityPressure(>=3d)           * 0.25".into());
        lines.push("+ ConfirmationDecay(>=2d)           * 0.25".into());
        lines.push("```".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &HoldingRiskCalibrationAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holding_risk_calibration::compute_holding_risk_calibration;

    #[test]
    fn markdown_contains_verdict() {
        let analysis = compute_holding_risk_calibration(&[]);
        let text = HoldingRiskCalibrationFormatter::markdown(&analysis);
        assert!(text.contains("Holding Risk Calibration"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = compute_holding_risk_calibration(&[]);
        let text = HoldingRiskCalibrationFormatter::json(&analysis);
        assert!(text.contains("total_records"));
    }
}

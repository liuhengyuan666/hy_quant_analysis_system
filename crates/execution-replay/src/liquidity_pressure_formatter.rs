use crate::liquidity_pressure::LiquidityPressureAnalysis;

/// Markdown / JSON formatter for `LiquidityPressureAnalysis`.
pub struct LiquidityPressureFormatter;

impl LiquidityPressureFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &LiquidityPressureAnalysis) -> String {
        let mut lines = Vec::new();
        lines.push("# LiquidityPressure Research Asset Analysis".into());
        lines.push(String::new());
        lines.push(format!("**Total Records:** {}", analysis.total_records));
        lines.push(format!(
            "**Baseline T+60 Negative Rate:** {:.1}%",
            analysis.baseline_negative_t60_rate * 100.0
        ));
        lines.push(format!(
            "**Baseline Avg T+60:** {:.2}%",
            analysis.baseline_avg_t60 * 100.0
        ));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(analysis.verdict.clone());
        lines.push(String::new());

        lines.push("## Signal Definition".into());
        lines.push(String::new());
        if let Some(level) = analysis.volume_level_threshold {
            lines.push(format!("- `volume_ratio < {:.2}` (relative volume level)", level));
        } else {
            lines.push(format!("- `volume_ratio_delta_5d < {:.0}%`", analysis.threshold_volume_ratio_delta * 100.0));
        }
        lines.push("- `today_return < 0%`".into());
        lines.push("- `breadth_delta_5d < 0`".into());
        lines.push(format!(
            "- Consecutive pressure days >= {}",
            analysis.consecutive_pressure_days
        ));
        lines.push(String::new());

        lines.push("## T+60 Performance".into());
        lines.push(String::new());
        lines.push("| Metric | Value |".into());
        lines.push("|---|---:|".into());
        lines.push(format!("| Signal Count | {} |", analysis.signal_count));
        lines.push(format!(
            "| Negative T+60 Rate | {:.1}% |",
            analysis.negative_t60_rate * 100.0
        ));
        lines.push(format!("| Lift | {:.2} |", analysis.lift));
        lines.push(format!("| Precision | {:.1}% |", analysis.precision * 100.0));
        lines.push(format!("| Avg T+60 | {:.2}% |", analysis.avg_t60 * 100.0));
        lines.push(format!("| Median T+60 | {:.2}% |", analysis.median_t60 * 100.0));
        lines.push(format!(
            "| False Reduce Rate | {:.1}% |",
            analysis.false_reduce_rate * 100.0
        ));
        lines.push(String::new());

        lines.push("## Role / Horizon".into());
        lines.push(String::new());
        lines.push("```text".into());
        lines.push("Role:    HoldingRisk".into());
        lines.push("Horizon: MediumTerm (T+60)".into());
        lines.push("```".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &LiquidityPressureAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquidity_pressure::{
        compute_liquidity_pressure_analysis, LiquidityPressureSignal,
    };

    #[test]
    fn markdown_contains_verdict() {
        let analysis = compute_liquidity_pressure_analysis(&[], 3);
        let text = LiquidityPressureFormatter::markdown(&analysis);
        assert!(text.contains("LiquidityPressure"));
        assert!(text.contains("HoldingRisk"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = compute_liquidity_pressure_analysis(&[], 3);
        let text = LiquidityPressureFormatter::json(&analysis);
        assert!(text.contains("total_records"));
    }
}

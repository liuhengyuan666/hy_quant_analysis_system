use crate::holding_risk_bundle::HoldingRiskBundleAnalysis;

/// Markdown / JSON formatter for `HoldingRiskBundleAnalysis`.
pub struct HoldingRiskBundleFormatter;

impl HoldingRiskBundleFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &HoldingRiskBundleAnalysis) -> String {
        let mut lines = Vec::new();

        lines.push("# Holding Risk Evidence Bundle Analysis".into());
        lines.push(String::new());
        lines.push(format!(
            "**Total Records:** {} | **Natural Horizon:** T+60 | **Medium-Term Holding Risk Focus**",
            analysis.total_records
        ));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(analysis.verdict.clone());
        lines.push(String::new());

        lines.push("## Signal-Count Buckets".into());
        lines.push(String::new());
        lines.push("| Score | Count | Negative T+60 | Baseline | Lift | Precision | Avg T+60 | Median T+60 | Avg Max DD |".into());
        lines.push("|-------|------:|--------------:|---------:|-----:|----------:|---------:|------------:|-----------:|".into());
        for b in &analysis.score_distribution {
            lines.push(format!(
                "| {} | {} | {:.1}% | {:.1}% | {:.2} | {:.1}% | {:.2}% | {:.2}% | {:.2}% |",
                b.score_label,
                b.count,
                b.t60_negative_rate * 100.0,
                b.baseline_negative_rate * 100.0,
                b.lift,
                b.precision * 100.0,
                b.avg_t60_return * 100.0,
                b.median_t60_return * 100.0,
                b.avg_max_drawdown * 100.0
            ));
        }
        lines.push(String::new());

        lines.push("## Interpretation".into());
        lines.push(String::new());
        lines.push("- Score 0: no holding risk signals; baseline expectation.".into());
        lines.push("- Score 1: one dimension of holding risk present (leadership, breadth, or liquidity).".into());
        lines.push("- Score 2: two dimensions align; elevated medium-term holding risk.".into());
        lines.push("- Score 3: all three dimensions align; strongest medium-term holding risk warning.".into());
        lines.push(String::new());
        lines.push("This is not an Exit Signal model. The bundle answers: *How much medium-term holding risk is present in current positions?*".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &HoldingRiskBundleAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holding_risk_bundle::{
        compute_holding_risk_bundle_analysis, HoldingRiskBundleAnalysis,
    };

    #[test]
    fn markdown_contains_score_table() {
        let analysis = HoldingRiskBundleAnalysis::default();
        let text = HoldingRiskBundleFormatter::markdown(&analysis);
        assert!(text.contains("Holding Risk Evidence Bundle Analysis"));
        assert!(text.contains("Signal-Count Buckets"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = HoldingRiskBundleAnalysis::default();
        let text = HoldingRiskBundleFormatter::json(&analysis);
        assert!(text.contains("total_records"));
        assert!(text.contains("score_distribution"));
    }
}

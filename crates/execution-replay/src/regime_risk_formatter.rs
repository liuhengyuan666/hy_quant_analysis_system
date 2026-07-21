use crate::regime_risk_model::RegimeRiskAnalysis;

/// Markdown / JSON formatter for `RegimeRiskAnalysis`.
pub struct RegimeRiskFormatter;

impl RegimeRiskFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &RegimeRiskAnalysis) -> String {
        let mut lines = Vec::new();
        lines.push("# Regime-Aware State Risk Model".into());
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
        lines.push("RegimeRiskScore = ".into());
        lines.push("  TrendBreakdown        * 1.0  (price < MA20 and MA60, MA60 slope < 0)".into());
        lines.push("+ VolatilityExpansion  * 1.0  (amplitude > 70th percentile over 60 days)".into());
        lines.push("+ MarketBreadthCollapse * 1.0  (breadth_pct < 30%)".into());
        lines.push("+ LiquidityStress      * 1.0  (volume_ratio < 0.6)".into());
        lines.push("```".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &RegimeRiskAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::regime_risk_model::compute_regime_risk_analysis;

    #[test]
    fn markdown_contains_verdict() {
        let analysis = compute_regime_risk_analysis(&[]);
        let text = RegimeRiskFormatter::markdown(&analysis);
        assert!(text.contains("Regime-Aware State Risk"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = compute_regime_risk_analysis(&[]);
        let text = RegimeRiskFormatter::json(&analysis);
        assert!(text.contains("total_records"));
    }
}

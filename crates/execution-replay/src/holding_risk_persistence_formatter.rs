use crate::holding_risk_persistence::HoldingRiskPersistenceAnalysis;

/// Markdown / JSON formatter for `HoldingRiskPersistenceAnalysis`.
pub struct HoldingRiskPersistenceFormatter;

impl HoldingRiskPersistenceFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &HoldingRiskPersistenceAnalysis) -> String {
        let mut lines = Vec::new();
        lines.push("# Holding Risk Persistence Analysis".into());
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

        lines.push("## Consecutive-Day Experiments".into());
        lines.push(String::new());
        lines.push("| Signal | Min Days | Samples | Negative T+60 | Lift | Precision | Avg T+60 | Median T+60 | False Reduce |".into());
        lines.push("|--------|---------:|--------:|--------------:|-----:|----------:|---------:|------------:|-------------:|".into());
        for exp in &analysis.experiments {
            lines.push(format!(
                "| {} | {} | {} | {:.1}% | {:.2} | {:.1}% | {:.2}% | {:.2}% | {:.1}% |",
                exp.signal_name,
                exp.min_consecutive_days,
                exp.sample_count,
                exp.negative_rate * 100.0,
                exp.lift,
                exp.precision * 100.0,
                exp.avg_t60 * 100.0,
                exp.median_t60 * 100.0,
                exp.false_reduce_rate * 100.0
            ));
        }
        lines.push(String::new());

        if let Some(exp) = &analysis.velocity_experiment {
            lines.push("## Velocity Experiment".into());
            lines.push(String::new());
            lines.push(format!(
                "| {} | window={} | samples={} | negative={:.1}% | lift={:.2} | precision={:.1}% | avg={:.2}% | median={:.2}% | false_reduce={:.1}% |",
                exp.signal_name,
                exp.velocity_window.unwrap_or(0),
                exp.sample_count,
                exp.negative_rate * 100.0,
                exp.lift,
                exp.precision * 100.0,
                exp.avg_t60 * 100.0,
                exp.median_t60 * 100.0,
                exp.false_reduce_rate * 100.0
            ));
            lines.push(String::new());
        }

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &HoldingRiskPersistenceAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holding_risk_persistence::{
        compute_holding_risk_persistence_analysis, HoldingRiskPersistenceAnalysis,
        PersistenceExperiment,
    };

    #[test]
    fn markdown_contains_experiments() {
        let analysis = HoldingRiskPersistenceAnalysis {
            total_records: 100,
            baseline_negative_t60_rate: 0.4,
            baseline_avg_t60: 0.05,
            experiments: vec![PersistenceExperiment {
                signal_name: "LeadershipDecay >= 2 consecutive days".into(),
                horizon: "T+60".into(),
                min_consecutive_days: 2,
                velocity_window: None,
                sample_count: 50,
                negative_rate: 0.6,
                baseline_negative_rate: 0.4,
                lift: 1.5,
                precision: 0.6,
                avg_t60: -0.02,
                median_t60: -0.03,
                false_reduce_rate: 0.35,
            }],
            velocity_experiment: None,
            verdict: "test".into(),
        };
        let text = HoldingRiskPersistenceFormatter::markdown(&analysis);
        assert!(text.contains("LeadershipDecay >= 2 consecutive days"));
        assert!(text.contains("1.50"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = compute_holding_risk_persistence_analysis(&[]);
        let text = HoldingRiskPersistenceFormatter::json(&analysis);
        assert!(text.contains("total_records"));
    }
}

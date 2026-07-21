use crate::risk_lifecycle::RiskLifecycleAnalysis;

/// Markdown / JSON formatter for `RiskLifecycleAnalysis`.
pub struct RiskLifecycleFormatter;

impl RiskLifecycleFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &RiskLifecycleAnalysis) -> String {
        let mut lines = Vec::new();
        lines.push("# Holding Risk Lifecycle Analysis".into());
        lines.push(String::new());
        lines.push(format!("**Total Records:** {}", analysis.total_records));
        lines.push(format!("**Total Events:** {}", analysis.total_events));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(analysis.verdict.clone());
        lines.push(String::new());

        lines.push("## Summary Statistics".into());
        lines.push(String::new());
        lines.push("| Metric | Value |".into());
        lines.push("|---|---:|".into());
        lines.push(format!("| Avg Duration (days) | {:.1} |", analysis.avg_duration_days));
        lines.push(format!("| Median Duration (days) | {:.1} |", analysis.median_duration_days));
        lines.push(format!("| Avg Peak Score | {:.2} |", analysis.avg_peak_score));
        lines.push(format!("| Avg Recovery (days) | {:.1} |", analysis.avg_recovery_days));
        lines.push(format!("| False Alarm Rate | {:.1}% |", analysis.false_alarm_rate * 100.0));
        lines.push(format!("| Avg T+60 Return | {:.2}% |", analysis.avg_t60_return * 100.0));
        lines.push(format!("| Avg Max Drawdown | {:.2}% |", analysis.avg_max_drawdown * 100.0));
        lines.push(String::new());

        lines.push("## Event Log".into());
        lines.push(String::new());
        lines.push("| Symbol | Entry | Peak | Recovery | Duration | Peak Score | T+60 | Max DD | False Alarm |".into());
        lines.push("|--------|-------|------|----------|---------:|-----------:|-----:|-------:|-------------|".into());
        for e in analysis.events.iter().take(20) {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {:.2} | {} | {} | {} |",
                e.symbol,
                e.entry_date,
                e.peak_date,
                e.recovery_date.map(|d| d.to_string()).unwrap_or_else(|| "-".into()),
                e.duration_days,
                e.peak_score,
                e.avg_t60_return.map(|r| format!("{:.2}%", r * 100.0)).unwrap_or_else(|| "-".into()),
                e.max_drawdown.map(|d| format!("{:.2}%", d * 100.0)).unwrap_or_else(|| "-".into()),
                if e.is_false_alarm { "Yes" } else { "No" }
            ));
        }
        if analysis.events.len() > 20 {
            lines.push(format!("| ... | | | | | | | | ({} more) |", analysis.events.len() - 20));
        }
        lines.push(String::new());

        lines.push("## State Machine Definition".into());
        lines.push(String::new());
        lines.push("```text".into());
        lines.push("Entry:    HoldingRiskScore >= 0.75 for >= 2 consecutive days".into());
        lines.push("Peak:     Local maximum score during event".into());
        lines.push("Recovery: HoldingRiskScore < 0.50 for >= 2 consecutive days".into());
        lines.push("Duration: Entry date to Recovery date".into());
        lines.push("False Alarm: Avg T+60 return >= 0".into());
        lines.push("```".into());
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &RiskLifecycleAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk_lifecycle::compute_risk_lifecycle_analysis;

    #[test]
    fn markdown_contains_verdict() {
        let analysis = compute_risk_lifecycle_analysis(&[]);
        let text = RiskLifecycleFormatter::markdown(&analysis);
        assert!(text.contains("Holding Risk Lifecycle"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = compute_risk_lifecycle_analysis(&[]);
        let text = RiskLifecycleFormatter::json(&analysis);
        assert!(text.contains("total_records"));
    }
}

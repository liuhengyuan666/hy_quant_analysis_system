use serde_json;

use crate::distribution_coverage::DistributionCoverageReview;

/// Formatter for Distribution Coverage Review.
pub struct DistributionCoverageFormatter;

impl DistributionCoverageFormatter {
    /// Returns compact JSON.
    pub fn json(review: &DistributionCoverageReview) -> String {
        serde_json::to_string_pretty(review).unwrap_or_else(|_| "{}".into())
    }

    /// Returns a Markdown report.
    pub fn markdown(review: &DistributionCoverageReview) -> String {
        let mut lines = Vec::new();
        lines.push("# Distribution Coverage Review".into());
        lines.push("".into());
        lines.push(format!("**Records:** {}", review.record_count));
        lines.push("".into());
        lines.push("Current Distribution observation condition: `close_position < 0.2 && volume_ratio > 1.5 && today_return < 0.0`".into());
        lines.push("".into());

        lines.push("## Feature Percentiles".into());
        lines.push("".into());
        lines.push("| Feature | Count | Min | P10 | P25 | P50 | P75 | P90 | P95 | Max | Mean |".into());
        lines.push("|---------|------:|----:|----:|----:|----:|----:|----:|----:|----:|-----:|".into());

        for (name, summary) in [
            ("close_position", &review.close_position),
            ("volume_ratio", &review.volume_ratio),
            ("today_return", &review.today_return),
        ] {
            lines.push(format!(
                "| {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} | {:.3} |",
                name,
                summary.count,
                summary.min,
                summary.p10,
                summary.p25,
                summary.p50,
                summary.p75,
                summary.p90,
                summary.p95,
                summary.max,
                summary.mean
            ));
        }
        lines.push("".into());

        lines.push("## Condition Coverage".into());
        lines.push("".into());
        let c = &review.condition_coverage;
        lines.push(format!("- Total records: {}", c.total_records));
        lines.push(format!("- Records with `today_return < 0.0`: {} ({:.1}%)",
            c.records_with_negative_return,
            c.records_with_negative_return as f64 / c.total_records as f64 * 100.0
        ));
        lines.push(format!("- Records with negative return + `close_position < 0.2`: {} ({:.1}% of down days)",
            c.records_with_negative_return_and_low_close,
            if c.records_with_negative_return == 0 { 0.0 } else { c.records_with_negative_return_and_low_close as f64 / c.records_with_negative_return as f64 * 100.0 }
        ));
        lines.push(format!("- Records with negative return + `volume_ratio > 1.5`: {} ({:.1}% of down days)",
            c.records_with_negative_return_and_high_volume,
            if c.records_with_negative_return == 0 { 0.0 } else { c.records_with_negative_return_and_high_volume as f64 / c.records_with_negative_return as f64 * 100.0 }
        ));
        lines.push(format!("- Records satisfying **all three** conditions: {} ({:.1}% of down days)",
            c.records_satisfying_all_conditions,
            if c.records_with_negative_return == 0 { 0.0 } else { c.records_satisfying_all_conditions as f64 / c.records_with_negative_return as f64 * 100.0 }
        ));
        lines.push(format!("- Records that actually produced a Distribution observation: {}", c.records_with_distribution_observation));
        lines.push(format!("- Coverage of all-conditions records: {:.1}%", c.coverage_pct * 100.0));
        lines.push("".into());

        lines.push("## Interpretation".into());
        lines.push("".into());
        if c.records_satisfying_all_conditions == 0 {
            lines.push("No record satisfies all three conditions. This means the current thresholds are extremely strict for this dataset.".into());
        } else if c.coverage_pct >= 0.95 {
            lines.push("Almost all records that satisfy the conditions produce a Distribution observation. The issue is likely the condition thresholds, not the observation logic.".into());
        } else if c.coverage_pct <= 0.5 {
            lines.push("Many records satisfy the conditions but do not produce a Distribution observation. This suggests the condition is necessary but not sufficient, or the observation logic has additional hidden filters.".into());
        } else {
            lines.push("Coverage is partial. Investigate the specific records that satisfy the conditions but did not trigger an observation.".into());
        }
        lines.push("".into());

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution_coverage::{
        compute_distribution_coverage_review, DistributionCoverageReview,
    };

    #[test]
    fn markdown_contains_header() {
        let review = compute_distribution_coverage_review(&[]);
        let md = DistributionCoverageFormatter::markdown(&review);
        assert!(md.contains("Distribution Coverage Review"));
    }

    #[test]
    fn json_round_trips() {
        let review = compute_distribution_coverage_review(&[]);
        let json = DistributionCoverageFormatter::json(&review);
        let restored: DistributionCoverageReview = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.record_count, 0);
    }
}

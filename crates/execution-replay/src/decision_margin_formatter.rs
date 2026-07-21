use serde_json;

use crate::decision_margin::{DecisionMarginReview, EvidenceDecisionProfile};

/// Formatter for Decision Margin Review.
pub struct DecisionMarginFormatter;

impl DecisionMarginFormatter {
    /// Returns compact JSON.
    pub fn json(review: &DecisionMarginReview) -> String {
        serde_json::to_string_pretty(review).unwrap_or_else(|_| "{}".into())
    }

    /// Returns a Markdown report.
    pub fn markdown(review: &DecisionMarginReview) -> String {
        let mut lines = Vec::new();
        lines.push("# Decision Margin Review".into());
        lines.push("".into());
        lines.push(format!("**Records:** {}", review.record_count));
        lines.push("".into());
        lines.push("This review shows, for each EvidenceKind, how `assessment.dominant_direction` maps to the final Decision.".into());
        lines.push("It answers: 'Did the decision match the assessment direction, and where are the threshold boundaries?'".into());
        lines.push("".into());

        for profile in &review.profiles {
            lines.append(&mut Self::format_profile(profile));
        }

        lines.join("\n")
    }

    fn format_profile(profile: &EvidenceDecisionProfile) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("## {:?}", profile.evidence_kind));
        lines.push("".into());
        lines.push(format!("- Records with this evidence: {}", profile.record_count));
        lines.push(format!("- Reduce threshold: {:.3}", profile.reduce_threshold));
        lines.push(format!("- Reduce recall: {:.1}%", profile.reduce_recall() * 100.0));
        lines.push(format!(
            "- Direction-positive records that became BuyNow: {} / Wait: {}",
            profile.buy_now_when_direction_positive, profile.wait_when_direction_positive
        ));
        lines.push(format!(
            "- Direction-negative records that became Reduce: {} / Wait: {} / (missed Reduce: {})",
            profile.reduce_when_direction_negative,
            profile.wait_when_direction_negative,
            profile.missed_reduce_count
        ));
        lines.push("".into());

        lines.push("### Dominant Direction Histogram".into());
        lines.push("".into());
        lines.push("| Range | Total | BuyNow | Wait | Reduce |".into());
        lines.push("|-------|------:|-------:|-----:|-------:|".into());

        for bucket in &profile.direction_histogram {
            lines.push(format!(
                "| [{:.2}, {:.2}) | {} | {} | {} | {} |",
                bucket.bin_start,
                bucket.bin_end,
                bucket.total,
                bucket.buy_now,
                bucket.wait,
                bucket.reduce,
            ));
        }
        lines.push("".into());
        lines.push("### Visual Bar Chart".into());
        lines.push("".into());
        let max_total = profile.direction_histogram.iter().map(|b| b.total).max().unwrap_or(0).max(1);
        for bucket in &profile.direction_histogram {
            let bar_len = if max_total > 0 {
                (bucket.total as f64 / max_total as f64 * 30.0) as usize
            } else {
                0
            };
            let bar: String = std::iter::repeat("█").take(bar_len).collect();
            lines.push(format!(
                "[{:.2}, {:.2}): {} {}",
                bucket.bin_start,
                bucket.bin_end,
                bucket.total,
                bar
            ));
        }
        lines.push("".into());
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_margin::{compute_decision_margin_review, DecisionMarginReview};

    #[test]
    fn markdown_contains_header() {
        let review = DecisionMarginReview {
            record_count: 0,
            profiles: vec![],
        };
        let md = DecisionMarginFormatter::markdown(&review);
        assert!(md.contains("Decision Margin Review"));
    }

    #[test]
    fn json_round_trips() {
        let review = DecisionMarginReview {
            record_count: 0,
            profiles: vec![],
        };
        let json = DecisionMarginFormatter::json(&review);
        let restored: DecisionMarginReview = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.record_count, 0);
    }
}

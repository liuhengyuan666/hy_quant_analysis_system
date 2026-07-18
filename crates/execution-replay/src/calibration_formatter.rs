use serde_json;

use crate::calibration::CalibrationReview;

/// Formatter for Calibration Review.
pub struct CalibrationFormatter;

impl CalibrationFormatter {
    /// Returns compact JSON.
    pub fn json(review: &CalibrationReview) -> String {
        serde_json::to_string_pretty(review).unwrap_or_else(|_| "{}".into())
    }

    /// Returns a Markdown report.
    pub fn markdown(review: &CalibrationReview) -> String {
        let mut lines = Vec::new();
        lines.push("# Directional Confidence Calibration Experiment".into());
        lines.push("".into());
        lines.push(format!("**Total Records:** {}", review.total_records));
        lines.push(format!(
            "**Baseline Reduce Candidates:** {} / {}",
            review.baseline_reduce_candidates, review.total_records
        ));
        lines.push(format!(
            "**Baseline Reduce Count:** {}",
            review.baseline_reduce_count
        ));
        lines.push("".into());
        lines.push("This experiment re-runs the DecisionEngine on the same records with alternative confidence thresholds, measuring coverage, precision, and opportunity cost.".into());
        lines.push("".into());

        lines.push("## Summary".into());
        lines.push("".into());
        lines.push("| Experiment | Reduce Candidates | Reduce Count | Avoided Loss | Missed Recovery | Missed Reduce | Precision | Recall | F1 | Avg T+20 (Reduce) |".into());
        lines.push("|------------|------------------:|-------------:|-------------:|----------------:|--------------:|----------:|-------:|---:|------------------:|".into());
        for result in &review.results {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                result.experiment_name,
                result.reduce_candidates,
                result.reduce_count,
                result.avoided_loss_count,
                result.missed_recovery_count,
                result.missed_reduce_count,
                fmt_pct(result.precision),
                fmt_pct(result.recall),
                fmt_pct(result.f1),
                fmt_pct(result.avg_t20_return_after_reduce)
            ));
        }
        lines.push("".into());

        lines.push("## Metrics Definition".into());
        lines.push("".into());
        lines.push("- **Reduce Candidates**: records with `dominant_direction < reduce_threshold`.".into());
        lines.push("- **Reduce Count**: how many candidates became `Reduce` under the experiment.".into());
        lines.push("- **Avoided Loss**: `Reduce` decision and actual T+20 return < 0.0.".into());
        lines.push("- **Missed Recovery**: `Reduce` decision and actual T+20 return >= 0.0.".into());
        lines.push("- **Missed Reduce**: `Wait` decision for a reduce candidate and T+20 return < 0.0.".into());
        lines.push("- **Precision**: avoided_loss / (avoided_loss + missed_recovery).".into());
        lines.push("- **Recall**: avoided_loss / (avoided_loss + missed_reduce).".into());
        lines.push("- **F1**: harmonic mean of precision and recall.".into());
        lines.push("- **Avg T+20 (Reduce)**: average forward T+20 return of records that became Reduce.".into());
        lines.push("".into());

        lines.push("## Per-Experiment Detail".into());
        lines.push("".into());
        for result in &review.results {
            lines.push(format!("### {}", result.experiment_name));
            lines.push("".into());
            lines.push(format!("- Description: {}", result.description));
            lines.push(format!("- Total records: {}", result.total_records));
            lines.push(format!("- Reduce candidates: {}", result.reduce_candidates));
            lines.push(format!("- Reduce count: {} ({:.1}% of candidates)",
                result.reduce_count,
                if result.reduce_candidates > 0 {
                    result.reduce_count as f64 / result.reduce_candidates as f64 * 100.0
                } else {
                    0.0
                }
            ));
            lines.push(format!(
                "- BuyNow count: {} | Wait count: {}",
                result.buy_now_count, result.wait_count
            ));
            lines.push(format!(
                "- Avoided loss: {} | Missed recovery: {} | Missed reduce: {} | Correct wait: {}",
                result.avoided_loss_count,
                result.missed_recovery_count,
                result.missed_reduce_count,
                result.correct_wait_count
            ));
            lines.push(format!(
                "- Precision: {} | Recall: {} | F1: {}",
                fmt_pct(result.precision),
                fmt_pct(result.recall),
                fmt_pct(result.f1)
            ));
            lines.push(format!(
                "- Avg T+20 after Reduce: {} | Avg T+60 after Reduce: {} | Avg T+120 after Reduce: {}",
                fmt_pct(result.avg_t20_return_after_reduce),
                fmt_pct(result.avg_t60_return_after_reduce),
                fmt_pct(result.avg_t120_return_after_reduce)
            ));
            lines.push(format!(
                "- Avg T+20 for all Reduce candidates: {}",
                fmt_pct(result.avg_t20_return_for_reduce_candidates)
            ));
            lines.push("".into());

            lines.push("#### Reduce Decisions (first 30)".into());
            lines.push("".into());
            lines.push("| # | Symbol | Date | Direction | Confidence | T+20 | T+60 | Outcome |".into());
            lines.push("|---|--------|------|-----------|------------|------|------|---------|".into());
            let reduce_decisions: Vec<_> = result
                .decisions
                .iter()
                .filter(|d| d.experiment_state == execution_engine::ExecutionState::Reduce)
                .collect();
            for (i, d) in reduce_decisions.iter().take(30).enumerate() {
                let outcome = d.t20_return.map(|r| {
                    if r < 0.0 {
                        "Avoided Loss"
                    } else {
                        "Missed Recovery"
                    }
                }).unwrap_or("Unknown");
                lines.push(format!(
                    "| {} | {} | {} | {:.3} | {:.3} | {} | {} | {} |",
                    i + 1,
                    d.symbol,
                    d.date,
                    d.dominant_direction,
                    d.confidence,
                    fmt_opt(d.t20_return),
                    fmt_opt(d.t60_return),
                    outcome
                ));
            }
            if reduce_decisions.len() > 30 {
                lines.push(format!("| ... | ... | ... | ... | ... | ... | ... | ({} total) |", reduce_decisions.len()));
            }
            lines.push("".into());
        }

        lines.push("## Recommendation".into());
        lines.push("".into());
        if review.results.len() < 2 {
            lines.push("No experiments were run or only a baseline was provided.".into());
        } else {
            let best = &review.results[1..]
                .iter()
                .max_by(|a, b| a.f1.partial_cmp(&b.f1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(&review.results[0]);
            lines.push(format!(
                "Best F1 experiment: **{}** (F1={:.1}%, Precision={:.1}%, Recall={:.1}%, Reduce count={}/{}).",
                best.experiment_name,
                best.f1.unwrap_or(0.0) * 100.0,
                best.precision.unwrap_or(0.0) * 100.0,
                best.recall.unwrap_or(0.0) * 100.0,
                best.reduce_count,
                best.reduce_candidates
            ));
            lines.push("".into());
            if let Some(p) = best.precision {
                if p < 0.5 {
                    lines.push("**Caution**: No experiment achieves 50% precision. Lowering the confidence threshold releases Reduce actions, but the majority of those Reduce actions would miss subsequent recoveries. This suggests the underlying bearish evidence is not yet strong enough for reliable Reduce signals.".into());
                    lines.push("".into());
                    lines.push("**Recommendation**: Do not lower the confidence threshold yet. Instead, investigate whether the bearish evidence quality can be improved (e.g., more specific Distribution/RiskExpansion conditions, or additional Holding Risk evidence) before recalibrating.".into());
                } else {
                    lines.push("This experiment meets the 50% precision threshold. It can be considered for promotion to the new ExecutionPolicy default after a review of the per-record decisions.".into());
                }
            }
        }
        lines.push("".into());
        lines.push("**No code changes are applied by this report.**".into());
        lines.push("".into());

        lines.join("\n")
    }
}

fn fmt_pct(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.1}%", v * 100.0),
        None => "N/A".into(),
    }
}

fn fmt_opt(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.2}%", v * 100.0),
        None => "N/A".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::compute_calibration_review;

    #[test]
    fn markdown_contains_header() {
        let review = compute_calibration_review(&[], &[]);
        let md = CalibrationFormatter::markdown(&review);
        assert!(md.contains("Directional Confidence Calibration Experiment"));
    }

    #[test]
    fn json_round_trips() {
        let review = compute_calibration_review(&[], &[]);
        let json = CalibrationFormatter::json(&review);
        let restored: CalibrationReview = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_records, 0);
    }
}

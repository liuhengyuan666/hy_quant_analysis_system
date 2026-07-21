use serde_json;

use crate::risk_semantics::RiskSemanticsReview;

/// Formatter for Risk Semantics Review.
pub struct RiskSemanticsFormatter;

impl RiskSemanticsFormatter {
    /// Returns compact JSON.
    pub fn json(review: &RiskSemanticsReview) -> String {
        serde_json::to_string_pretty(review).unwrap_or_else(|_| "{}".into())
    }

    /// Returns a Markdown report.
    pub fn markdown(review: &RiskSemanticsReview) -> String {
        let mut lines = Vec::new();
        lines.push("# Risk Semantics Review".into());
        lines.push("".into());
        lines.push(format!("**Total Records:** {}", review.total_records));
        lines.push("".into());
        lines.push("This review analyzes `RiskLevel::High` records to determine whether the current".into());
        lines.push("risk semantics are better interpreted as 'Entry Risk' (do not buy) or 'Holding Risk' (reduce position).".into());
        lines.push("".into());

        lines.push("## Table 1: Risk Distribution".into());
        lines.push("".into());
        lines.push("| Risk Level | Count | % of Total |".into());
        lines.push("|------------|------:|-----------:|".into());
        lines.push(format!(
            "| Low | {} | {:.1}% |",
            review.risk_distribution.low,
            review.risk_distribution.low as f64 / review.total_records as f64 * 100.0
        ));
        lines.push(format!(
            "| Medium | {} | {:.1}% |",
            review.risk_distribution.medium,
            review.risk_distribution.medium as f64 / review.total_records as f64 * 100.0
        ));
        lines.push(format!(
            "| High | {} | {:.1}% |",
            review.risk_distribution.high,
            review.risk_distribution.high as f64 / review.total_records as f64 * 100.0
        ));
        lines.push("".into());

        lines.push("## Table 2: RiskHigh Evidence Composition".into());
        lines.push("".into());
        lines.push("| Evidence | Count | % of High Risk Records | Risk Category (Proposal) |".into());
        lines.push("|----------|------:|-----------------------:|--------------------------|".into());
        for comp in &review.high_risk_evidence_composition {
            let category = if review.semantic_mapping.holding_risk.contains(&comp.evidence_kind) {
                "Holding Risk"
            } else if review.semantic_mapping.entry_risk.contains(&comp.evidence_kind) {
                "Entry Risk"
            } else {
                "Ambiguous"
            };
            lines.push(format!(
                "| {:?} | {} | {:.1}% | {} |",
                comp.evidence_kind, comp.count, comp.pct_of_group, category
            ));
        }
        lines.push("".into());

        lines.push("## Table 3: RiskHigh Decision Context".into());
        lines.push("".into());
        let ctx = &review.high_risk_decision_context;
        lines.push(format!("- High risk records: {}", ctx.count));
        lines.push(format!(
            "- Direction: mean={:.3}, min={:.3}, p25={:.3}, p50={:.3}, p75={:.3}, max={:.3}",
            ctx.direction_summary.mean,
            ctx.direction_summary.min,
            ctx.direction_summary.p25,
            ctx.direction_summary.p50,
            ctx.direction_summary.p75,
            ctx.direction_summary.max
        ));
        lines.push(format!(
            "- Confidence: mean={:.3}, min={:.3}, p25={:.3}, p50={:.3}, p75={:.3}, max={:.3}",
            ctx.confidence_summary.mean,
            ctx.confidence_summary.min,
            ctx.confidence_summary.p25,
            ctx.confidence_summary.p50,
            ctx.confidence_summary.p75,
            ctx.confidence_summary.max
        ));
        lines.push(format!(
            "- Consensus: mean={:.3}, min={:.3}, p25={:.3}, p50={:.3}, p75={:.3}, max={:.3}",
            ctx.consensus_summary.mean,
            ctx.consensus_summary.min,
            ctx.consensus_summary.p25,
            ctx.consensus_summary.p50,
            ctx.consensus_summary.p75,
            ctx.consensus_summary.max
        ));
        lines.push(format!(
            "- Coverage: mean={:.3}, min={:.3}, p25={:.3}, p50={:.3}, p75={:.3}, max={:.3}",
            ctx.coverage_summary.mean,
            ctx.coverage_summary.min,
            ctx.coverage_summary.p25,
            ctx.coverage_summary.p50,
            ctx.coverage_summary.p75,
            ctx.coverage_summary.max
        ));
        lines.push("".into());
        lines.push("### Decision Breakdown for High Risk Records".into());
        lines.push("".into());
        lines.push("| Decision | Count |".into());
        lines.push("|----------|------:|".into());
        let mut decisions: Vec<_> = ctx.decision_breakdown.iter().collect();
        decisions.sort_by(|a, b| b.1.cmp(a.1));
        for (decision, count) in decisions {
            lines.push(format!("| {:?} | {} |", decision, count));
        }
        lines.push("".into());

        lines.push("## Table 4: RiskHigh Future Outcome Analysis".into());
        lines.push("".into());
        lines.push("| Group | Count | T+20 Mean | T+60 Mean | T+120 Mean | Negative T+20 % | Negative T+60 % | Negative T+120 % | MAE Mean | Max Drawdown Mean |".into());
        lines.push("|-------|------:|----------:|----------:|-----------:|----------------:|----------------:|-----------------:|---------:|------------------:|".into());

        for (name, outcome) in [
            ("High Risk", &review.high_risk_outcome),
            ("High Risk + Wait", &review.high_risk_wait_outcome),
            ("RiskHigh + Bearish + Wait (blocked Reduce)", &review.blocked_reduce_candidates_outcome),
            ("Medium Risk", &review.medium_risk_outcome),
            ("Low Risk", &review.low_risk_outcome),
        ] {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                name,
                outcome.count,
                fmt_pct(outcome.t20_mean),
                fmt_pct(outcome.t60_mean),
                fmt_pct(outcome.t120_mean),
                fmt_ratio(outcome.negative_t20_ratio),
                fmt_ratio(outcome.negative_t60_ratio),
                fmt_ratio(outcome.negative_t120_ratio),
                fmt_pct(outcome.mae_mean),
                fmt_pct(outcome.max_drawdown_mean)
            ));
        }
        if let Some(outcome) = &review.high_risk_reduce_outcome {
            lines.push(format!(
                "| High Risk + Reduce | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                outcome.count,
                fmt_pct(outcome.t20_mean),
                fmt_pct(outcome.t60_mean),
                fmt_pct(outcome.t120_mean),
                fmt_ratio(outcome.negative_t20_ratio),
                fmt_ratio(outcome.negative_t60_ratio),
                fmt_ratio(outcome.negative_t120_ratio),
                fmt_pct(outcome.mae_mean),
                fmt_pct(outcome.max_drawdown_mean)
            ));
        }
        lines.push("".into());

        lines.push("## Table 5: Risk Semantic Mapping Proposal".into());
        lines.push("".into());
        lines.push("| Evidence | Proposed Risk Type | Rationale |".into());
        lines.push("|----------|-------------------|-----------|".into());
        for kind in &review.semantic_mapping.entry_risk {
            lines.push(format!("| {:?} | Entry Risk | Makes opening a position dangerous |", kind));
        }
        for kind in &review.semantic_mapping.holding_risk {
            lines.push(format!("| {:?} | Holding Risk | Suggests existing positions should be reduced |", kind));
        }
        for kind in &review.semantic_mapping.ambiguous {
            lines.push(format!("| {:?} | Ambiguous / Context-dependent | Direction and timing depend on other evidence |", kind));
        }
        lines.push("".into());

        lines.push("## Blocked Reduce Candidates (RiskHigh + Wait)".into());
        lines.push("".into());
        lines.push(format!("Count: {}", review.blocked_reduce_candidates.len()));
        lines.push("".into());
        lines.push("| # | Symbol | Date | Direction | Confidence | Consensus | Risk | Decision | Evidences | T+20 | T+60 |".into());
        lines.push("|---|--------|------|-----------|------------|-----------|------|----------|-----------|------|------|".into());
        for (i, record) in review.blocked_reduce_candidates.iter().take(50).enumerate() {
            let evidences = record
                .evidences
                .iter()
                .map(|k| format!("{:?}", k))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "| {} | {} | {} | {:.3} | {:.3} | {:.3} | {:?} | {:?} | {} | {} | {} |",
                i + 1,
                record.symbol,
                record.date,
                record.dominant_direction,
                record.confidence,
                record.consensus,
                record.risk,
                record.decision_state,
                evidences,
                fmt_opt(record.outcome.t20_return),
                fmt_opt(record.outcome.t60_return)
            ));
        }
        if review.blocked_reduce_candidates.len() > 50 {
            lines.push("| ... | ... | ... | ... | ... | ... | ... | ... | ... | ... | ... |".into());
            lines.push(format!("| | | | | | | | | | ({} records total, see JSON) |", review.blocked_reduce_candidates.len()));
        }
        lines.push("".into());

        lines.push("## Interpretation & Recommendation".into());
        lines.push("".into());
        let _high_risk_negative_t20 = review.high_risk_outcome.negative_t20_ratio.unwrap_or(0.0);
        let blocked_negative_t20 = review
            .blocked_reduce_candidates_outcome
            .negative_t20_ratio
            .unwrap_or(0.0);
        let blocked_t20_mean = review.blocked_reduce_candidates_outcome.t20_mean.unwrap_or(0.0);
        let blocked_t60_mean = review.blocked_reduce_candidates_outcome.t60_mean.unwrap_or(0.0);
        if review.blocked_reduce_candidates.is_empty() {
            lines.push("No RiskHigh records were blocked from Reduce by the Risk gate. The Risk gate is not the bottleneck for Reduce.".into());
        } else if blocked_t20_mean < -0.01 {
            lines.push(format!(
                "The {} bearish RiskHigh+Wait candidates have an average T+20 return of {:.2}% and T+60 of {:.2}%, with {:.1}% negative T+20. \
                 This suggests that waiting on these bearish high-risk signals was costly on average. \
                 The current 'RiskHigh -> Wait' semantics likely suppresses necessary Reduce actions.",
                review.blocked_reduce_candidates.len(),
                blocked_t20_mean * 100.0,
                blocked_t60_mean * 100.0,
                blocked_negative_t20 * 100.0
            ));
            lines.push("".into());
            lines.push("**Recommendation**: Consider splitting RiskLevel into EntryRisk (suppress BuyNow) and HoldingRisk (drive Reduce), rather than a single gate that blocks all decisions.".into());
        } else if blocked_t20_mean > 0.01 {
            lines.push(format!(
                "The {} bearish RiskHigh+Wait candidates have an average T+20 return of {:.2}%. \
                 This suggests that waiting was profitable on average for these bearish high-risk signals. \
                 Changing 'RiskHigh -> Wait' to 'RiskHigh -> Reduce' would likely have underperformed.",
                review.blocked_reduce_candidates.len(),
                blocked_t20_mean * 100.0
            ));
            lines.push("".into());
            lines.push("**Recommendation**: Keep the current RiskHigh semantics, but address the confidence bottleneck for the 98 confidence-blocked Reduce candidates.".into());
        } else {
            lines.push(format!(
                "The {} bearish RiskHigh+Wait candidates have a near-zero average T+20 return ({:.2}%). \
                 The outcome data is mixed and does not strongly support changing RiskHigh semantics.",
                review.blocked_reduce_candidates.len(),
                blocked_t20_mean * 100.0
            ));
            lines.push("".into());
            lines.push("**Recommendation**: Collect more evidence or run a controlled Calibration experiment before changing RiskHigh semantics.".into());
        }
        lines.push("".into());
        lines.push("**No code changes are proposed here.** This is input for the 2A-5 Calibration Proposal.".into());
        lines.push("".into());

        lines.join("\n")
    }
}

fn fmt_pct(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.2}%", v * 100.0),
        None => "N/A".into(),
    }
}

fn fmt_ratio(value: Option<f64>) -> String {
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
    use crate::risk_semantics::compute_risk_semantics_review;
    use execution_engine::v2::assessment::RiskLevel;
    use execution_engine::ExecutionState;

    #[test]
    fn markdown_contains_tables() {
        let review = compute_risk_semantics_review(&[]);
        let md = RiskSemanticsFormatter::markdown(&review);
        assert!(md.contains("Risk Semantics Review"));
        assert!(md.contains("Risk Distribution"));
        assert!(md.contains("RiskHigh Evidence Composition"));
    }

    #[test]
    fn json_round_trips() {
        let review = compute_risk_semantics_review(&[]);
        let json = RiskSemanticsFormatter::json(&review);
        let restored: RiskSemanticsReview = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_records, 0);
    }
}

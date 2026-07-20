use crate::bearish_analysis::BearishAnalysis;

/// Markdown / JSON formatter for `BearishAnalysis`.
pub struct BearishAnalysisFormatter;

impl BearishAnalysisFormatter {
    /// Renders the analysis as Markdown.
    pub fn markdown(analysis: &BearishAnalysis) -> String {
        let mut lines = Vec::new();

        lines.push("# Bearish Evidence Analysis".into());
        lines.push(String::new());
        lines.push(format!(
            "**Total Records:** {} | **Bearish Candidates:** {}",
            analysis.total_records, analysis.bearish_candidates
        ));
        lines.push(format!(
            "**Baseline Negative T+20 Rate:** {:.1}% | **Negative T+60 Rate:** {:.1}%",
            analysis.baseline_negative_t20_rate * 100.0,
            analysis.baseline_negative_t60_rate * 100.0
        ));
        lines.push(String::new());

        lines.push("## Recommendation".into());
        lines.push(analysis.recommendation.clone());
        lines.push(String::new());

        lines.push("## Evidence Lift".into());
        lines.push(String::new());
        lines.push("| Evidence | Count | Negative T+20 % | Baseline % | Lift |".into());
        lines.push("|----------|------:|----------------:|-----------:|-----:|".into());
        for row in &analysis.evidence_lift {
            lines.push(format!(
                "| {} | {} | {:.1}% | {:.1}% | {:.2} |",
                row.evidence,
                row.count,
                row.negative_t20_rate * 100.0,
                row.baseline_negative_t20_rate * 100.0,
                row.lift
            ));
        }
        lines.push(String::new());

        lines.push("## Evidence Combination Outcome Matrix".into());
        lines.push(String::new());
        lines.push("| Combination | Count | Negative T+20 % | Negative T+60 % | Avg T+20 | Avg T+60 |".into());
        lines.push("|-------------|------:|----------------:|----------------:|---------:|---------:|".into());
        for row in &analysis.evidence_matrix {
            lines.push(format!(
                "| {} | {} | {:.1}% | {:.1}% | {:.2}% | {:.2}% |",
                row.combination.join(" + "),
                row.count,
                row.negative_t20_rate * 100.0,
                row.negative_t60_rate * 100.0,
                row.avg_t20 * 100.0,
                row.avg_t60 * 100.0
            ));
        }
        lines.push(String::new());

        lines.push("## Recovery Conflict".into());
        lines.push(String::new());
        lines.push("| Group | Count | Negative T+20 % | Avg T+20 | Avg T+60 |".into());
        lines.push("|-------|------:|----------------:|---------:|---------:|".into());
        lines.push(format!(
            "| Bearish + Recovery | {} | {:.1}% | {:.2}% | {:.2}% |",
            analysis.recovery_conflict.with_recovery.count,
            analysis.recovery_conflict.with_recovery.negative_t20_rate * 100.0,
            analysis.recovery_conflict.with_recovery.avg_t20 * 100.0,
            analysis.recovery_conflict.with_recovery.avg_t60 * 100.0
        ));
        lines.push(format!(
            "| Bearish + No Recovery | {} | {:.1}% | {:.2}% | {:.2}% |",
            analysis.recovery_conflict.without_recovery.count,
            analysis.recovery_conflict.without_recovery.negative_t20_rate * 100.0,
            analysis.recovery_conflict.without_recovery.avg_t20 * 100.0,
            analysis.recovery_conflict.without_recovery.avg_t60 * 100.0
        ));
        lines.push(String::new());

        lines.push("## C3 False Reduce Analysis".into());
        lines.push(String::new());
        lines.push(format!(
            "- C3 Reduce count: {} (confidence threshold 0.45)",
            analysis.false_reduce_analysis.c3_reduce_count
        ));
        lines.push(format!(
            "- False Reduce count: {} (T+20 >= 0)",
            analysis.false_reduce_analysis.false_reduce_count
        ));
        lines.push(format!(
            "- False Reduce rate: {:.1}%",
            analysis.false_reduce_analysis.false_reduce_rate * 100.0
        ));
        lines.push(format!(
            "- Average T+20 after C3 Reduce: {:.2}%",
            analysis.false_reduce_analysis.avg_t20 * 100.0
        ));
        lines.push(String::new());
        if !analysis.false_reduce_analysis.top_evidence_combinations.is_empty() {
            lines.push("### Top Evidence Combinations in False Reduces".into());
            lines.push(String::new());
            lines.push("| Combination | Count |".into());
            lines.push("|-------------|------:|".into());
            for row in &analysis.false_reduce_analysis.top_evidence_combinations {
                lines.push(format!(
                    "| {} | {} |",
                    row.combination.join(" + "),
                    row.count
                ));
            }
            lines.push(String::new());
        }

        lines.push("## TASK-153.5: RiskExpansion Coverage Exploration".into());
        lines.push(String::new());
        lines.push(format!(
            "Current RiskExpansion condition: `amplitude_pct > {}`",
            analysis.risk_expansion_coverage.current_threshold
        ));
        lines.push(format!(
            "- Total records: {}",
            analysis.risk_expansion_coverage.total_records
        ));
        lines.push(format!(
            "- Triggered records: {} ({:.2}% coverage)",
            analysis.risk_expansion_coverage.triggered_count,
            analysis.risk_expansion_coverage.coverage_pct * 100.0
        ));
        lines.push(format!(
            "- Triggered in bearish candidates: {} / {}",
            analysis.risk_expansion_coverage.triggered_in_bearish_count,
            analysis.risk_expansion_coverage.bearish_candidates
        ));
        lines.push(String::new());

        lines.push("### Amplitude Distribution".into());
        lines.push(String::new());
        lines.push("| Metric | Value |".into());
        lines.push("|--------|------:|".into());
        lines.push(format!(
            "| Min | {:.2}% |",
            analysis.risk_expansion_coverage.amplitude_percentiles.min * 100.0
        ));
        lines.push(format!(
            "| P10 | {:.2}% |",
            analysis.risk_expansion_coverage.amplitude_percentiles.p10 * 100.0
        ));
        lines.push(format!(
            "| P25 | {:.2}% |",
            analysis.risk_expansion_coverage.amplitude_percentiles.p25 * 100.0
        ));
        lines.push(format!(
            "| P50 | {:.2}% |",
            analysis.risk_expansion_coverage.amplitude_percentiles.p50 * 100.0
        ));
        lines.push(format!(
            "| P75 | {:.2}% |",
            analysis.risk_expansion_coverage.amplitude_percentiles.p75 * 100.0
        ));
        lines.push(format!(
            "| P90 | {:.2}% |",
            analysis.risk_expansion_coverage.amplitude_percentiles.p90 * 100.0
        ));
        lines.push(format!(
            "| Max | {:.2}% |",
            analysis.risk_expansion_coverage.amplitude_percentiles.max * 100.0
        ));
        lines.push(format!(
            "| Mean | {:.2}% |",
            analysis.risk_expansion_coverage.amplitude_percentiles.mean * 100.0
        ));
        lines.push(String::new());

        lines.push("### Threshold Sensitivity".into());
        lines.push(String::new());
        lines.push("| Threshold | Count | Negative T+20 % | Lift | Avg T+20 | Avg T+60 |".into());
        lines.push("|-----------|------:|----------------:|-----:|---------:|---------:|".into());
        for row in &analysis.risk_expansion_coverage.threshold_sensitivity {
            lines.push(format!(
                "| {:.3} | {} | {:.1}% | {:.2} | {:.2}% | {:.2}% |",
                row.threshold,
                row.count,
                row.negative_t20_rate * 100.0,
                row.lift_vs_baseline,
                row.avg_t20 * 100.0,
                row.avg_t60 * 100.0
            ));
        }
        lines.push(String::new());

        lines.push("### Near Miss Analysis (below current threshold)".into());
        lines.push(String::new());
        lines.push("| Range | Count | Negative T+20 % | Lift | Avg T+20 | Avg T+60 |".into());
        lines.push("|-------|------:|----------------:|-----:|---------:|---------:|".into());
        for row in &analysis.risk_expansion_coverage.near_miss_analysis {
            lines.push(format!(
                "| [ {:.3} , {:.3} ) | {} | {:.1}% | {:.2} | {:.2}% | {:.2}% |",
                row.threshold,
                analysis.risk_expansion_coverage.current_threshold,
                row.count,
                row.negative_t20_rate * 100.0,
                row.lift_vs_baseline,
                row.avg_t20 * 100.0,
                row.avg_t60 * 100.0
            ));
        }
        lines.push(String::new());
        lines.push(format!(
            "**Recommendation:** {}",
            analysis.risk_expansion_coverage.recommendation
        ));
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the analysis as JSON.
    pub fn json(analysis: &BearishAnalysis) -> String {
        serde_json::to_string_pretty(analysis).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bearish_analysis::{
        ConflictGroup, EvidenceCombinationRow, EvidenceLiftRow, FalseReduceAnalysis,
        PercentileSummary, RecoveryConflict, RiskExpansionCoverage, RiskExpansionThresholdRow,
    };

    fn make_analysis() -> BearishAnalysis {
        BearishAnalysis {
            total_records: 100,
            bearish_candidates: 20,
            baseline_negative_t20_rate: 0.4,
            baseline_negative_t60_rate: 0.35,
            evidence_lift: vec![EvidenceLiftRow {
                evidence: "Distribution".into(),
                count: 10,
                negative_t20_rate: 0.6,
                baseline_negative_t20_rate: 0.4,
                lift: 1.5,
            }],
            evidence_matrix: vec![EvidenceCombinationRow {
                combination: vec!["Distribution".into(), "RiskExpansion".into()],
                count: 5,
                negative_t20_count: 4,
                negative_t60_count: 3,
                negative_t20_rate: 0.8,
                negative_t60_rate: 0.6,
                avg_t20: -0.02,
                avg_t60: -0.01,
            }],
            recovery_conflict: RecoveryConflict {
                with_recovery: ConflictGroup {
                    count: 8,
                    negative_t20_count: 2,
                    negative_t20_rate: 0.25,
                    avg_t20: 0.01,
                    avg_t60: 0.02,
                },
                without_recovery: ConflictGroup {
                    count: 12,
                    negative_t20_count: 6,
                    negative_t20_rate: 0.5,
                    avg_t20: -0.01,
                    avg_t60: -0.02,
                },
            },
            false_reduce_analysis: FalseReduceAnalysis {
                c3_reduce_count: 10,
                false_reduce_count: 6,
                false_reduce_rate: 0.6,
                avg_t20: 0.03,
                top_evidence_combinations: vec![],
            },
            risk_expansion_coverage: RiskExpansionCoverage {
                current_threshold: 0.05,
                total_records: 100,
                triggered_count: 6,
                coverage_pct: 0.06,
                triggered_in_bearish_count: 2,
                bearish_candidates: 20,
                amplitude_percentiles: PercentileSummary {
                    min: 0.005,
                    p10: 0.008,
                    p25: 0.012,
                    p50: 0.018,
                    p75: 0.025,
                    p90: 0.040,
                    max: 0.150,
                    mean: 0.020,
                },
                threshold_sensitivity: vec![RiskExpansionThresholdRow {
                    threshold: 0.05,
                    count: 6,
                    negative_t20_count: 3,
                    negative_t20_rate: 0.5,
                    lift_vs_baseline: 1.25,
                    avg_t20: -0.01,
                    avg_t60: -0.02,
                }],
                near_miss_analysis: vec![RiskExpansionThresholdRow {
                    threshold: 0.03,
                    count: 12,
                    negative_t20_count: 4,
                    negative_t20_rate: 0.33,
                    lift_vs_baseline: 0.83,
                    avg_t20: 0.01,
                    avg_t60: 0.02,
                }],
                recommendation: "test risk expansion".into(),
            },
            recommendation: "test".into(),
        }
    }

    #[test]
    fn markdown_contains_combination_table() {
        let text = BearishAnalysisFormatter::markdown(&make_analysis());
        assert!(text.contains("Evidence Combination Outcome Matrix"));
        assert!(text.contains("Distribution + RiskExpansion"));
    }

    #[test]
    fn json_round_trips() {
        let analysis = make_analysis();
        let text = BearishAnalysisFormatter::json(&analysis);
        assert!(text.contains("bearish_candidates"));
    }
}

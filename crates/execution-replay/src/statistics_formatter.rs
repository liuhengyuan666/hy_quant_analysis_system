use execution_engine::v2::evidence::EvidenceKind;
use execution_engine::ExecutionState;
use serde_json;

use crate::statistics::{
    AssessmentHistograms, DecisionDistribution, EvidenceFrequency, EvidencePairMatrix,
    ExecutionStatistics, OutcomeBucket, OutcomeMatrix, PriorDistribution,
};

/// Presentation-layer formatter for `ExecutionStatistics`.
///
/// Statistics belongs to the Domain; this module only converts it to consumer
/// formats (JSON, Markdown). No computation or interpretation happens here.
pub struct ExecutionStatisticsFormatter;

impl ExecutionStatisticsFormatter {
    /// Returns a compact JSON representation.
    pub fn json(stats: &ExecutionStatistics) -> String {
        serde_json::to_string_pretty(stats).unwrap_or_else(|_| "{}".into())
    }

    /// Returns a human-readable Markdown report.
    pub fn markdown(stats: &ExecutionStatistics) -> String {
        let mut lines = Vec::new();

        lines.push("# Execution Statistics".into());
        lines.push("".into());
        lines.push(format!("**Records:** {}", stats.meta.record_count));
        if let Some(scope) = &stats.meta.scope {
            lines.push(format!("**Scope:** {}", scope));
        }
        if let (Some(from), Some(to)) = (stats.meta.from_date, stats.meta.to_date) {
            lines.push(format!("**Period:** {} ~ {}", from, to));
        }
        lines.push(format!(
            "**Engine Version:** {}",
            stats.meta.execution_engine_version
        ));
        if let Some(hash) = &stats.meta.policy_hash {
            lines.push(format!("**Policy Hash:** {}", hash));
        }
        lines.push(format!("**Generated At:** {}", stats.meta.generated_at));
        lines.push("".into());

        lines.append(&mut Self::format_decision_distribution(&stats.decision_distribution));
        lines.push("".into());
        lines.append(&mut Self::format_prior_distribution(&stats.prior_distribution));
        lines.push("".into());
        lines.append(&mut Self::format_evidence_frequency(&stats.evidence_frequency));
        lines.push("".into());
        lines.append(&mut Self::format_evidence_pairs(&stats.evidence_pairs));
        lines.push("".into());
        lines.append(&mut Self::format_assessment_histograms(&stats.assessment_histograms));
        lines.push("".into());
        lines.append(&mut Self::format_outcome_matrix(&stats.outcome_matrix));

        lines.join("\n")
    }

    fn format_decision_distribution(dist: &DecisionDistribution) -> Vec<String> {
        let mut lines = vec!["## Decision Distribution".into(), "".into()];
        let total = dist.total();
        lines.push(format!("Total: {}", total));
        lines.push("".into());
        lines.push("| State | Count | Ratio |".into());
        lines.push("|-------|------:|------:|".into());
        for state in [
            ExecutionState::BuyNow,
            ExecutionState::Wait,
            ExecutionState::Reduce,
        ] {
            let count = dist.counts.get(&state).copied().unwrap_or(0);
            let ratio = dist.ratio(state);
            lines.push(format!("| {:?} | {} | {:.2}% |", state, count, ratio * 100.0));
        }
        lines
    }

    fn format_prior_distribution(dist: &PriorDistribution) -> Vec<String> {
        let mut lines = vec!["## Prior Distribution".into(), "".into()];
        let total = dist.total();
        if total == 0 {
            lines.push("No Prior Evidence found.".into());
            return lines;
        }
        lines.push(format!("Total: {}", total));
        lines.push("".into());
        lines.push("| Prior | Count | Ratio |".into());
        lines.push("|-------|------:|------:|".into());
        let mut entries: Vec<(&String, &usize)> = dist.counts.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        for (label, count) in entries {
            let ratio = dist.ratio(label);
            lines.push(format!("| {} | {} | {:.2}% |", label, count, ratio * 100.0));
        }
        lines
    }

    fn format_evidence_frequency(freq: &EvidenceFrequency) -> Vec<String> {
        let mut lines = vec!["## Evidence Frequency".into(), "".into()];
        let total = freq.total();
        if total == 0 {
            lines.push("No Evidence found.".into());
            return lines;
        }
        lines.push(format!("Total Evidence: {}", total));
        lines.push("".into());
        lines.push("| Evidence | Count | Ratio |".into());
        lines.push("|----------|------:|------:|".into());
        let mut entries: Vec<(&EvidenceKind, &usize)> = freq.counts.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        for (kind, count) in entries {
            let ratio = freq.ratio(*kind);
            lines.push(format!("| {:?} | {} | {:.2}% |", kind, count, ratio * 100.0));
        }
        lines
    }

    fn format_evidence_pairs(pairs: &EvidencePairMatrix) -> Vec<String> {
        let mut lines = vec!["## Evidence Pair Matrix".into(), "".into()];
        if pairs.pairs.is_empty() {
            lines.push("No evidence pairs found.".into());
            return lines;
        }
        lines.push("| Pair | Count |".into());
        lines.push("|------|------:|".into());
        let mut entries: Vec<(&String, &usize)> = pairs.pairs.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        for (pair, count) in entries {
            lines.push(format!("| {} | {} |", pair, count));
        }
        lines
    }

    fn format_assessment_histograms(hists: &AssessmentHistograms) -> Vec<String> {
        let mut lines = vec!["## Assessment Histograms".into(), "".into()];
        lines.push(format!("Bins: {}", hists.bin_count));
        lines.push("".into());

        lines.push("### Confidence".into());
        lines.append(&mut Self::format_histogram(&hists.confidence));
        lines.push("".into());

        lines.push("### Consensus".into());
        lines.append(&mut Self::format_histogram(&hists.consensus));
        lines.push("".into());

        lines.push("### Coverage".into());
        lines.append(&mut Self::format_histogram(&hists.coverage));
        lines.push("".into());

        lines.push("### Risk".into());
        lines.push("| Risk Level | Count |".into());
        lines.push("|------------|------:|".into());
        let mut entries: Vec<(&String, &usize)> = hists.risk.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        for (label, count) in entries {
            lines.push(format!("| {} | {} |", label, count));
        }

        lines
    }

    fn format_histogram(values: &[usize]) -> Vec<String> {
        let mut lines = Vec::new();
        let max = values.iter().copied().max().unwrap_or(0).max(1);
        let width = 40;
        for (i, &count) in values.iter().enumerate() {
            let bar_len = if max > 0 {
                (count as f64 / max as f64 * width as f64) as usize
            } else {
                0
            };
            let bar: String = std::iter::repeat("█").take(bar_len).collect();
            let lower = i as f64 / values.len() as f64;
            let upper = (i + 1) as f64 / values.len() as f64;
            lines.push(format!("[{:.1} - {:.1}): {} {}", lower, upper, count, bar));
        }
        lines
    }

    fn format_outcome_matrix(matrix: &OutcomeMatrix) -> Vec<String> {
        let mut lines = vec!["## Outcome Matrix".into(), "".into()];
        lines.push("| Decision | Hit | Miss | TooEarly | TooLate | Unknown |".into());
        lines.push("|----------|----:|-----:|---------:|--------:|--------:|".into());
        for state in [
            ExecutionState::BuyNow,
            ExecutionState::Wait,
            ExecutionState::Reduce,
        ] {
            let row_total = matrix.row_total(state);
            if row_total == 0 {
                continue;
            }
            let hit = matrix.get(state, OutcomeBucket::Hit);
            let miss = matrix.get(state, OutcomeBucket::Miss);
            let too_early = matrix.get(state, OutcomeBucket::TooEarly);
            let too_late = matrix.get(state, OutcomeBucket::TooLate);
            let unknown = matrix.get(state, OutcomeBucket::Unknown);
            lines.push(format!(
                "| {:?} | {} | {} | {} | {} | {} |",
                state, hit, miss, too_early, too_late, unknown
            ));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statistics::{
        EvidenceFrequency, ExecutionStatisticsMeta,
    };
    use chrono::Utc;

    fn make_empty_stats() -> ExecutionStatistics {
        ExecutionStatistics {
            meta: ExecutionStatisticsMeta {
                record_count: 0,
                scope: Some("CN".into()),
                from_date: None,
                to_date: None,
                generated_at: Utc::now(),
                execution_engine_version: "v2.0.0-mvp".into(),
                policy_hash: None,
            },
            evidence_frequency: EvidenceFrequency::default(),
            evidence_pairs: crate::statistics::EvidencePairMatrix::default(),
            decision_distribution: DecisionDistribution::default(),
            prior_distribution: PriorDistribution::default(),
            assessment_histograms: crate::statistics::AssessmentHistograms::new(10),
            outcome_matrix: OutcomeMatrix::default(),
        }
    }

    #[test]
    fn markdown_contains_scope() {
        let stats = make_empty_stats();
        let md = ExecutionStatisticsFormatter::markdown(&stats);
        assert!(md.contains("CN"));
        assert!(md.contains("Decision Distribution"));
        assert!(md.contains("Evidence Frequency"));
    }

    #[test]
    fn json_round_trips() {
        let stats = make_empty_stats();
        let json = ExecutionStatisticsFormatter::json(&stats);
        let restored: ExecutionStatistics = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.meta.record_count, 0);
        assert_eq!(restored.meta.scope, Some("CN".into()));
    }
}

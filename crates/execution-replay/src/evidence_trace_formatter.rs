use execution_engine::ExecutionState;
use serde_json;

use crate::evidence_trace::EvidenceTrace;

/// Presentation-layer formatter for `EvidenceTrace`.
pub struct EvidenceTraceFormatter;

impl EvidenceTraceFormatter {
    /// Returns compact JSON.
    pub fn json(trace: &EvidenceTrace) -> String {
        serde_json::to_string_pretty(trace).unwrap_or_else(|_| "{}".into())
    }

    /// Returns a Markdown funnel report.
    pub fn markdown(trace: &EvidenceTrace) -> String {
        let mut lines = Vec::new();

        lines.push("# Evidence Trace / Funnel".into());
        lines.push("".into());
        lines.push(format!("**Records:** {}", trace.meta.record_count));
        if let Some(scope) = &trace.meta.scope {
            lines.push(format!("**Scope:** {}", scope));
        }
        if let (Some(from), Some(to)) = (trace.meta.from_date, trace.meta.to_date) {
            lines.push(format!("**Period:** {} ~ {}", from, to));
        }
        lines.push(format!("**Generated At:** {}", trace.meta.generated_at));
        lines.push("".into());

        lines.push("## Funnel by EvidenceKind".into());
        lines.push("".into());
        lines.push("| Evidence | Obs | Obs→Evd | Evd | Evd→Asm | Asm | Wait | BuyNow | Reduce |".into());
        lines.push("|----------|----:|--------:|----:|--------:|----:|-----:|-------:|-------:|".into());

        for row in &trace.rows {
            let obs_to_evd = row.observation_to_evidence_retention();
            let evd_to_asm = row.evidence_to_assessment_retention();
            let wait = row.decision_share(ExecutionState::Maintain);
            let buy = row.decision_share(ExecutionState::Increase);
            let reduce = row.decision_share(ExecutionState::Reduce);
            let asm = row.in_assessment_count();

            lines.push(format!(
                "| {:?} | {} | {:.1}% | {} | {:.1}% | {} | {:.1}% | {:.1}% | {:.1}% |",
                row.evidence_kind,
                row.observation_count,
                obs_to_evd * 100.0,
                row.evidence_count,
                evd_to_asm * 100.0,
                asm,
                wait * 100.0,
                buy * 100.0,
                reduce * 100.0,
            ));
        }

        lines.push("".into());
        lines.push("## Legend".into());
        lines.push("".into());
        lines.push("- **Obs**: observations of this kind (only for observation-derived evidences)".into());
        lines.push("- **Obs→Evd**: share of observations that became evidence items".into());
        lines.push("- **Evd**: evidence items produced".into());
        lines.push("- **Evd→Asm**: share of evidence items that reached Assessment (supporting/conflicting/neutral)".into());
        lines.push("- **Asm**: records where this evidence reached Assessment".into());
        lines.push("- **Wait / BuyNow / Reduce**: share of records with this evidence that ended in each decision".into());
        lines.push("".into());

        lines.push("## Focus: Reduce Evidence".into());
        lines.push("".into());
        let reduce_kinds = [
            execution_engine::v2::evidence::EvidenceKind::RiskExpansion,
            execution_engine::v2::evidence::EvidenceKind::Distribution,
            execution_engine::v2::evidence::EvidenceKind::MomentumFailure,
            execution_engine::v2::evidence::EvidenceKind::LiquidityConfirmation,
        ];
        for kind in reduce_kinds {
            if let Some(row) = trace.get(kind) {
                lines.push(format!("### {:?}", kind));
                lines.push(format!("- Observations: {}", row.observation_count));
                lines.push(format!("- Evidence items: {}", row.evidence_count));
                lines.push(format!("- In Assessment: {}", row.in_assessment_count()));
                lines.push(format!("- Supporting: {}", row.supporting_count));
                lines.push(format!("- Conflicting: {}", row.conflicting_count));
                lines.push(format!("- Neutral: {}", row.neutral_count));
                lines.push(format!(
                    "- Decisions: Wait={} BuyNow={} Reduce={}",
                    row.decision_counts.get(&ExecutionState::Maintain).unwrap_or(&0),
                    row.decision_counts.get(&ExecutionState::Increase).unwrap_or(&0),
                    row.decision_counts.get(&ExecutionState::Reduce).unwrap_or(&0),
                ));
                lines.push("".into());
            }
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_trace::{compute_evidence_trace, EvidenceTraceMeta};

    #[test]
    fn markdown_contains_header() {
        let trace = EvidenceTrace {
            meta: EvidenceTraceMeta {
                record_count: 0,
                scope: None,
                from_date: None,
                to_date: None,
                generated_at: chrono::Utc::now(),
            },
            rows: vec![],
        };
        let md = EvidenceTraceFormatter::markdown(&trace);
        assert!(md.contains("Evidence Trace / Funnel"));
    }

    #[test]
    fn json_round_trips() {
        let trace = EvidenceTrace {
            meta: EvidenceTraceMeta {
                record_count: 0,
                scope: None,
                from_date: None,
                to_date: None,
                generated_at: chrono::Utc::now(),
            },
            rows: vec![],
        };
        let json = EvidenceTraceFormatter::json(&trace);
        let restored: EvidenceTrace = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.meta.record_count, 0);
    }
}

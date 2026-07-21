use crate::evidence_registry::{EvidenceDescriptor, EvidenceRegistry};

/// Markdown / JSON formatter for `EvidenceRegistry`.
pub struct EvidenceRegistryFormatter;

impl EvidenceRegistryFormatter {
    /// Renders the registry as Markdown.
    pub fn markdown(registry: &EvidenceRegistry) -> String {
        let mut lines = Vec::new();
        lines.push("# Evidence Horizon Registry".into());
        lines.push(String::new());
        lines.push(format!("**Version:** {}", registry.version));
        lines.push(format!("**Total Assets:** {}", registry.assets.len()));
        lines.push(format!(
            "**Validated:** {} | **Conditional:** {} | **Rejected:** {}",
            registry
                .assets
                .iter()
                .filter(|a| a.validation_status == crate::evidence_registry::ValidationStatus::Validated)
                .count(),
            registry
                .assets
                .iter()
                .filter(|a| a.validation_status == crate::evidence_registry::ValidationStatus::Conditional)
                .count(),
            registry
                .assets
                .iter()
                .filter(|a| a.validation_status == crate::evidence_registry::ValidationStatus::Rejected)
                .count()
        ));
        lines.push(String::new());

        lines.push("## Assets".into());
        lines.push(String::new());
        lines.push("| Evidence | Role | Horizon | Status | Standalone | Decision Candidate | Metrics | Validation Requirement | Dependencies |".into());
        lines.push("|----------|------|---------|--------|------------|--------------------|---------|------------------------|--------------|".into());
        for asset in &registry.assets {
            let metrics = asset
                .target_metric
                .as_ref()
                .map(|m| {
                    format!(
                        "precision={:.1}% lift={:.2} n={}",
                        m.precision * 100.0,
                        m.lift,
                        m.sample_count
                    )
                })
                .unwrap_or_else(|| "-".into());
            let req = asset
                .validation_requirement
                .as_ref()
                .map(|r| {
                    format!(
                        "n>={} p>={:.0}% l>={:.1} fa<={:.0}%",
                        r.min_samples,
                        r.min_precision * 100.0,
                        r.min_lift,
                        r.max_false_alarm * 100.0
                    )
                })
                .unwrap_or_else(|| "-".into());
            let deps = if asset.dependencies.is_empty() {
                "-".into()
            } else {
                asset
                    .dependencies
                    .iter()
                    .map(|d| format!("{}", d))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                asset.name,
                format!("{:?}", asset.role),
                format!("{:?}", asset.horizon),
                format!("{:?}", asset.validation_status),
                if asset.standalone_validity { "Yes" } else { "No" },
                if asset.decision_candidate { "Yes" } else { "No" },
                metrics,
                req,
                deps
            ));
        }
        lines.push(String::new());

        lines.push("## Decision-Ready Bundles".into());
        lines.push(String::new());
        let candidates: Vec<&EvidenceDescriptor> = registry.decision_candidates();
        if candidates.is_empty() {
            lines.push("No validated decision candidates in registry.".into());
        } else {
            for c in candidates {
                lines.push(format!(
                    "- **{}** ({:?}, {:?}): precision={:.1}%, lift={:.2}",
                    c.name,
                    c.role,
                    c.horizon,
                    c.target_metric.as_ref().map(|m| m.precision * 100.0).unwrap_or(0.0),
                    c.target_metric.as_ref().map(|m| m.lift).unwrap_or(0.0)
                ));
            }
        }
        lines.push(String::new());

        lines.push("## Validated Assets (Meet Validation Requirement)".into());
        lines.push(String::new());
        let validated: Vec<&EvidenceDescriptor> = registry.validated_assets();
        if validated.is_empty() {
            lines.push("No assets meet validation requirements.".into());
        } else {
            for v in validated {
                lines.push(format!(
                    "- **{}** ({:?}, {:?}): n={}, precision={:.1}%, lift={:.2}",
                    v.name,
                    v.role,
                    v.horizon,
                    v.target_metric.as_ref().map(|m| m.sample_count).unwrap_or(0),
                    v.target_metric.as_ref().map(|m| m.precision * 100.0).unwrap_or(0.0),
                    v.target_metric.as_ref().map(|m| m.lift).unwrap_or(0.0)
                ));
            }
        }
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the registry as JSON.
    pub fn json(registry: &EvidenceRegistry) -> String {
        serde_json::to_string_pretty(registry).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_registry::EvidenceRegistry;

    #[test]
    fn markdown_contains_registry_header() {
        let registry = EvidenceRegistry::v8_default();
        let text = EvidenceRegistryFormatter::markdown(&registry);
        assert!(text.contains("Evidence Horizon Registry"));
        assert!(text.contains("LeadershipDecay"));
    }

    #[test]
    fn json_round_trips() {
        let registry = EvidenceRegistry::v8_default();
        let text = EvidenceRegistryFormatter::json(&registry);
        assert!(text.contains("LeadershipDecay"));
        assert!(text.contains("HOLDING_RISK"));
    }
}

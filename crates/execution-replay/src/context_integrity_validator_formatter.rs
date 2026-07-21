use crate::context_integrity_validator::{ContextIntegrityValidation, FieldValidation};

/// Markdown / JSON formatter for `ContextIntegrityValidation`.
///
/// This formatter is designed for CI gate output: it surfaces only the fields
/// that failed and the specific contract violations, so a developer can quickly
/// understand why Evidence Modeling is blocked.
pub struct ContextIntegrityValidatorFormatter;

impl ContextIntegrityValidatorFormatter {
    /// Renders the gate result as Markdown.
    pub fn markdown(validation: &ContextIntegrityValidation) -> String {
        let mut lines = Vec::new();

        lines.push("# ExecutionContext Fact Integrity Gate".into());
        lines.push(String::new());
        lines.push(format!("**Status:** {}", if validation.passed { "PASS" } else { "FAIL" }));
        lines.push(format!("**Total Records:** {}", validation.total_records));
        lines.push(format!(
            "**Fields Audited:** {} | **Failed:** {}",
            validation.fields.len(),
            validation.fields.iter().filter(|f| !f.passed).count()
        ));
        lines.push(String::new());

        lines.push("## Verdict".into());
        lines.push(validation.verdict.clone());
        lines.push(String::new());

        let failed: Vec<&FieldValidation> = validation.fields.iter().filter(|f| !f.passed).collect();
        if !failed.is_empty() {
            lines.push("## Failed Fields".into());
            lines.push(String::new());
            lines.push("| Field | Violations | Min | Max | Variance | Unique Ratio | Dominant Ratio |".into());
            lines.push("|-------|------------|----:|----:|---------:|-------------:|----------------:|".into());
            for field in failed {
                let violations = field
                    .violations
                    .iter()
                    .map(format_violation)
                    .collect::<Vec<_>>()
                    .join("; ");
                lines.push(format!(
                    "| {} | {} | {:.2} | {:.2} | {:.4e} | {:.4e} | {:.2}% |",
                    field.field_name,
                    violations,
                    field.min,
                    field.max,
                    field.variance,
                    field.unique_ratio,
                    field.dominant_value_ratio * 100.0
                ));
            }
            lines.push(String::new());
        }

        lines.push("## All Fields".into());
        lines.push(String::new());
        lines.push("| Field | Status | Unique | Variance | Unique Ratio | Dominant Ratio |".into());
        lines.push("|-------|--------|-------:|---------:|-------------:|----------------:|".into());
        for field in &validation.fields {
            lines.push(format!(
                "| {} | {} | {} | {:.4e} | {:.4e} | {:.2}% |",
                field.field_name,
                if field.passed { "PASS" } else { "FAIL" },
                field.unique_values,
                field.variance,
                field.unique_ratio,
                field.dominant_value_ratio * 100.0
            ));
        }
        lines.push(String::new());

        lines.join("\n")
    }

    /// Renders the gate result as JSON.
    pub fn json(validation: &ContextIntegrityValidation) -> String {
        serde_json::to_string_pretty(validation).unwrap_or_default()
    }
}

fn format_violation(violation: &crate::context_integrity_validator::ContextIntegrityViolation) -> String {
    use crate::context_integrity_validator::ContextIntegrityViolation;
    match violation {
        ContextIntegrityViolation::Placeholder { value } => format!("placeholder={:.2}", value),
        ContextIntegrityViolation::LowVariance { variance, threshold } => {
            format!("variance={:.4e}<threshold={:.4e}", variance, threshold)
        }
        ContextIntegrityViolation::LowUniqueRatio { ratio, threshold } => {
            format!("unique_ratio={:.4e}<threshold={:.4e}", ratio, threshold)
        }
        ContextIntegrityViolation::HighDominantValueRatio {
            ratio,
            threshold,
            value,
        } => {
            format!(
                "dominant_ratio={:.2}%>threshold={:.2}% at value={:.2}",
                ratio * 100.0,
                threshold * 100.0,
                value
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_integrity_validator::{
        validate_execution_context, ContextIntegrityViolation, FieldValidation,
    };

    fn minimal_validation() -> ContextIntegrityValidation {
        ContextIntegrityValidation {
            total_records: 100,
            fields: vec![FieldValidation {
                field_name: "breadth.breadth_pct".into(),
                sample_count: 100,
                unique_values: 1,
                unique_ratio: 0.01,
                dominant_value: Some(50.0),
                dominant_value_ratio: 1.0,
                min: 50.0,
                max: 50.0,
                mean: 50.0,
                variance: 0.0,
                known_placeholder_detected: true,
                placeholder_value: Some(50.0),
                violations: vec![ContextIntegrityViolation::Placeholder { value: 50.0 }],
                passed: false,
            }],
            passed: false,
            verdict: "test".into(),
        }
    }

    #[test]
    fn markdown_contains_fail_status() {
        let v = minimal_validation();
        let text = ContextIntegrityValidatorFormatter::markdown(&v);
        assert!(text.contains("FAIL"));
        assert!(text.contains("breadth.breadth_pct"));
        assert!(text.contains("placeholder=50.00"));
    }

    #[test]
    fn json_round_trips() {
        let v = minimal_validation();
        let text = ContextIntegrityValidatorFormatter::json(&v);
        assert!(text.contains("passed"));
        assert!(text.contains("breadth.breadth_pct"));
    }
}

use anyhow::{Context, Result};
use app_service::{AppContext, ReportScope};
use chrono::NaiveDate;
use execution_replay::{
    DecisionGateFormatter, DecisionMarginFormatter, DistributionCoverageFormatter,
    EvidenceTraceFormatter, ExecutionStatisticsFormatter, ValidationFormatter, ValidationReportFormatter,
};
use std::path::PathBuf;

/// V8 Execution Platform Validation CLI — single historical case replay.
///
/// This is not a user-facing production command. It is a validation tool used
/// to verify that the Execution Platform produces a complete, explainable, and
/// reproducible `ExecutionResearchRecord` for a historical symbol/date.
pub fn handle_validate_execution_replay(
    context: &AppContext,
    symbol: String,
    date: NaiveDate,
    scope: ReportScope,
    output: String,
) -> Result<()> {
    let record = context.validate_execution_replay(&symbol, date, scope)?;
    let formatter = ValidationFormatter;
    let text = formatter.format(&record, &output);
    println!("{}", text);
    Ok(())
}

/// V8 Execution Platform Validation CLI — run an entire golden suite.
///
/// This command is the platform-level regression entry point. It loads a
/// `ValidationSuite`, runs every case against the Execution Platform, and
/// produces a pass/fail summary. It is intended for CI and ADR review, not for
/// end-user trading workflows.
pub fn handle_validate_execution_suite(
    context: &AppContext,
    suite_path: PathBuf,
    output: String,
) -> Result<()> {
    let summary = context.validate_execution_suite(&suite_path)?;
    let formatter = ValidationReportFormatter;
    let text = match output.to_lowercase().as_str() {
        "json" => formatter.format_json(&summary),
        "detail" => formatter.format_detail(&summary),
        _ => formatter.format_summary(&summary),
    };
    println!("{}", text);
    Ok(())
}

/// V8 Execution Platform Validation CLI — discover historical candidates.
///
/// Scans persisted market data for all symbol/date combinations that have
/// complete inputs (signal, strategy state, daily bar) and can therefore be
/// evaluated by the Execution Platform. The output is used to select real-world
/// golden cases for the validation suite.
pub fn handle_find_validation_candidates(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScope,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let candidates = context.find_validation_candidates(from, to, scope, decision_filter.as_deref())?;

    let text = match output.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&candidates).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e)),
        _ => format_candidates_markdown(&candidates),
    };
    println!("{}", text);
    Ok(())
}

fn format_candidates_markdown(candidates: &[execution_replay::ValidationCandidate]) -> String {
    use std::collections::BTreeMap;

    let mut by_decision: BTreeMap<String, Vec<&execution_replay::ValidationCandidate>> = BTreeMap::new();
    for candidate in candidates {
        by_decision
            .entry(candidate.decision_state.clone())
            .or_default()
            .push(candidate);
    }

    let mut lines = Vec::new();
    lines.push("# Validation Candidates".to_string());
    lines.push(format!("Total: {}\n", candidates.len()));

    for (decision, items) in &by_decision {
        lines.push(format!("## {} ({})", decision, items.len()));
        lines.push("| Date | Symbol | Signal | Score | Strategy | Regime | Conf | Risk | Evidences |".to_string());
        lines.push("|------|--------|--------|-------|----------|--------|------|------|-----------|".to_string());
        for item in items.iter().take(20) {
            lines.push(format!(
                "| {} | {} | {} | {:.1} | {} | {} | {:.2} | {:?} | {} |",
                item.date, item.symbol, item.signal_label, item.signal_score,
                item.strategy_state, item.market_regime_label, item.confidence,
                item.risk, item.evidence_count
            ));
        }
        if items.len() > 20 {
            lines.push(format!("| ... | | | | | | | | ({} more) |", items.len() - 20));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

/// V8 Execution Platform — compute Execution Statistics over a set of records.
///
/// Supports two input paths:
/// - A Golden Suite file (`--suite`), treated as a Representative Sample.
/// - A historical date range (`--from`, `--to`, `--scope`), treated as the Full Population.
///
/// Output is either JSON or Markdown and contains only empirical facts; no
/// calibration conclusions or recommendations.
pub fn handle_execution_statistics(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let stats = match suite_path {
        Some(path) => context.execution_statistics_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_statistics_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => ExecutionStatisticsFormatter::json(&stats),
        _ => ExecutionStatisticsFormatter::markdown(&stats),
    };
    println!("{}", text);
    Ok(())
}

/// V8 Execution Platform — compute an Evidence Trace over a set of records.
///
/// Traces each EvidenceKind through Observation → Evidence → Assessment →
/// Decision. This is a root-cause tool, not a user-facing trading command.
pub fn handle_execution_evidence_trace(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let trace = match suite_path {
        Some(path) => context.execution_evidence_trace_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_evidence_trace_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => EvidenceTraceFormatter::json(&trace),
        _ => EvidenceTraceFormatter::markdown(&trace),
    };
    println!("{}", text);
    Ok(())
}

/// 2A-4A: Distribution Coverage Review.
pub fn handle_execution_distribution_coverage(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let review = match suite_path {
        Some(path) => context.execution_distribution_coverage_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_distribution_coverage_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => DistributionCoverageFormatter::json(&review),
        _ => DistributionCoverageFormatter::markdown(&review),
    };
    println!("{}", text);
    Ok(())
}

/// 2A-4B: Decision Margin Review.
pub fn handle_execution_decision_margin(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let review = match suite_path {
        Some(path) => context.execution_decision_margin_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_decision_margin_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => DecisionMarginFormatter::json(&review),
        _ => DecisionMarginFormatter::markdown(&review),
    };
    println!("{}", text);
    Ok(())
}

/// 2A-4C/2A-4.5: Decision Gate Analysis.
pub fn handle_execution_decision_gate(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.execution_decision_gate_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_decision_gate_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => DecisionGateFormatter::json(&analysis),
        _ => DecisionGateFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

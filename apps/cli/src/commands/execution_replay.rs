use anyhow::{Context, Result};
use app_service::{AppContext, ReportScope};
use chrono::NaiveDate;
use execution_replay::{
    BearishAnalysisFormatter, CalibrationFormatter, ConfirmationDecayFormatter,
    ContextIntegrityAuditFormatter, ContextIntegrityValidatorFormatter, DecisionGateFormatter,
    DecisionMarginFormatter, DistributionCoverageFormatter, EvidenceRegistryFormatter,
    EvidenceTraceFormatter, ExecutionStatisticsFormatter, HoldingRiskBundleFormatter,
    HoldingRiskCalibrationFormatter, HoldingRiskPersistenceFormatter, LeadershipDecayHorizonFormatter,
    LiquidityPressureFormatter, RegimeRiskFormatter, RiskLifecycleFormatter, RiskSemanticsFormatter,
    ShadowDeploymentFormatter, ShadowModeFormatter, StateRiskAccelerationFormatter,
    TransitionAnalysisFormatter, ValidationFormatter, ValidationReportFormatter,
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

    let text = if output.to_lowercase() == "json" {
        serde_json::to_string_pretty(&candidates).unwrap_or_default()
    } else {
        let mut lines = vec!["# Validation Candidates".to_string(), String::new()];
        lines.push("| Symbol | Date | Scope | Signal | State | Decision | Confidence |".to_string());
        lines.push("|--------|------|-------|--------|-------|----------|------------:|".to_string());
        for c in &candidates {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {:.2} |",
                c.symbol, c.date, c.scope, c.signal_label, c.strategy_state, c.decision_state, c.confidence
            ));
        }
        lines.join("\n")
    };
    println!("{}", text);
    Ok(())
}

/// TASK-160.2A: LiquidityPressure Research Asset — sustained capital pressure
/// (turnover decay + price weakness + breadth not recovering + persistence).
pub fn handle_liquidity_pressure(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
    consecutive_days: usize,
    volume_delta_threshold: f64,
    price_weakness: bool,
    breadth_weakness: bool,
    volume_level_threshold: Option<f64>,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.liquidity_pressure_from_suite(&path, consecutive_days, volume_delta_threshold, price_weakness, breadth_weakness, volume_level_threshold)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.liquidity_pressure_from_range(from, to, scope, decision_filter.as_deref(), consecutive_days, volume_delta_threshold, price_weakness, breadth_weakness, volume_level_threshold)?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => LiquidityPressureFormatter::json(&analysis),
        _ => LiquidityPressureFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-161: Holding Risk Calibration v2 — compute HoldingRiskScore and validate
/// with score buckets, regime split, and walk-forward validation.
pub fn handle_holding_risk_calibration(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.holding_risk_calibration_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.holding_risk_calibration_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => HoldingRiskCalibrationFormatter::json(&analysis),
        _ => HoldingRiskCalibrationFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-163: Holding Risk Lifecycle Analysis — build a risk state machine around
/// HoldingRiskScore: entry, peak, recovery, duration, and false alarm analysis.
pub fn handle_risk_lifecycle(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.risk_lifecycle_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.risk_lifecycle_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => RiskLifecycleFormatter::json(&analysis),
        _ => RiskLifecycleFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-166: Regime-Aware State Risk Model — identify when the market is ALREADY
/// in a dangerous state (State Detector), not 'deteriorating' transitions.
pub fn handle_regime_risk(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.regime_risk_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.regime_risk_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => RegimeRiskFormatter::json(&analysis),
        _ => RegimeRiskFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-168: State Risk Acceleration Model — identify accelerating-decline
/// conditions (not oversold/mean-reversion).
pub fn handle_state_risk_acceleration(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.state_risk_acceleration_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.state_risk_acceleration_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => StateRiskAccelerationFormatter::json(&analysis),
        _ => StateRiskAccelerationFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-167: Shadow Mode Runtime Wiring — generate daily shadow-mode output using
/// market_regime_label as State Context and HoldingRiskScore as Transition Evidence.
pub fn handle_shadow_mode(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let report = match suite_path {
        Some(path) => context.shadow_mode_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.shadow_mode_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => ShadowModeFormatter::json(&report),
        _ => ShadowModeFormatter::markdown(&report),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-169: Shadow Deployment Contract — generate daily ShadowRiskAssessment
/// using market_regime_label as State Context and HoldingRiskScore as Transition
/// Evidence. Explicitly prohibited for DecisionEngine consumption.
pub fn handle_shadow_deployment(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let report = match suite_path {
        Some(path) => context.shadow_deployment_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.shadow_deployment_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => ShadowDeploymentFormatter::json(&report),
        _ => ShadowDeploymentFormatter::markdown(&report),
    };
    println!("{}", text);
    Ok(())
}


/// TASK-160.3: Evidence Horizon Registry — view the canonical Evidence Asset
/// registry with role / horizon / validation status / dependencies.
pub fn handle_evidence_registry(output: String) -> Result<()> {
    let registry = execution_replay::EvidenceRegistry::v8_default();
    let text = match output.to_lowercase().as_str() {
        "json" => EvidenceRegistryFormatter::json(&registry),
        _ => EvidenceRegistryFormatter::markdown(&registry),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-160.3: Validate a bundle of evidence ids against the Evidence Horizon
/// Registry. Returns error if dependencies are missing or evidence is unknown.
pub fn handle_evidence_validate_bundle(evidence_ids: Vec<String>, output: String) -> Result<()> {
    let registry = execution_replay::EvidenceRegistry::v8_default();
    let ids: Vec<execution_replay::EvidenceId> = evidence_ids
        .iter()
        .map(|s| {
            let normalized = s.to_lowercase();
            match normalized.as_str() {
                "leadershipdecay" | "leadership_decay" | "leadership-decay" => {
                    Ok(execution_replay::EvidenceId::LeadershipDecay)
                }
                "liquiditypressure" | "liquidity_pressure" | "liquidity-pressure" => {
                    Ok(execution_replay::EvidenceId::LiquidityPressure)
                }
                "confirmationdecay" | "confirmation_decay" | "confirmation-decay" => {
                    Ok(execution_replay::EvidenceId::ConfirmationDecay)
                }
                "breadthdeterioration" | "breadth_deterioration" | "breadth-deterioration" => {
                    Ok(execution_replay::EvidenceId::BreadthDeterioration)
                }
                "recoveryfailure" | "recovery_failure" | "recovery-failure" => {
                    Ok(execution_replay::EvidenceId::RecoveryFailure)
                }
                _ => anyhow::bail!("unknown evidence id: {}", s),
            }
        })
        .collect::<Result<Vec<_>>>()?;

    match registry.validate_bundle(&ids) {
        Ok(()) => {
            let decision_ready = registry.is_bundle_decision_ready(&ids);
            let text = match output.to_lowercase().as_str() {
                "json" => {
                    format!("{{\"valid\": true, \"decision_ready\": {}}}", decision_ready)
                }
                _ => {
                    if decision_ready {
                        "Bundle is valid and decision-ready.".into()
                    } else {
                        "Bundle is valid but not decision-ready.".into()
                    }
                }
            };
            println!("{}", text);
            Ok(())
        }
        Err(e) => {
            let text = match output.to_lowercase().as_str() {
                "json" => format!("{{\"valid\": false, \"error\": \"{}\"}}", e),
                _ => format!("Bundle is invalid: {}", e),
            };
            println!("{}", text);
            anyhow::bail!("bundle validation failed");
        }
    }
}


/// TASK-160.2B: ConfirmationDecay Research Asset — change-based confirmation
/// analysis (delta/velocity/persistence + optional price weakness).
pub fn handle_confirmation_decay(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
    delta_5d_threshold: f64,
    slope_10d_threshold: f64,
    min_consecutive_days: usize,
    price_weakness: bool,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.confirmation_decay_from_suite(&path, delta_5d_threshold, slope_10d_threshold, min_consecutive_days, price_weakness)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.confirmation_decay_from_range(from, to, scope, decision_filter.as_deref(), delta_5d_threshold, slope_10d_threshold, min_consecutive_days, price_weakness)?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => ConfirmationDecayFormatter::json(&analysis),
        _ => ConfirmationDecayFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-160.2B: Holding Risk Bundle V4 — add ConfirmationDecay as Confirmatory
/// Dimension to the V3 bundle.
pub fn handle_execution_holding_risk_bundle_v4(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.holding_risk_bundle_v4_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.holding_risk_bundle_v4_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => HoldingRiskBundleFormatter::json(&analysis),
        _ => HoldingRiskBundleFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}


/// TASK-160.2A: Holding Risk Bundle V3 — combine LeadershipDecay persistence,
/// LiquidityPressure, and BreadthDeterioration into a medium-term (T+60) holding
/// risk score.
pub fn handle_execution_holding_risk_bundle_v3(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.holding_risk_bundle_v3_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.holding_risk_bundle_v3_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => HoldingRiskBundleFormatter::json(&analysis),
        _ => HoldingRiskBundleFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
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

/// 2A-4C: Risk Semantics Review.
pub fn handle_execution_risk_semantics(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let review = match suite_path {
        Some(path) => context.execution_risk_semantics_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_risk_semantics_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => RiskSemanticsFormatter::json(&review),
        _ => RiskSemanticsFormatter::markdown(&review),
    };
    println!("{}", text);
    Ok(())
}

/// 2A-5: Directional Confidence Calibration Experiment.
pub fn handle_execution_calibration(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let review = match suite_path {
        Some(path) => context.execution_calibration_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_calibration_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => CalibrationFormatter::json(&review),
        _ => CalibrationFormatter::markdown(&review),
    };
    println!("{}", text);
    Ok(())
}

/// 2B-1: Bearish Evidence Analysis.
///
/// Analyzes existing bearish Assessment candidates and their Evidence composition
/// against historical outcomes. This is a read-only research tool used to discover
/// Exit-specific patterns before any new EvidenceKind is introduced.
pub fn handle_execution_bearish_analysis(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.execution_bearish_analysis_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_bearish_analysis_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => BearishAnalysisFormatter::json(&analysis),
        _ => BearishAnalysisFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// 2B-2: Transition Evidence Modeling — discover change/deterioration signals from
/// existing records and outcomes. Research-only; does not modify any Observation,
/// Evidence, Assessment, Decision, or Policy code.
pub fn handle_execution_transition_analysis(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    candidate: String,
    output: String,
) -> Result<()> {
    let candidate: execution_replay::TransitionCandidate = candidate
        .parse()
        .map_err(|e: String| anyhow::anyhow!("invalid --candidate value: {}", e))?;

    let analysis = match suite_path {
        Some(path) => context.execution_transition_analysis_from_suite(&path, candidate)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_transition_analysis_from_range(from, to, scope, decision_filter.as_deref(), candidate)?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => TransitionAnalysisFormatter::json(&analysis),
        _ => TransitionAnalysisFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// 2B-0: ResearchContext Fact Integrity Audit — verify all ResearchContext-derived
/// fields before any Evidence Modeling work.
pub fn handle_execution_context_integrity_audit(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let report = match suite_path {
        Some(path) => context.execution_context_integrity_audit_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_context_integrity_audit_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => ContextIntegrityAuditFormatter::json(&report),
        _ => ContextIntegrityAuditFormatter::markdown(&report),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-159: Context Integrity Gate — strict pass/fail firewall for the
/// ResearchContext → ExecutionEvent fact lineage. Exits with non-zero status
/// when the gate fails, so it can be used as a CI step.
pub fn handle_execution_context_integrity_gate(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
    strict: bool,
    live: bool,
) -> Result<()> {
    let validation = if live {
        let scope = scope.context("--scope required when --live is used")?;
        context.execution_context_live_integrity_check(scope)?
    } else {
        match suite_path {
            Some(path) => context.execution_context_integrity_gate_from_suite(&path)?,
            None => {
                let from = from.context("--from required when --suite is not provided")?;
                let to = to.context("--to required when --suite is not provided")?;
                let scope = scope.context("--scope required when --suite is not provided")?;
                context.execution_context_integrity_gate_from_range(from, to, scope, decision_filter.as_deref())?
            }
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => ContextIntegrityValidatorFormatter::json(&validation),
        _ => ContextIntegrityValidatorFormatter::markdown(&validation),
    };
    println!("{}", text);

    if strict && !validation.passed {
        anyhow::bail!("Context Integrity Gate FAILED. Evidence Modeling is blocked.");
    }
    Ok(())
}
/// LeadershipDecay signal. Research-only; does not modify the Execution Pipeline.
pub fn handle_execution_leadership_decay_horizon(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.execution_leadership_decay_horizon_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_leadership_decay_horizon_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => LeadershipDecayHorizonFormatter::json(&analysis),
        _ => LeadershipDecayHorizonFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// 2B-3: Holding Risk Evidence Bundle — combine LeadershipDecay, BreadthDeterioration,
/// and LiquidityDeterioration into a medium-term (T+60) holding risk score.
/// Research-only; does not modify the Execution Pipeline.
pub fn handle_execution_holding_risk_bundle(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.execution_holding_risk_bundle_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.execution_holding_risk_bundle_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => HoldingRiskBundleFormatter::json(&analysis),
        _ => HoldingRiskBundleFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-160.1: Holding Risk Persistence Analysis — test whether sustained
/// LeadershipDecay is a stronger medium-term Holding Risk signal than a single-day
/// snapshot. Research-only; does not modify the Execution Pipeline.
pub fn handle_holding_risk_persistence(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.holding_risk_persistence_from_suite(&path)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.holding_risk_persistence_from_range(from, to, scope, decision_filter.as_deref())?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => HoldingRiskPersistenceFormatter::json(&analysis),
        _ => HoldingRiskPersistenceFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

/// TASK-160.1: Holding Risk Bundle V2 — persistence-aware combination of
/// LeadershipDecay, BreadthDeterioration, and LiquidityDeterioration into a
/// medium-term (T+60) holding risk score.
pub fn handle_execution_holding_risk_bundle_v2(
    context: &AppContext,
    suite_path: Option<PathBuf>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    scope: Option<ReportScope>,
    decision_filter: Option<String>,
    output: String,
    min_leadership_persistence_days: usize,
) -> Result<()> {
    let analysis = match suite_path {
        Some(path) => context.holding_risk_bundle_v2_from_suite(&path, min_leadership_persistence_days)?,
        None => {
            let from = from.context("--from required when --suite is not provided")?;
            let to = to.context("--to required when --suite is not provided")?;
            let scope = scope.context("--scope required when --suite is not provided")?;
            context.holding_risk_bundle_v2_from_range(from, to, scope, decision_filter.as_deref(), min_leadership_persistence_days)?
        }
    };

    let text = match output.to_lowercase().as_str() {
        "json" => HoldingRiskBundleFormatter::json(&analysis),
        _ => HoldingRiskBundleFormatter::markdown(&analysis),
    };
    println!("{}", text);
    Ok(())
}

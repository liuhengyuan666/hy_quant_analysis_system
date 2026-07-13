use anyhow::{Context, Result};
use app_service::{AppContext, ReportScope, ResearchSnapshot};
use chrono::{Local, NaiveDate, Utc};
use core_domain::AnalysisScope;
use core_domain::research::percentile::percentile_rank;
use market_fingerprint_engine;
use report_builder::{
    AnalyticsReportInput, ConfirmationReportInput, ExplanationReportInput, RecoveryReportInput,
    ReviewReportInput, ReviewProfile, ReviewReportBuilder,
    ResearchReportBuilder, SrdReportInput, StretchReportInput,
};
use report_renderer::{self, MarkdownFormatter, TextFormatter};
use reporting::{Formatter, ReportingSnapshot};
use std::collections::BTreeMap;
use crate::ReportScopeArg;

/// Render an analyze_with_action result as markdown
pub fn render_action_result_md(value: &serde_json::Value) -> String {
    let mut md = String::new();

    md.push_str(&format!(
        "# Research Analysis: {}\n\n",
        value["action"].as_str().unwrap_or("unknown")
    ));

    if let Some(scope) = value["scope"].as_str() {
        md.push_str(&format!("**Scope**: {}\n\n", scope));
    }

    if value["placeholder"].as_bool().unwrap_or(false) {
        md.push_str(
            "> **Warning**: This analysis was generated in placeholder mode. \
             No real LLM provider was configured.\n\n",
        );
    }

    if let Some(content) = value["markdown"].as_str() {
        md.push_str(content);
        md.push_str("\n\n");
    }

    md
}

pub fn handle_list_actions() -> Result<()> {
    println!("Available Research Actions:");
    println!("  - market_story: 市场叙事");
    println!("  - explain_decision: 解释决策");
    println!("  - preclose_review: 收盘前复核");
    println!("  - risk_view: 风险视角");
    println!("  - devils_advocate: 唱反调");
    Ok(())
}

pub fn handle_benchmark_action(
    _context: &AppContext,
    action: String,
    provider_config: String,
    runs: usize,
    format: String,
    scope: ReportScopeArg,
    quiet: bool,
) -> Result<()> {
    if !quiet {
        eprintln!("[benchmark] Action '{}' is not yet benchmarkable after Research Layer refactor.", action);
        eprintln!("[benchmark] Scope: {:?}, Providers: {}, Runs: {}", scope, provider_config, runs);
    }
    println!("Benchmark not yet implemented for new ResearchAction architecture.");
    println!("Format: {}", format);
    Ok(())
}

pub fn compute_srd_input(
    context: &AppContext,
    scope: core_domain::AnalysisScope,
    date: Option<NaiveDate>,
) -> Result<(SrdReportInput, ReportingSnapshot)> {
    // 1. Resolve target date
    let target_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "signal_snapshot")
            .unwrap_or(None)
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    // 2. Build ResearchContext + ResearchSnapshot from a single dataset fetch
    let (research_ctx, snapshot) = context.build_research_bundle_for_date(target_date, scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx,
    };

    // 4. Compute SRD metrics from snapshot
    let strong_buy_count = snapshot.strong_buy_count();
    let buy_count = snapshot.buy_count();
    let avg_signal = snapshot.average_signal();
    let duration = snapshot.divergence_duration();
    let breadth_trend = snapshot.breadth_trend();
    let rotation_pattern = snapshot.rotation_pattern();
    let historical_percentile = snapshot.signal_percentile();
    let state_label = snapshot.state_label();

    // 5. Narrative helpers
    let interpretation = srd_interpretation(
        strong_buy_count,
        buy_count,
        breadth_trend,
        rotation_pattern,
        &state_label,
    );
    let confidence = srd_confidence(strong_buy_count, buy_count, breadth_trend, duration);

    let input = SrdReportInput {
        strong_buy_count,
        buy_count,
        average_signal: avg_signal,
        duration,
        breadth_trend: breadth_trend.to_string(),
        rotation_pattern: rotation_pattern.to_string(),
        historical_percentile,
        interpretation,
        confidence: confidence.to_string(),
        state_label,
    };

    Ok((input, reporting_snapshot))
}

/// SRD (Signal-Regime Divergence): observation tool.
/// Reports statistics when strategy_state is conservative (NO_TRADE/DE_RISK/LEFT_PROBE)
/// AND top signals contain StrongBuy/Buy labels.
pub fn handle_research_srd(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
) -> Result<()> {
    let scope: core_domain::AnalysisScope = match scope_arg {
        ReportScopeArg::Global => core_domain::AnalysisScope::Global,
        ReportScopeArg::Cn => core_domain::AnalysisScope::Cn,
        ReportScopeArg::Hk => core_domain::AnalysisScope::Hk,
    };

    let (input, reporting_snapshot) = compute_srd_input(context, scope, date)?;

    let doc = ResearchReportBuilder::build_srd(&reporting_snapshot, &input)?;
    let mut formatter = TextFormatter::new();
    report_renderer::render(&mut formatter, &doc);
    println!("{}", formatter.finalize());

    Ok(())
}

pub fn handle_analyze(
    context: AppContext,
    action: String,
    scope: ReportScopeArg,
    format: String,
    _deterministic: bool,
    _seed: u64,
) -> Result<()> {
    let scope: ReportScope = scope.into();
    let result = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime");
        runtime.block_on(context.analyze_with_action(&action, scope))
    })
    .join()
    .expect("LLM analysis thread panicked")?;

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        "markdown" => {
            let md = render_action_result_md(&result);
            println!("{}", md);
        }
        _ => {
            anyhow::bail!(
                "Unsupported format: {}. Use 'json' or 'markdown'",
                format
            );
        }
    }
    Ok(())
}

/// One-sentence interpretation for SRD output.
fn srd_interpretation(
    strong_buy: usize,
    buy: usize,
    breadth_trend: &str,
    rotation_pattern: &str,
    state_label: &str,
) -> String {
    let signal_strength = if strong_buy >= 5 {
        "very strong"
    } else if strong_buy >= 3 {
        "strong"
    } else if strong_buy + buy >= 3 {
        "moderate"
    } else {
        "limited"
    };

    let breadth_word = match breadth_trend {
        "Improving" => "with improving breadth",
        "Weakening" => "but breadth is weakening",
        _ => "with stable breadth",
    };

    let rotation_word = match rotation_pattern {
        "Technology Dominant" => "concentrated in technology",
        "Defensive" => "rotating defensively",
        _ => "across mixed themes",
    };

    format!(
        "Signals are {} while Strategy remains {} ({} {}, {}).",
        signal_strength, state_label, breadth_word, rotation_word,
        if strong_buy >= 3 {
            "a pattern often seen around early trend transitions"
        } else {
            "suggesting a tentative rather than confirmed shift"
        }
    )
}

/// Confidence label for SRD based on signal breadth and duration.
fn srd_confidence(strong_buy: usize, buy: usize, breadth_trend: &str, duration: i64) -> &'static str {
    let bullish_count = strong_buy + buy;
    let breadth_ok = breadth_trend == "Improving" || breadth_trend == "Neutral";
    let duration_ok = duration >= 2;

    match (bullish_count >= 5, breadth_ok, duration_ok) {
        (true, true, true) => "High",
        (true, true, false) | (false, true, true) | (true, false, true) => "Moderate",
        _ => "Low",
    }
}

/// One-sentence interpretation for Stretch output.
fn stretch_interpretation(
    overall: &str,
    crowding: &str,
    breadth: &str,
    momentum: &str,
) -> String {
    match (overall, crowding, breadth, momentum) {
        ("Extreme", "Extreme", "Normal", "Extreme") => {
            "Momentum has accelerated rapidly and crowding is high, but broad participation remains healthy — current stretch resembles early acceleration rather than late-stage exhaustion.".to_string()
        }
        ("Extreme", _, "Extreme", "Extreme") => {
            "Both momentum and breadth are stretched simultaneously; this is a more uniform (and riskier) form of market heat.".to_string()
        }
        ("Extreme", "Extreme", _, _) => {
            "Crowding is extreme: a large share of momentum is concentrated in a few themes, raising the risk of a sharp unwind.".to_string()
        }
        ("Extreme", _, _, "Extreme") => {
            "Momentum is extreme relative to history, suggesting rapid price appreciation that may be due for consolidation.".to_string()
        }
        ("Elevated", _, _, _) => {
            "The market shows moderate stretch; conditions are warmer than average but not yet broadly extreme.".to_string()
        }
        _ => {
            "Stretch readings are within normal ranges; no material overheating is evident.".to_string()
        }
    }
}

/// Risk level label derived from Stretch dimensions.
fn stretch_risk_level(overall: &str, breadth: &str) -> &'static str {
    match (overall, breadth) {
        ("Extreme", "Extreme") => "High",
        ("Extreme", "Normal") | ("Extreme", "Elevated") => "Moderate-High",
        ("Elevated", _) => "Moderate",
        _ => "Low",
    }
}

pub fn compute_stretch_input(
    context: &AppContext,
    scope: core_domain::AnalysisScope,
    date: Option<NaiveDate>,
) -> Result<(StretchReportInput, ReportingSnapshot)> {
    use chrono::Duration;

    let report_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "rotation_rank")?
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    let (research_ctx, snapshot) = context.build_research_bundle_for_date(report_date, scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx,
    };
    if snapshot.rotations.is_empty() {
        anyhow::bail!(
            "No rotation data available for date {} and scope {:?}",
            report_date, scope
        );
    }

    let (crowding_level, concentration_pct, _) = snapshot.stretch_crowding();
    let (momentum_level, rs120_max, top5_rs120_avg) = snapshot.stretch_momentum();
    let (breadth_level, breadth_pct, breadth_sma5) = snapshot.stretch_breadth();
    let leverage_level = snapshot.stretch_leverage();
    let (overall, _weighted_score) = snapshot.stretch_overall();

    let start_date = report_date - Duration::days(120);
    let hist_rotations = market_store::fetch_rotation_ranks_for_range(
        &context.storage,
        start_date,
        report_date,
    )?;

    let mut date_map: BTreeMap<NaiveDate, Vec<f64>> = BTreeMap::new();
    for r in &hist_rotations {
        date_map.entry(r.date).or_default().push(r.momentum_score);
    }

    let hist_concentrations: Vec<f64> = date_map.values().filter_map(|scores| {
        if scores.is_empty() { return None; }
        let mut s = scores.clone();
        s.sort_by(|a, b| b.total_cmp(a));
        let total: f64 = s.iter().sum();
        if total <= 0.0 { return None; }
        let top5_sum: f64 = s.iter().take(5).sum();
        Some((top5_sum / total) * 100.0)
    }).collect();

    let crowding_percentile = if hist_concentrations.len() > 5 {
        let mut sorted_hist = hist_concentrations.clone();
        sorted_hist.sort_by(|a, b| a.total_cmp(b));
        percentile_rank(concentration_pct, &sorted_hist)
    } else {
        f64::NAN
    };

    let overall_evidence = {
        let mut parts: Vec<String> = Vec::new();
        if crowding_level != "Normal" {
            parts.push(format!("Crowding = {}", crowding_level));
        }
        if momentum_level != "Normal" {
            parts.push(format!("Momentum = {}", momentum_level));
        }
        if breadth_level != "Normal" {
            parts.push(format!("Breadth = {}", breadth_level));
        }
        if leverage_level != "Normal" {
            parts.push(format!("Leverage = {}", leverage_level));
        }
        if parts.is_empty() {
            "All dimensions within normal ranges".to_string()
        } else {
            parts.join("; ")
        }
    };

    let stretch_interp = stretch_interpretation(overall, crowding_level, breadth_level, momentum_level);
    let risk_level = stretch_risk_level(overall, breadth_level);

    let input = StretchReportInput {
        overall: overall.to_string(),
        crowding_level: crowding_level.to_string(),
        crowding_concentration_pct: concentration_pct,
        crowding_percentile,
        breadth_level: breadth_level.to_string(),
        breadth_pct,
        breadth_sma5,
        momentum_level: momentum_level.to_string(),
        rs120_max,
        rs120_top5_avg: top5_rs120_avg,
        leverage_level: leverage_level.to_string(),
        interpretation: stretch_interp,
        risk_level: risk_level.to_string(),
        overall_evidence,
    };

    Ok((input, reporting_snapshot))
}

/// Market Stretch analysis — pure observation tool
pub fn handle_research_stretch(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
) -> Result<()> {
    let scope: core_domain::AnalysisScope = match scope_arg {
        ReportScopeArg::Global => core_domain::AnalysisScope::Global,
        ReportScopeArg::Cn => core_domain::AnalysisScope::Cn,
        ReportScopeArg::Hk => core_domain::AnalysisScope::Hk,
    };

    let (input, reporting_snapshot) = compute_stretch_input(context, scope, date)?;

    let doc = ResearchReportBuilder::build_stretch(&reporting_snapshot, &input)?;
    let mut formatter = TextFormatter::new();
    report_renderer::render(&mut formatter, &doc);
    println!("{}", formatter.finalize());

    Ok(())
}

pub fn compute_analytics_input(
    context: &AppContext,
    condition: &str,
    horizon: usize,
    scope: AnalysisScope,
) -> Result<(AnalyticsReportInput, ReportingSnapshot, core_domain::research::attribution::Evidence)> {
    if horizon != 20 && horizon != 60 {
        anyhow::bail!(
            "Analytics MVP only supports --horizon 20 or 60 (got {})",
            horizon
        );
    }

    let anchor_symbol = match scope {
        AnalysisScope::Global | AnalysisScope::Cn => "000300",
        AnalysisScope::Hk => "HSCEI",
    };

    let anchor_bars = market_store::fetch_daily_bars(&context.storage, anchor_symbol)?;
    let close_by_date: BTreeMap<NaiveDate, f64> =
        anchor_bars.iter().map(|b| (b.date, b.close)).collect();

    let Some(target_date) = close_by_date.keys().last().copied() else {
        anyhow::bail!("No anchor bars available for {}", anchor_symbol);
    };
    let Some(earliest_date) = close_by_date.keys().next().copied() else {
        anyhow::bail!("No anchor bars available for {}", anchor_symbol);
    };

    let evidence = context.research_condition_evidence(
        condition,
        scope,
        horizon,
        earliest_date,
        target_date,
    )?;

    let occurrences = evidence.occurrences;
    let (forward_return_median, forward_return_mean, forward_return_best, forward_return_worst, positive_ratio, median_max_drawdown) =
        if occurrences > 0 {
            let mut sorted_returns = evidence.forward_returns.clone();
            sorted_returns.sort_by(|a, b| a.total_cmp(b));
            let mut sorted_dds = evidence.max_drawdowns.clone();
            sorted_dds.sort_by(|a, b| a.total_cmp(b));

            let median = regime_audit::common::percentile(&sorted_returns, 0.50);
            let mean = evidence.forward_returns.iter().sum::<f64>() / occurrences as f64;
            let worst = sorted_returns.first().copied().unwrap_or(0.0);
            let best = sorted_returns.last().copied().unwrap_or(0.0);
            let p_ratio = evidence.forward_returns.iter().filter(|&&r| r > 0.0).count() as f64 / occurrences as f64;
            let median_dd = regime_audit::common::percentile(&sorted_dds, 0.50);
            (median, mean, best, worst, p_ratio, median_dd)
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        };

    let research_ctx = context.build_research_context_for_date(target_date, scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx,
    };

    let input = AnalyticsReportInput {
        condition: condition.to_string(),
        horizon,
        history_window: format!("{} ~ {}", earliest_date, target_date),
        occurrences,
        forward_return_median,
        forward_return_mean,
        forward_return_best,
        forward_return_worst,
        positive_ratio,
        median_max_drawdown,
    };

    Ok((input, reporting_snapshot, evidence))
}

/// Conditional forward-return analytics — answers "what happened after this condition historically".
/// MVP: only supports `--horizon 20|60` and two hard-coded reproducible conditions.
pub fn handle_research_analytics(
    context: &AppContext,
    condition: String,
    horizon: usize,
    scope_arg: ReportScopeArg,
    save_evidence: bool,
) -> Result<()> {
    let scope: AnalysisScope = match scope_arg {
        ReportScopeArg::Global => AnalysisScope::Global,
        ReportScopeArg::Cn => AnalysisScope::Cn,
        ReportScopeArg::Hk => AnalysisScope::Hk,
    };

    let (input, reporting_snapshot, evidence) = compute_analytics_input(context, &condition, horizon, scope)?;

    let doc = ResearchReportBuilder::build_analytics(&reporting_snapshot, &input)?;
    let mut formatter = TextFormatter::new();
    report_renderer::render(&mut formatter, &doc);
    println!("{}", formatter.finalize());

    // Optionally persist the evidence asset to the workspace for downstream replay/audit.
    if save_evidence {
        let workspace = app_service::workspace::WorkspaceManager::default_workspace()
            .context("Failed to initialize workspace")?;
        let evidence_id = workspace.write_evidence(
            &evidence,
            &condition,
            scope,
            horizon,
            "replay",
            app_service::workspace::ResearchAssetLifecycle::Draft,
        )?;
        println!("Evidence saved: {}", evidence_id);
    }

    Ok(())
}

/// V7 Workflow — Observe: aggregate SRD, Stretch, Analytics, and Health into a single observation report.
/// This is the primary daily research observation entry point; it does not modify any decision logic.
pub fn handle_research_observe(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
    condition: String,
    horizon: usize,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let scope: AnalysisScope = match scope_arg {
        ReportScopeArg::Global => AnalysisScope::Global,
        ReportScopeArg::Cn => AnalysisScope::Cn,
        ReportScopeArg::Hk => AnalysisScope::Hk,
    };

    let target_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "signal_snapshot")
            .unwrap_or(None)
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    let (srd_input, srd_snapshot) = compute_srd_input(context, scope, Some(target_date))?;
    let (stretch_input, stretch_snapshot) = compute_stretch_input(context, scope, Some(target_date))?;
    let (analytics_input, analytics_snapshot, _evidence) =
        compute_analytics_input(context, &condition, horizon, scope)?;
    let health = context.check_data_health()?;

    let srd_doc = ResearchReportBuilder::build_srd(&srd_snapshot, &srd_input)?;
    let stretch_doc = ResearchReportBuilder::build_stretch(&stretch_snapshot, &stretch_input)?;
    let analytics_doc = ResearchReportBuilder::build_analytics(&analytics_snapshot, &analytics_input)?;

    let mut srd_formatter = TextFormatter::new();
    report_renderer::render(&mut srd_formatter, &srd_doc);
    let srd_md = srd_formatter.finalize();

    let mut stretch_formatter = TextFormatter::new();
    report_renderer::render(&mut stretch_formatter, &stretch_doc);
    let stretch_md = stretch_formatter.finalize();

    let mut analytics_formatter = TextFormatter::new();
    report_renderer::render(&mut analytics_formatter, &analytics_doc);
    let analytics_md = analytics_formatter.finalize();

    let mut combined = String::new();
    combined.push_str(&format!(
        "# Research Observation Report: {} ({}\n\n",
        scope.as_str(),
        target_date
    ));
    combined.push_str("## Data Health\n\n");
    combined.push_str(&format!(
        "- Freshest market date: {:?}\n",
        health.freshest_market_date
    ));
    combined.push_str(&format!(
        "- Symbols on freshest date: {}/{}\n",
        health.symbols_on_freshest_market_date, health.checked_symbols
    ));
    combined.push_str(&format!(
        "- Healthy symbols: {}, Review: {}, Critical: {}\n",
        health.healthy_symbols, health.review_symbols, health.critical_symbols
    ));
    combined.push_str(&format!(
        "- Healthy macro sources: {}, Review: {}, Critical: {}\n",
        health.healthy_macro_sources, health.review_macro_sources, health.critical_macro_sources
    ));
    combined.push_str("\n");
    combined.push_str("## Signal-Regime Divergence (SRD)\n\n");
    combined.push_str(&srd_md);
    combined.push_str("\n\n");
    combined.push_str("## Market Stretch\n\n");
    combined.push_str(&stretch_md);
    combined.push_str("\n\n");
    combined.push_str("## Conditional Analytics\n\n");
    combined.push_str(&analytics_md);

    let output_path = match output {
        Some(p) => p,
        None => std::path::PathBuf::from(format!(
            "reports/research-observe-{}-{}.md",
            scope.as_str().to_lowercase(),
            target_date
        )),
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {:?}", parent))?;
    }

    std::fs::write(&output_path, combined)
        .with_context(|| format!("Failed to write observation report to {:?}", output_path))?;

    println!("Research observation report written to: {}", output_path.display());

    Ok(())
}

/// V7.4 / ADR-078 Research Explanation — Phase 1 architecture-only.
///
/// Explains why a given condition (e.g., srd-strong) performs differently across
/// regimes. In Phase 1 this produces the full report structure but does not yet
/// implement concrete attribution dimensions (TASK-104).
pub fn handle_research_explain(
    context: &AppContext,
    condition: String,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
    _horizon: usize,
) -> Result<()> {
    let scope: AnalysisScope = match scope_arg {
        ReportScopeArg::Global => AnalysisScope::Global,
        ReportScopeArg::Cn => AnalysisScope::Cn,
        ReportScopeArg::Hk => AnalysisScope::Hk,
    };

    let target_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "signal_snapshot")
            .unwrap_or(None)
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    // Build ResearchContext for metadata and observation fields.
    let research_ctx = context.build_research_context_for_date(target_date, scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx,
    };

    let state_label = reporting_snapshot.research.market_state.label.clone();
    let signal_summary = format!(
        "{} bullish / {} strong-buy / avg-score {:.1}",
        reporting_snapshot.research.signal.bullish_count,
        reporting_snapshot.research.signal.strong_buy_count,
        reporting_snapshot.research.signal.average_score
    );
    let breadth_pct = Some(reporting_snapshot.research.breadth.breadth_pct);
    let liquidity_score = reporting_snapshot.research.market_state.liquidity_score;
    let macro_regime = Some(reporting_snapshot.research.market_state.label.clone());

    // Use a sensible default horizon for the explanation; the CLI surface may
    // expose this later. For now we explain on the 20-day forward horizon.
    let horizon = if _horizon == 20 || _horizon == 60 {
        _horizon
    } else {
        20
    };

    // Compute real evidence over the full available history up to the target date.
    // AppService clamps the window to stored anchor bars.
    let evidence = context.research_condition_evidence(
        &condition,
        scope,
        horizon,
        NaiveDate::from_ymd_opt(1900, 1, 1).unwrap(),
        target_date,
    )?;

    let registry = core_domain::research::attribution::mvp_registry();
    let observation = core_domain::research::attribution::Observation {
        state: state_label,
        signal_summary,
        breadth_pct,
        liquidity_score: Some(liquidity_score),
        macro_regime,
    };
    let mut explanation = core_domain::research::attribution::build_explanation(
        &condition,
        scope.as_str(),
        target_date,
        observation,
        evidence,
        &registry,
        vec![],
        "Validate with 90-day Shadow Production; compare against candidate evidence from Historical Replay.",
    );

    // Enrich explanation with MVP-specific hypothesis/confidence/limitations.
    explanation.hypothesis =
        core_domain::research::attribution::generate_hypothesis(&condition, &explanation.attributions);
    explanation.confidence =
        core_domain::research::attribution::generate_confidence(&explanation.attributions);
    explanation.limitations =
        core_domain::research::attribution::generate_limitations(&explanation.attributions);

    let input = ExplanationReportInput {
        condition: explanation.condition,
        observation_state: explanation.observation.state,
        observation_signal_summary: explanation.observation.signal_summary,
        observation_breadth_pct: explanation.observation.breadth_pct,
        observation_liquidity_score: explanation.observation.liquidity_score,
        observation_macro_regime: explanation.observation.macro_regime,
        evidence_occurrences: explanation.evidence.occurrences,
        evidence_history_window: explanation.evidence.history_window,
        evidence_positive_ratio: explanation.evidence.positive_ratio,
        evidence_median_forward_return: explanation.evidence.median_forward_return,
        attributions: explanation.attributions,
        hypothesis: explanation.hypothesis,
        confidence: explanation.confidence,
        limitations: explanation.limitations,
        next_validation: explanation.next_validation,
    };

    let doc = ResearchReportBuilder::build_explanation(&reporting_snapshot, &input)?;
    let mut formatter = TextFormatter::new();
    report_renderer::render(&mut formatter, &doc);
    println!("{}", formatter.finalize());

    Ok(())
}

/// Quarterly Review — aggregate SRD/Stretch/Analytics over a window into a Markdown report.
/// This is a synthesis layer: it only observes and reports, never modifies decision logic.
pub fn handle_research_review(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    use std::collections::HashMap;

    let analysis_scope = match scope_arg {
        ReportScopeArg::Global => AnalysisScope::Global,
        ReportScopeArg::Cn => AnalysisScope::Cn,
        ReportScopeArg::Hk => AnalysisScope::Hk,
    };

    // Resolve window
    let window_to = match to {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "signal_snapshot")?
            .unwrap_or_else(|| Local::now().date_naive()),
    };
    let window_from = match from {
        Some(d) => d,
        None => window_to - chrono::Duration::days(90),
    };

    if window_from > window_to {
        anyhow::bail!("--from must be on or before --to");
    }

    // Build snapshots for each date in the window. Skip dates that lack data.
    let mut snapshots: Vec<(NaiveDate, ResearchSnapshot)> = Vec::new();
    let mut d = window_from;
    while d <= window_to {
        match context.build_research_snapshot_for_date(d, analysis_scope) {
            Ok(snapshot) if !snapshot.rotations.is_empty() => snapshots.push((d, snapshot)),
            _ => {}
        }
        d += chrono::Duration::days(1);
    }

    if snapshots.is_empty() {
        anyhow::bail!(
            "No research data available between {} and {} for scope {:?}",
            window_from, window_to, scope_arg
        );
    }

    // ---- SRD aggregation ----
    let mut srd_days: Vec<NaiveDate> = Vec::new();
    let mut srd_durations: Vec<i64> = Vec::new();
    let mut longest_streak: i64 = 0;
    let mut current_streak: i64 = 0;

    for (date, snapshot) in &snapshots {
        let state_conservative = snapshot
            .state
            .as_ref()
            .map(|s| {
                matches!(
                    s.state,
                    core_domain::StrategyState::NoTrade
                        | core_domain::StrategyState::DeRisk
                        | core_domain::StrategyState::LeftProbe
                )
            })
            .unwrap_or(false);
        let strong_buy_count = snapshot.strong_buy_count();
        let is_srd = state_conservative && strong_buy_count >= 5;

        if is_srd {
            srd_days.push(*date);
            srd_durations.push(snapshot.divergence_duration());
            current_streak += 1;
            longest_streak = longest_streak.max(current_streak);
        } else {
            current_streak = 0;
        }
    }

    // ---- Stretch aggregation ----
    let mut stretch_distribution: HashMap<String, usize> = HashMap::new();
    let mut stretch_extreme_days: Vec<NaiveDate> = Vec::new();
    let mut crowding_distribution: HashMap<String, usize> = HashMap::new();
    let mut momentum_distribution: HashMap<String, usize> = HashMap::new();
    let mut breadth_distribution: HashMap<String, usize> = HashMap::new();

    for (date, snapshot) in &snapshots {
        let (overall, _) = snapshot.stretch_overall();
        *stretch_distribution.entry(overall.to_string()).or_insert(0) += 1;
        match overall {
            "Extreme" => stretch_extreme_days.push(*date),
            _ => {}
        }

        let (crowding_level, _, _) = snapshot.stretch_crowding();
        *crowding_distribution.entry(crowding_level.to_string()).or_insert(0) += 1;

        let (momentum_level, _, _) = snapshot.stretch_momentum();
        *momentum_distribution.entry(momentum_level.to_string()).or_insert(0) += 1;

        let (breadth_level, _, _) = snapshot.stretch_breadth();
        *breadth_distribution.entry(breadth_level.to_string()).or_insert(0) += 1;
    }

    // ---- Analytics aggregation ----
    let mut analytics_sections: Vec<String> = Vec::new();
    let conditions = vec![
        ("srd-strong", 20),
        ("srd-strong", 60),
        ("stretch-extreme-crowding-momentum", 20),
        ("stretch-extreme-crowding-momentum", 60),
    ];

    for (condition, horizon) in conditions {
        let evidence = context.research_condition_evidence(
            condition,
            analysis_scope,
            horizon,
            window_from,
            window_to,
        )?;
        analytics_sections.push(format_analytics_md(condition, horizon, &evidence));
    }

    // ---- Evidence worth ADR review ----
    let mut review_points: Vec<String> = Vec::new();
    if longest_streak >= 5 {
        review_points.push(format!(
            "SRD streak reached {} days — worth reviewing in a future ADR if the pattern persists across the 90-day observation window.",
            longest_streak
        ));
    }
    if !stretch_extreme_days.is_empty() {
        review_points.push(format!(
            "Market Stretch = Extreme on {} days — worth tracking whether extremes correlate with subsequent drawdowns.",
            stretch_extreme_days.len()
        ));
    }
    if !srd_days.is_empty() && !stretch_extreme_days.is_empty() {
        let overlap: Vec<NaiveDate> = srd_days
            .iter()
            .filter(|d| stretch_extreme_days.contains(d))
            .copied()
            .collect();
        if !overlap.is_empty() {
            review_points.push(format!(
                "SRD and Stretch Extreme co-occurred on {} days — worth noting as a potential compound regime tension.",
                overlap.len()
            ));
        }
    }

    // ---- Build ResearchContext for pipeline ----
    let research_ctx = context.build_research_context_for_date(window_to, analysis_scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx,
    };

    // ---- Populate ReviewReportInput ----
    let srd_frequency_pct = if !snapshots.is_empty() {
        (srd_days.len() as f64 / snapshots.len() as f64) * 100.0
    } else {
        0.0
    };
    let avg_divergence_duration = if !srd_durations.is_empty() {
        srd_durations.iter().sum::<i64>() as f64 / srd_durations.len() as f64
    } else {
        0.0
    };

    let input = ReviewReportInput {
        window_from,
        window_to,
        observation_days: snapshots.len(),
        calendar_days: (window_to - window_from).num_days() + 1,
        srd_frequency_pct,
        avg_divergence_duration,
        longest_srd_streak: longest_streak,
        latest_srd_dates: srd_days,
        stretch_distribution,
        crowding_distribution,
        momentum_distribution,
        breadth_distribution,
        analytics_sections,
        review_points,
    };

    // ---- Build document via ReviewReportBuilder ----
    let builder = ReviewReportBuilder::new(ReviewProfile::Quarterly);
    let doc = builder.build(&reporting_snapshot, &input)?;

    // ---- Resolve output path ----
    let scope_label = match scope_arg {
        ReportScopeArg::Global => "GLOBAL",
        ReportScopeArg::Cn => "CN",
        ReportScopeArg::Hk => "HK",
    };
    let output_path = output.unwrap_or_else(|| {
        std::path::PathBuf::from(format!(
            "reports/research-quarterly-{}-{}.md",
            scope_label.to_lowercase(),
            window_to
        ))
    });

    // ---- Write Markdown report to file ----
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    {
        let mut md_formatter = MarkdownFormatter::new();
        report_renderer::render(&mut md_formatter, &doc);
        std::fs::write(&output_path, md_formatter.finalize())?;
    }

    // ---- Print text summary to stdout ----
    {
        let mut text_formatter = TextFormatter::new();
        report_renderer::render(&mut text_formatter, &doc);
        println!("{}", text_formatter.finalize());
    }

    Ok(())
}

/// Format analytics results as a Markdown subsection.
fn format_analytics_md(
    condition: &str,
    horizon: usize,
    evidence: &core_domain::research::attribution::Evidence,
) -> String {
    let mut s = String::new();
    s.push_str(&format!("### Condition: `{}` | Horizon: {} days\n\n", condition, horizon));
    s.push_str(&format!("- **Occurrences**: {}\n", evidence.occurrences));

    if evidence.forward_returns.is_empty() {
        s.push_str("- Not enough observations.\n");
        return s;
    }

    let mut sorted_returns = evidence.forward_returns.clone();
    sorted_returns.sort_by(|a, b| a.total_cmp(b));
    let mut sorted_dds = evidence.max_drawdowns.clone();
    sorted_dds.sort_by(|a, b| a.total_cmp(b));

    let count = evidence.forward_returns.len();
    let median = regime_audit::common::percentile(&sorted_returns, 0.50);
    let mean = evidence.forward_returns.iter().sum::<f64>() / count as f64;
    let worst = sorted_returns.first().copied().unwrap_or(0.0);
    let best = sorted_returns.last().copied().unwrap_or(0.0);
    let positive_ratio = evidence.forward_returns.iter().filter(|&&r| r > 0.0).count() as f64 / count as f64;
    let median_max_dd = regime_audit::common::percentile(&sorted_dds, 0.50);

    s.push_str(&format!("- **Forward return median**: {:+.1}%\n", median * 100.0));
    s.push_str(&format!("- **Forward return mean**: {:+.1}%\n", mean * 100.0));
    s.push_str(&format!("- **Forward return best**: {:+.1}%\n", best * 100.0));
    s.push_str(&format!("- **Forward return worst**: {:+.1}%\n", worst * 100.0));
    s.push_str(&format!("- **Positive ratio**: {:.1}%\n", positive_ratio * 100.0));
    s.push_str(&format!("- **Median max drawdown**: {:.1}%\n", median_max_dd * 100.0));
    s
}

/// Market Confirmation analysis — quantifies how well the current market trend
/// is confirmed across Trend, Participation, and Risk dimensions.
pub fn handle_research_confirmation(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
) -> Result<()> {
    let scope: core_domain::AnalysisScope = match scope_arg {
        ReportScopeArg::Global => core_domain::AnalysisScope::Global,
        ReportScopeArg::Cn => core_domain::AnalysisScope::Cn,
        ReportScopeArg::Hk => core_domain::AnalysisScope::Hk,
    };

    // Resolve target date
    let target_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "signal_snapshot")
            .unwrap_or(None)
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    // Build ResearchContext from dataset (no snapshot needed for confirmation-only)
    let research_ctx = context.build_research_context_for_date(target_date, scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx.clone(),
    };

    // Populate ConfirmationReportInput from ResearchContext
    let input = ConfirmationReportInput {
        trend_score: research_ctx.confirmation.trend.score,
        trend_label: research_ctx.confirmation.trend.label,
        participation_score: research_ctx.confirmation.participation.score,
        participation_label: research_ctx.confirmation.participation.label,
        risk_score: research_ctx.confirmation.risk.score,
        risk_label: research_ctx.confirmation.risk.label,
        overall: research_ctx.confirmation.overall,
    };

    // Build document via Reporting Pipeline
    let doc = ResearchReportBuilder::build_confirmation(&reporting_snapshot, &input)?;
    let mut formatter = TextFormatter::new();
    report_renderer::render(&mut formatter, &doc);
    println!("{}", formatter.finalize());

    Ok(())
}

/// Recovery Index analysis — measures how much the market has recovered from
/// drawdown or stress, scored 0-100 across multiple dimensions.
pub fn handle_research_recovery(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
) -> Result<()> {
    let scope: core_domain::AnalysisScope = match scope_arg {
        ReportScopeArg::Global => core_domain::AnalysisScope::Global,
        ReportScopeArg::Cn => core_domain::AnalysisScope::Cn,
        ReportScopeArg::Hk => core_domain::AnalysisScope::Hk,
    };

    // Resolve target date
    let target_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "signal_snapshot")
            .unwrap_or(None)
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    // Build ResearchContext from dataset
    let research_ctx = context.build_research_context_for_date(target_date, scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx.clone(),
    };

    // Classify recovery label from score
    let recovery_label = recovery_index_label(research_ctx.recovery.score);

    // Populate RecoveryReportInput from ResearchContext
    let input = RecoveryReportInput {
        index: research_ctx.recovery.score,
        label: recovery_label.to_string(),
        drivers: research_ctx.recovery.drivers,
    };

    // Build document via Reporting Pipeline
    let doc = ResearchReportBuilder::build_recovery(&reporting_snapshot, &input)?;
    let mut formatter = TextFormatter::new();
    report_renderer::render(&mut formatter, &doc);
    println!("{}", formatter.finalize());

    Ok(())
}

/// Map recovery index score (0-100) to a human-readable label.
fn recovery_index_label(score: f64) -> &'static str {
    match score {
        s if s >= 80.0 => "Strong Recovery",
        s if s >= 60.0 => "Recovering",
        s if s >= 40.0 => "Stabilizing",
        s if s >= 20.0 => "Under Pressure",
        _ => "Crisis",
    }
}

/// V7.2C Research Calibration — run the calibration framework over a historical window.
/// Generates a markdown report under `reports/calibration/`.
pub fn handle_research_calibration(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    horizon: usize,
    top_n: usize,
    lookback: usize,
) -> Result<()> {
    let scope: app_service::ReportScope = scope_arg.into();

    let output_path = context.run_research_calibration(
        scope, from, to, horizon, top_n, lookback,
    )?;

    println!("Research Calibration complete.");
    println!("Report written to: {}", output_path.display());

    Ok(())
}

/// V7.3 Research Consensus — synthesize Observation, Evolution, and Historical Evidence
/// into a research-language interpretation (Bias, Confidence, Supporting/Contradicting Evidence).
pub fn handle_research_consensus(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
    horizon: usize,
    top_n: usize,
    lookback: usize,
) -> Result<()> {
    let scope: app_service::ReportScope = scope_arg.into();

    let output_path = context.run_research_consensus(
        scope, date, horizon, top_n, lookback,
    )?;

    println!("Research Consensus complete.");
    println!("Report written to: {}", output_path.display());

    Ok(())
}

/// V7.2B Historical Analogue Search — find similar market conditions in history
/// and profile forward outcomes for the matched dates.
pub fn handle_research_analogues(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
    horizon: usize,
    top_n: usize,
    lookback: usize,
) -> Result<()> {
    let scope: app_service::ReportScope = scope_arg.into();

    // Run the search via AppContext orchestration
    let result = context.research_analogues(scope, date, horizon, top_n, lookback)?;

    // Render as plain text (MVP: direct output; no ReportBuilder)
    println!("Historical Evidence Search");
    println!("  Searched days:        {}", result.searched_days);
    println!("  Filtered days:        {}", result.filtered_days);
    println!("  Average distance:      {:.2}", result.average_distance);
    println!();

    println!("Top Matches");
    for (i, m) in result.matches.iter().enumerate() {
        let level_str = match m.level {
            market_fingerprint_engine::MatchLevel::VeryHigh => "Very High",
            market_fingerprint_engine::MatchLevel::High => "High",
            market_fingerprint_engine::MatchLevel::Moderate => "Moderate",
            market_fingerprint_engine::MatchLevel::Weak => "Weak",
        };
        println!("  #{:<3} {}  {}", i + 1, m.date, level_str);
    }
    println!();

    if let Some(outcome) = &result.outcome {
        println!("Forward {}D Outcome", outcome.horizon_days);
        println!("  Median:    {:+.1}%", outcome.median * 100.0);
        println!("  Mean:      {:+.1}%", outcome.mean * 100.0);
        println!("  Best:      {:+.1}%", outcome.best * 100.0);
        println!("  Worst:     {:+.1}%", outcome.worst * 100.0);
        println!("  Win Rate:  {:.0}%", outcome.win_rate * 100.0);
        println!(
            "  Median Max Drawdown: {:+.1}%",
            outcome.median_max_drawdown * 100.0
        );
    } else {
        println!("Forward Outcome: No valid forward data available for matched dates.");
    }

    Ok(())
}

/// V7 Workflow — Replay: run historical analytics across conditions and horizons,
/// saving each result as an Evidence Asset in the workspace, and emit the same
/// index files previously produced by the PowerShell pipeline.
pub fn handle_research_replay(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    output_dir: String,
) -> Result<()> {
    use std::fs;
    use std::path::PathBuf;

    let scope: app_service::ReportScope = scope_arg.into();

    // Resolve date window. Analytics themselves use the full available history;
    // from/to are recorded as replay-window metadata.
    let to = to.unwrap_or_else(|| Local::now().date_naive());
    let from = from.unwrap_or_else(|| to - chrono::Duration::days(90));

    // Default replay configuration: matches the PowerShell pipeline.
    let conditions = vec![
        "srd-strong".to_string(),
        "stretch-extreme-crowding-momentum".to_string(),
    ];
    let horizons = vec![20usize, 60usize];

    let summaries = context.research_replay(scope, from, to, &conditions, &horizons)?;

    // Build candidate list from evidence summaries using the same deterministic
    // placeholder rule as the old PowerShell pipeline.
    let candidates: Vec<serde_json::Value> = summaries
        .iter()
        .filter(|s| {
            s.occurrences >= 5 && (s.positive_ratio >= 0.75 || s.positive_ratio <= 0.25)
        })
        .map(|s| {
            serde_json::json!({
                "id": format!(
                    "CD-{}-{}-{}-H{}",
                    to,
                    s.scope.to_uppercase(),
                    s.condition.to_uppercase(),
                    s.horizon
                ),
                "condition": s.condition,
                "scope": s.scope,
                "horizon": s.horizon,
                "window": format!("{} ~ {}", from, to),
                "positive_ratio": s.positive_ratio,
                "occurrences": s.occurrences,
                "status": "candidate_evidence",
                "attribution_status": "pending",
                "evidence_id": s.id,
            })
        })
        .collect();

    let evidence_json: Vec<serde_json::Value> = summaries
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "condition": s.condition,
                "scope": s.scope,
                "horizon": s.horizon,
                "occurrences": s.occurrences,
                "positive_ratio": s.positive_ratio,
                "median_forward_return": s.median_forward_return,
                "generated_at": Utc::now().to_rfc3339(),
                "status": "candidate",
                "workspace_path": s.workspace_path,
            })
        })
        .collect();

    let scope_str = scope.as_str();
    let manifest = serde_json::json!({
        "generated_at": Utc::now().to_rfc3339(),
        "schema_version": "v1",
        "analytics_version": "v1",
        "from": from,
        "to": to,
        "scopes": [scope_str],
        "conditions": conditions,
        "horizons": horizons,
        "reports": [],
        "indices": ["manifest.json", "summary.json", "evidence-index.json", "candidate-index.json"],
    });

    let summary = serde_json::json!({
        "generated_at": Utc::now().to_rfc3339(),
        "schema_version": "v1",
        "from": from,
        "to": to,
        "scopes": [scope_str],
        "conditions_analyzed": conditions,
        "total_evidence": evidence_json.len(),
        "total_candidates": candidates.len(),
        "scope_summaries": [{
            "scope": scope_str,
            "from": from,
            "to": to,
            "conditions": conditions,
            "horizons": horizons,
            "evidence_count": evidence_json.len(),
        }],
        "top_candidates": candidates.iter().take(5).cloned().collect::<Vec<_>>(),
    });

    let evidence_index = serde_json::json!({
        "generated_at": Utc::now().to_rfc3339(),
        "schema_version": "v1",
        "evidence": evidence_json,
    });

    let candidate_index = serde_json::json!({
        "generated_at": Utc::now().to_rfc3339(),
        "schema_version": "v1",
        "candidates": candidates,
    });

    let out = PathBuf::from(output_dir);
    fs::create_dir_all(&out)?;

    fs::write(out.join("manifest.json"), serde_json::to_string_pretty(&manifest)?)?;
    fs::write(out.join("summary.json"), serde_json::to_string_pretty(&summary)?)?;
    fs::write(out.join("evidence-index.json"), serde_json::to_string_pretty(&evidence_index)?)?;
    fs::write(out.join("candidate-index.json"), serde_json::to_string_pretty(&candidate_index)?)?;

    println!("Historical Replay complete.");
    println!("Scope: {}", scope_str);
    println!("Window: {} ~ {}", from, to);
    println!("Evidence entries: {}", evidence_json.len());
    println!("Candidate entries: {}", candidates.len());
    println!("Output directory: {}", out.display());

    Ok(())
}


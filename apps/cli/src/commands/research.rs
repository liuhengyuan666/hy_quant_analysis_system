use anyhow::Result;
use app_service::{AppContext, ReportScope, ResearchSnapshot};
use chrono::{Local, NaiveDate, Utc};
use core_domain::AnalysisScope;
use core_domain::research::classification::classify_level;
use core_domain::research::percentile::percentile_rank;
use report_builder::{
    AnalyticsReportInput, ReviewReportInput, ReviewProfile, ReviewReportBuilder,
    ResearchReportBuilder, SrdReportInput, StretchReportInput,
};
use report_renderer::{self, MarkdownFormatter, TextFormatter};
use reporting::{Formatter, ReportingSnapshot};
use std::collections::{BTreeMap, BTreeSet};
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

    // 6. Populate SrdReportInput
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

    // 7. Build document via Reporting Pipeline
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

/// Market Stretch analysis — pure observation tool
pub fn handle_research_stretch(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
) -> Result<()> {
    use chrono::Duration;

    let scope: core_domain::AnalysisScope = match scope_arg {
        ReportScopeArg::Global => core_domain::AnalysisScope::Global,
        ReportScopeArg::Cn => core_domain::AnalysisScope::Cn,
        ReportScopeArg::Hk => core_domain::AnalysisScope::Hk,
    };

    // 1. Resolve target date
    let report_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "rotation_rank")?
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    // 2. Build ResearchContext + ResearchSnapshot from a single dataset fetch
    let (research_ctx, snapshot) = context.build_research_bundle_for_date(report_date, scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx,
    };
    if snapshot.rotations.is_empty() {
        anyhow::bail!(
            "No rotation data available for date {} and scope {:?}",
            report_date, scope_arg
        );
    }

    // 4. Compute Stretch levels from snapshot
    let (crowding_level, concentration_pct, _) = snapshot.stretch_crowding();
    let (momentum_level, rs120_max, top5_rs120_avg) = snapshot.stretch_momentum();
    let (breadth_level, breadth_pct, breadth_sma5) = snapshot.stretch_breadth();
    let leverage_level = snapshot.stretch_leverage();
    let (overall, _weighted_score) = snapshot.stretch_overall();

    // 5. Historical crowding percentile (120-day lookback)
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

    // 6. Evidence and narrative
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

    // 7. Populate StretchReportInput
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

    // 8. Build document via Reporting Pipeline
    let doc = ResearchReportBuilder::build_stretch(&reporting_snapshot, &input)?;
    let mut formatter = TextFormatter::new();
    report_renderer::render(&mut formatter, &doc);
    println!("{}", formatter.finalize());

    Ok(())
}

/// Conditional forward-return analytics — answers "what happened after this condition historically".
/// MVP: only supports `--horizon 20|60` and two hard-coded reproducible conditions.
pub fn handle_research_analytics(
    context: &AppContext,
    condition: String,
    horizon: usize,
    scope_arg: ReportScopeArg,
) -> Result<()> {
    if horizon != 20 && horizon != 60 {
        anyhow::bail!(
            "Analytics MVP only supports --horizon 20 or 60 (got {})",
            horizon
        );
    }

    let scope: AnalysisScope = match scope_arg {
        ReportScopeArg::Global => AnalysisScope::Global,
        ReportScopeArg::Cn => AnalysisScope::Cn,
        ReportScopeArg::Hk => AnalysisScope::Hk,
    };

    let anchor_symbol = match scope_arg {
        ReportScopeArg::Global | ReportScopeArg::Cn => "000300",
        ReportScopeArg::Hk => "HSCEI",
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

    let matched_dates = match condition.as_str() {
        "srd-strong" => match_srd_strong(context, scope, earliest_date, target_date)?,
        "stretch-extreme-crowding-momentum" => {
            match_stretch_extreme(context, scope_arg, scope, earliest_date, target_date)?
        }
        _ => anyhow::bail!(
            "Unknown condition '{}'. MVP supports: srd-strong, stretch-extreme-crowding-momentum",
            condition
        ),
    };

    let mut returns: Vec<f64> = Vec::new();
    let mut max_drawdowns: Vec<f64> = Vec::new();

    for date in &matched_dates {
        let Some(current_close) = close_by_date.get(date) else { continue };
        if *current_close <= 0.0 {
            continue;
        }

        let start = date.succ_opt().unwrap_or(*date);
        let forward_entries: Vec<(NaiveDate, f64)> = close_by_date
            .range(start..)
            .take(horizon)
            .map(|(d, c)| (*d, *c))
            .collect();

        if forward_entries.len() < horizon {
            continue;
        }

        let forward_close = forward_entries.last().unwrap().1;
        let ret = (forward_close - current_close) / current_close;
        returns.push(ret);

        let forward_closes: Vec<f64> = forward_entries.iter().map(|(_, c)| *c).collect();
        let dd = regime_audit::common::calculate_max_drawdown(*current_close, &forward_closes);
        max_drawdowns.push(dd);
    }

    // Compute analytics statistics
    let occurrences = returns.len();
    let (forward_return_median, forward_return_mean, forward_return_best, forward_return_worst, positive_ratio, median_max_drawdown) =
        if occurrences > 0 {
            let mut sorted_returns = returns.clone();
            sorted_returns.sort_by(|a, b| a.total_cmp(b));
            let mut sorted_dds = max_drawdowns.clone();
            sorted_dds.sort_by(|a, b| a.total_cmp(b));

            let median = regime_audit::common::percentile(&sorted_returns, 0.50);
            let mean = returns.iter().sum::<f64>() / occurrences as f64;
            let worst = sorted_returns.first().copied().unwrap_or(0.0);
            let best = sorted_returns.last().copied().unwrap_or(0.0);
            let p_ratio = returns.iter().filter(|&&r| r > 0.0).count() as f64 / occurrences as f64;
            let median_dd = regime_audit::common::percentile(&sorted_dds, 0.50);
            (median, mean, best, worst, p_ratio, median_dd)
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        };

    // Build ResearchContext for the pipeline
    let research_ctx = context.build_research_context_for_date(target_date, scope)?;
    let reporting_snapshot = ReportingSnapshot {
        generated_at: Utc::now(),
        research: research_ctx,
    };

    // Populate AnalyticsReportInput
    let input = AnalyticsReportInput {
        condition: condition.clone(),
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

    // Build document via Reporting Pipeline
    let doc = ResearchReportBuilder::build_analytics(&reporting_snapshot, &input)?;
    let mut formatter = TextFormatter::new();
    report_renderer::render(&mut formatter, &doc);
    println!("{}", formatter.finalize());

    Ok(())
}

/// Match dates where StrongBuy >= 5 and StrategyState is conservative.
fn match_srd_strong(
    context: &AppContext,
    scope: AnalysisScope,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    let states = market_store::fetch_strategy_states_for_scope(&context.storage, scope)?;
    let conservative_dates: BTreeSet<NaiveDate> = states
        .iter()
        .filter(|s| {
            s.date >= from
                && s.date <= to
                && matches!(
                    s.state,
                    core_domain::StrategyState::NoTrade
                        | core_domain::StrategyState::DeRisk
                        | core_domain::StrategyState::LeftProbe
                )
        })
        .map(|s| s.date)
        .collect();

    let signal_snapshots = market_store::fetch_signal_snapshots_for_range_with_scope(
        &context.storage, scope, from, to,
    )?;

    let mut strong_buy_by_date: BTreeMap<NaiveDate, usize> = BTreeMap::new();
    for s in &signal_snapshots {
        if matches!(s.signal_label, core_domain::SignalLabel::StrongBuy) {
            *strong_buy_by_date.entry(s.date).or_insert(0) += 1;
        }
    }

    let mut matched = Vec::new();
    for date in conservative_dates {
        let count = strong_buy_by_date.get(&date).copied().unwrap_or(0);
        if count >= 5 {
            matched.push(date);
        }
    }
    matched.sort();
    Ok(matched)
}

/// Match dates where Stretch Overall=Extreme, Crowding=Extreme, Momentum=Extreme, Breadth=Normal.
fn match_stretch_extreme(
    context: &AppContext,
    scope_arg: ReportScopeArg,
    scope: AnalysisScope,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<NaiveDate>> {
    let rotations = market_store::fetch_rotation_ranks_for_range(&context.storage, from, to)?;

    let instruments = context.seed_universe().unwrap_or_default();
    let symbol_in_scope = |symbol: &str| match scope_arg {
        ReportScopeArg::Global => true,
        ReportScopeArg::Cn => instruments
            .iter()
            .any(|i| i.symbol == symbol && i.market == core_domain::Market::Cn),
        ReportScopeArg::Hk => instruments
            .iter()
            .any(|i| i.symbol == symbol && i.market == core_domain::Market::Hk),
    };

    let mut rotation_by_date: BTreeMap<NaiveDate, Vec<(f64, f64)>> = BTreeMap::new();
    for r in rotations {
        if !symbol_in_scope(&r.symbol) {
            continue;
        }
        rotation_by_date
            .entry(r.date)
            .or_default()
            .push((r.momentum_score, r.rs_120));
    }

    let env_snapshots = market_store::fetch_environment_snapshots_for_scope(
        &context.storage,
        scope,
        from,
        to,
    )?;
    let breadth_by_date: BTreeMap<NaiveDate, f64> = env_snapshots
        .iter()
        .map(|e| (e.date, e.breadth_pct))
        .collect();

    let mut matched = Vec::new();
    for (date, rows) in &rotation_by_date {
        if rows.is_empty() {
            continue;
        }
        let total_momentum: f64 = rows.iter().map(|(m, _)| m).sum();
        let mut sorted = rows.clone();
        sorted.sort_by(|a, b| b.0.total_cmp(&a.0));
        let top5_sum: f64 = sorted.iter().take(5).map(|(m, _)| m).sum();
        let concentration_pct = if total_momentum > 0.0 {
            (top5_sum / total_momentum) * 100.0
        } else {
            0.0
        };
        let rs120_max = rows.iter().map(|(_, rs)| *rs).fold(f64::NEG_INFINITY, f64::max);

        let crowding_level = classify_level(concentration_pct, 30.0, 50.0, true);
        let momentum_level = classify_level(rs120_max, 70.0, 85.0, true);

        let breadth_pct = breadth_by_date.get(date).copied().unwrap_or(0.0);
        let breadth_level = classify_level(breadth_pct, 35.0, 20.0, false);

        if crowding_level == "Extreme" && momentum_level == "Extreme" && breadth_level == "Normal" {
            matched.push(*date);
        }
    }
    Ok(matched)
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
        let matched_dates = match condition {
            "srd-strong" => match_srd_strong(context, analysis_scope, window_from, window_to)?,
            "stretch-extreme-crowding-momentum" => {
                match_stretch_extreme(context, scope_arg, analysis_scope, window_from, window_to)?
            }
            _ => Vec::new(),
        };

        let anchor_symbol = match scope_arg {
            ReportScopeArg::Global | ReportScopeArg::Cn => "000300",
            ReportScopeArg::Hk => "HSCEI",
        };
        let returns = collect_forward_returns(context, anchor_symbol, &matched_dates, horizon)?;
        let max_drawdowns = collect_max_drawdowns(context, anchor_symbol, &matched_dates, horizon)?;
        analytics_sections.push(format_analytics_md(
            condition,
            horizon,
            &returns,
            &max_drawdowns,
        ));
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

/// Collect forward returns for a list of matched dates.
fn collect_forward_returns(
    context: &AppContext,
    anchor_symbol: &str,
    dates: &[NaiveDate],
    horizon: usize,
) -> Result<Vec<f64>> {
    let bars = market_store::fetch_daily_bars(&context.storage, anchor_symbol)?;
    let close_by_date: BTreeMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let mut returns = Vec::new();
    for d in dates {
        let Some(start) = close_by_date.get(d) else { continue };
        // Find horizon-th future trading day
        let mut future_dates: Vec<NaiveDate> = close_by_date.keys().copied().filter(|k| k > d).collect();
        future_dates.sort();
        let Some(end_date) = future_dates.get(horizon.saturating_sub(1)) else { continue };
        let Some(end) = close_by_date.get(end_date) else { continue };
        returns.push((end - start) / start);
    }
    Ok(returns)
}

/// Collect maximum drawdowns for a list of matched dates over a horizon.
fn collect_max_drawdowns(
    context: &AppContext,
    anchor_symbol: &str,
    dates: &[NaiveDate],
    horizon: usize,
) -> Result<Vec<f64>> {
    let bars = market_store::fetch_daily_bars(&context.storage, anchor_symbol)?;
    let close_by_date: BTreeMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let mut drawdowns = Vec::new();
    for d in dates {
        let mut future_dates: Vec<NaiveDate> = close_by_date.keys().copied().filter(|k| k > d).collect();
        future_dates.sort();
        let window = future_dates.into_iter().take(horizon).collect::<Vec<_>>();
        if window.is_empty() {
            continue;
        }
        let start_price = close_by_date.get(d).copied().unwrap_or(0.0);
        if start_price <= 0.0 {
            continue;
        }
        let prices: Vec<f64> = window
            .iter()
            .filter_map(|date| close_by_date.get(date).copied())
            .collect();
        if prices.is_empty() {
            continue;
        }
        let dd = regime_audit::common::calculate_max_drawdown(start_price, &prices);
        drawdowns.push(dd);
    }
    Ok(drawdowns)
}

/// Format analytics results as a Markdown subsection.
fn format_analytics_md(
    condition: &str,
    horizon: usize,
    returns: &[f64],
    max_drawdowns: &[f64],
) -> String {
    let mut s = String::new();
    s.push_str(&format!("### Condition: `{}` | Horizon: {} days\n\n", condition, horizon));
    s.push_str(&format!("- **Occurrences**: {}\n", returns.len()));

    if returns.is_empty() {
        s.push_str("- Not enough observations.\n");
        return s;
    }

    let mut sorted_returns = returns.to_vec();
    sorted_returns.sort_by(|a, b| a.total_cmp(b));
    let mut sorted_dds = max_drawdowns.to_vec();
    sorted_dds.sort_by(|a, b| a.total_cmp(b));

    let count = returns.len();
    let median = regime_audit::common::percentile(&sorted_returns, 0.50);
    let mean = returns.iter().sum::<f64>() / count as f64;
    let worst = sorted_returns.first().copied().unwrap_or(0.0);
    let best = sorted_returns.last().copied().unwrap_or(0.0);
    let positive_ratio = returns.iter().filter(|&&r| r > 0.0).count() as f64 / count as f64;
    let median_max_dd = regime_audit::common::percentile(&sorted_dds, 0.50);

    s.push_str(&format!("- **Forward return median**: {:+.1}%\n", median * 100.0));
    s.push_str(&format!("- **Forward return mean**: {:+.1}%\n", mean * 100.0));
    s.push_str(&format!("- **Forward return best**: {:+.1}%\n", best * 100.0));
    s.push_str(&format!("- **Forward return worst**: {:+.1}%\n", worst * 100.0));
    s.push_str(&format!("- **Positive ratio**: {:.1}%\n", positive_ratio * 100.0));
    s.push_str(&format!("- **Median max drawdown**: {:.1}%\n", median_max_dd * 100.0));
    s
}


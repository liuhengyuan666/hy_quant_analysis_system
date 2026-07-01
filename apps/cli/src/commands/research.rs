use anyhow::Result;
use app_service::{AppContext, ReportScope};
use chrono::{Local, NaiveDate};
use core_domain::{AnalysisScope, RotationRankSnapshot};
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
    use core_domain::SignalLabel;

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

    // 2. Fetch latest signal snapshots via typed API for count/avg
    let latest_signals = market_store::fetch_signal_snapshots_for_date_with_scope(
        &context.storage,
        target_date,
        scope,
    )?;

    // 3. Fetch strategy states for scope (for streak/divergence analysis)
    let states = market_store::fetch_strategy_states_for_scope(&context.storage, scope)?;

    // 4. Bulk-fetch signal history (raw query) for duration + percentile
    let lookback = target_date - chrono::Duration::days(365);
    let signal_query = format!(
        "SELECT date, final_score, signal_label FROM quant.signal_snapshot \
         WHERE date BETWEEN '{}' AND '{}' AND analysis_scope = '{}' \
         ORDER BY date FORMAT JSONEachRow",
        lookback,
        target_date,
        scope.as_str()
    );
    let signal_body = market_store::fetch_clickhouse_text(&context.storage, &signal_query)?;

    // Parse into a date-keyed map
    use std::collections::BTreeMap;
    let mut daily_signals: BTreeMap<NaiveDate, Vec<(f64, String)>> = BTreeMap::new();
    for line in signal_body.lines().filter(|l| !l.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line)?;
        let date_str = match row["date"].as_str() {
            Some(d) => d,
            None => continue,
        };
        let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else {
            continue;
        };
        let score = row["final_score"].as_f64().unwrap_or(0.0);
        let label = row["signal_label"].as_str().unwrap_or("").to_string();
        daily_signals.entry(date).or_default().push((score, label));
    }

    // 5. SRD duration: consecutive days with conservative state + StrongBuy/Buy signals
    let is_conservative = |state: &core_domain::StrategyState| -> bool {
        matches!(
            state,
            core_domain::StrategyState::NoTrade
                | core_domain::StrategyState::DeRisk
                | core_domain::StrategyState::LeftProbe
        )
    };

    let mut recent_states: Vec<&core_domain::StrategyStateSnapshot> =
        states.iter().filter(|s| s.date <= target_date).collect();
    recent_states.sort_by(|a, b| b.date.cmp(&a.date));

    let mut duration: i64 = 0;
    for state_snapshot in &recent_states {
        if !is_conservative(&state_snapshot.state) {
            break;
        }
        let has_divergent = daily_signals
            .get(&state_snapshot.date)
            .map(|signals| signals.iter().any(|(_, label)| label == "StrongBuy" || label == "Buy"))
            .unwrap_or(false);
        if has_divergent {
            duration += 1;
        } else {
            break;
        }
    }

    // 6. Signal counts for latest date
    let strong_buy_count = latest_signals
        .iter()
        .filter(|s| matches!(s.signal_label, SignalLabel::StrongBuy))
        .count();
    let buy_count = latest_signals
        .iter()
        .filter(|s| matches!(s.signal_label, SignalLabel::Buy))
        .count();
    let avg_signal = if !latest_signals.is_empty() {
        latest_signals.iter().map(|s| s.final_score).sum::<f64>() / latest_signals.len() as f64
    } else {
        0.0
    };

    // 7. Breadth trend from environment
    let env_lookback = target_date - chrono::Duration::days(60);
    let env_snapshots = market_store::fetch_environment_snapshots_for_scope(
        &context.storage,
        scope,
        env_lookback,
        target_date,
    )?;
    let latest_env = env_snapshots.iter().max_by_key(|e| e.date);
    let breadth_trend = match latest_env {
        Some(env) => {
            let delta = env.breadth_5d_delta.unwrap_or(0.0);
            if delta > 0.05 {
                "Improving"
            } else if delta < -0.05 {
                "Weakening"
            } else {
                "Neutral"
            }
        }
        None => "Neutral",
    };

    // 8. Rotation pattern based on top momentum scores
    let rotations = market_store::fetch_rotation_ranks_for_date(&context.storage, target_date)?;
    let mut sorted_rotations = rotations.clone();
    sorted_rotations.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));
    let top_count = sorted_rotations.len().min(10);
    let top_10_avg_momentum: f64 = if top_count > 0 {
        sorted_rotations.iter().take(top_count).map(|r| r.momentum_score).sum::<f64>()
            / top_count as f64
    } else {
        0.0
    };
    let rotation_pattern = if top_10_avg_momentum > 1.5 {
        "Technology Dominant"
    } else if top_10_avg_momentum < 0.3 {
        "Defensive"
    } else {
        "Mixed"
    };

    // 9. Historical percentile of today's average signal
    let mut all_avg_signals: Vec<f64> = Vec::new();
    for (_, signals) in &daily_signals {
        if !signals.is_empty() {
            let avg = signals.iter().map(|(s, _)| s).sum::<f64>() / signals.len() as f64;
            all_avg_signals.push(avg);
        }
    }
    all_avg_signals.sort_by(|a, b| a.total_cmp(b));
    let historical_percentile = if all_avg_signals.is_empty() {
        50.0
    } else {
        let below = all_avg_signals.iter().filter(|&&v| v < avg_signal).count();
        (below as f64 / all_avg_signals.len() as f64) * 100.0
    };

    // 10. Narrative helpers
    let state_label = recent_states
        .first()
        .map(|s| format!("{:?}", s.state))
        .unwrap_or_else(|| "NO_TRADE".to_string());
    let interpretation = srd_interpretation(
        strong_buy_count,
        buy_count,
        breadth_trend,
        rotation_pattern,
        &state_label,
    );
    let confidence = srd_confidence(strong_buy_count, buy_count, breadth_trend, duration);
    let percentile_text = percentile_label(historical_percentile);

    // 11. Output clean text table
    println!(
        "SRD Statistics | Date: {} | Scope: {}",
        target_date,
        scope.as_str()
    );
    println!("{:=<80}", "");
    println!("  StrongBuy count:       {}", strong_buy_count);
    println!("  Buy count:             {}", buy_count);
    println!("  Average Signal:        {:.1}", avg_signal);
    println!(
        "  Duration:              {} days (consecutive trading days with divergence)",
        duration
    );
    println!("  Breadth trend:         {}", breadth_trend);
    println!("  Rotation pattern:      {}", rotation_pattern);
    println!(
        "  Historical percentile: {:.0}% ({})",
        historical_percentile, percentile_text
    );
    println!("{:-<80}", "");
    println!("  Interpretation:        {}", interpretation);
    println!("  Confidence:            {}", confidence);
    println!("{:=<80}", "");
    println!("Observation tool — does not influence any decision logic");

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

/// Classify a metric into Normal / Elevated / Extreme
fn classify_level(value: f64, elevated_threshold: f64, extreme_threshold: f64, higher_is_more_extreme: bool) -> &'static str {
    if higher_is_more_extreme {
        if value >= extreme_threshold { "Extreme" }
        else if value >= elevated_threshold { "Elevated" }
        else { "Normal" }
    } else {
        if value <= extreme_threshold { "Extreme" }
        else if value <= elevated_threshold { "Elevated" }
        else { "Normal" }
    }
}

/// Compute the percentile of a value within a sorted slice (0.0 = lowest, 100.0 = highest)
fn percentile_rank(value: f64, sorted: &[f64]) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let count = sorted.len();
    let below = sorted.iter().filter(|&&v| v <= value).count();
    (below as f64 / count as f64) * 100.0
}

/// Textual label for a percentile rank.
fn percentile_label(p: f64) -> &'static str {
    if p < 20.0 {
        "Very Low"
    } else if p < 40.0 {
        "Low"
    } else if p < 60.0 {
        "Moderate"
    } else if p < 80.0 {
        "High"
    } else {
        "Very High"
    }
}

/// Convert a Normal/Elevated/Extreme level into a numeric score for weighted composition.
fn level_to_score(level: &str) -> f64 {
    match level {
        "Extreme" => 2.0,
        "Elevated" => 1.0,
        _ => 0.0,
    }
}

/// Weighted Stretch Overall: Momentum 40%, Crowding 30%, Breadth 20%, Leverage 10%.
/// Returns the classified level and the raw weighted score.
fn weighted_stretch_overall(
    crowding: &str,
    breadth: &str,
    momentum: &str,
    leverage: &str,
) -> (&'static str, f64) {
    let score = level_to_score(momentum) * 0.40
        + level_to_score(crowding) * 0.30
        + level_to_score(breadth) * 0.20
        + level_to_score(leverage) * 0.10;
    let level = if score >= 1.2 {
        "Extreme"
    } else if score >= 0.5 {
        "Elevated"
    } else {
        "Normal"
    };
    (level, score)
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
    scope: ReportScopeArg,
    date: Option<NaiveDate>,
) -> Result<()> {
    use chrono::Duration;

    // 1. Resolve target date
    let report_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "rotation_rank")?
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    // 2. Fetch current rotation data
    let mut rows = market_store::fetch_rotation_ranks_for_date(&context.storage, report_date)?;

    // 3. Scope filter (mirrors handle_rotation_ranking pattern)
    if !matches!(scope, ReportScopeArg::Global) {
        let instruments = context.seed_universe().unwrap_or_default();
        rows = rows.into_iter().filter(|row| {
            instruments.iter().any(|inst| {
                if inst.symbol != row.symbol { return false; }
                match (scope, &inst.market) {
                    (ReportScopeArg::Cn, core_domain::Market::Cn) => true,
                    (ReportScopeArg::Hk, core_domain::Market::Hk) => true,
                    _ => false,
                }
            })
        }).collect::<Vec<_>>();
    }

    if rows.is_empty() {
        anyhow::bail!(
            "No rotation data available for date {} and scope {:?}",
            report_date, scope
        );
    }

    // 4. Sort by momentum descending
    rows.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));

    let total_momentum: f64 = rows.iter().map(|r| r.momentum_score).sum();
    let top5_slice: Vec<&RotationRankSnapshot> = rows.iter().take(5).collect();

    // ====== CROWDING ======
    let concentration_pct = if total_momentum > 0.0 {
        let top5_momentum: f64 = top5_slice.iter().map(|r| r.momentum_score).sum();
        (top5_momentum / total_momentum) * 100.0
    } else {
        0.0
    };

    // Historical percentile: fetch recent ~60 trading days of concentration values
    let start_date = report_date - Duration::days(120);
    let hist_query = format!(
        "SELECT date,symbol,momentum_score FROM quant.rotation_rank WHERE date >= '{}' ORDER BY date FORMAT JSONEachRow",
        start_date
    );
    let hist_body = market_store::fetch_clickhouse_text(&context.storage, &hist_query)?;
    let hist_all: Vec<serde_json::Value> = market_store::parse_json_each_row(&hist_body, "rotation rank row")?;

    // Group momentum scores by date
    use std::collections::BTreeMap;
    let mut date_map: BTreeMap<NaiveDate, Vec<f64>> = BTreeMap::new();
    for row in &hist_all {
        if let (Some(date_str), Some(score)) = (
            row.get("date").and_then(|v| v.as_str()),
            row.get("momentum_score").and_then(|v| v.as_f64()),
        ) {
            if let Ok(d) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                date_map.entry(d).or_default().push(score);
            }
        }
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

    let crowding_level = classify_level(concentration_pct, 30.0, 50.0, true);

    // ====== MOMENTUM ======
    let rs120_max = rows.iter().map(|r| r.rs_120).fold(f64::NEG_INFINITY, f64::max);
    let top5_rs120_avg = if !top5_slice.is_empty() {
        top5_slice.iter().map(|r| r.rs_120).sum::<f64>() / top5_slice.len() as f64
    } else {
        0.0
    };
    let momentum_level = classify_level(rs120_max, 70.0, 85.0, true);

    // ====== BREADTH ======
    let analysis_scope = match scope {
        ReportScopeArg::Global => AnalysisScope::Global,
        ReportScopeArg::Cn => AnalysisScope::Cn,
        ReportScopeArg::Hk => AnalysisScope::Hk,
    };
    let env = market_store::fetch_latest_environment_on_or_before(
        &context.storage, report_date, analysis_scope,
    )?;

    let (breadth_pct, breadth_sma5, breadth_level) = if let Some(ref e) = env {
        let bp = e.breadth_pct; // already stored as 0-100 percentage
        let sma5 = e.breadth_pct_sma5;
        let level = classify_level(bp, 35.0, 20.0, false);
        (bp, sma5, level)
    } else {
        (0.0, None, "Normal")
    };

    // ====== LEVERAGE ======
    let leverage_level = "Normal";

    // ====== OVERALL ======
    let (overall, _weighted_score) = weighted_stretch_overall(
        crowding_level,
        breadth_level,
        momentum_level,
        leverage_level,
    );
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

    // ====== NARRATIVE ======
    let stretch_interp = stretch_interpretation(overall, crowding_level, breadth_level, momentum_level);
    let risk_level = stretch_risk_level(overall, breadth_level);

    // ====== OUTPUT ======
    let scope_label = match scope {
        ReportScopeArg::Global => "Global",
        ReportScopeArg::Cn => "CN",
        ReportScopeArg::Hk => "HK",
    };

    println!("Market Stretch");
    println!("  Scope:                 {}", scope_label);
    println!("  Date:                  {}", report_date);
    println!();
    println!("  Overall:               {}", overall);
    println!("    Evidence:            {}", overall_evidence);
    println!();
    println!("  Interpretation:        {}", stretch_interp);
    println!("  Risk Level:            {}", risk_level);
    println!();
    println!("  Crowding:              {}", crowding_level);
    println!("    Evidence:");
    println!("      Top5 Rotation = {:.1}%", concentration_pct);
    if crowding_percentile.is_finite() {
        println!("      Historical Percentile = {:.0}%", crowding_percentile);
    } else {
        println!("      Historical Percentile = N/A (insufficient history)");
    }
    println!();
    println!("  Breadth:               {}", breadth_level);
    println!("    Evidence:");
    println!("      Breadth = {:.1}%", breadth_pct);
    match breadth_sma5 {
        Some(sma5) => println!("      SMA5 = {:.1}%", sma5),
        None => println!("      SMA5 = N/A"),
    }
    println!();
    println!("  Momentum:              {}", momentum_level);
    println!("    Evidence:");
    println!("      RS120 Max = {:.1}", rs120_max);
    println!("      RS120 Top5 Avg = {:.1}", top5_rs120_avg);
    println!();
    println!("  Leverage:              {}", leverage_level);
    println!("    Evidence:");
    println!("      (not yet available \u{2014} margin data source pending)");
    println!();
    println!("  Observation tool \u{2014} does not influence any decision logic");
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

    print_analytics_report(
        &condition,
        scope.as_str(),
        horizon,
        earliest_date,
        target_date,
        &returns,
        &max_drawdowns,
    );
    Ok(())
}

const ANALYTICS_VERSION: &str = "v1";

fn print_analytics_report(
    condition: &str,
    scope_label: &str,
    horizon: usize,
    window_from: NaiveDate,
    window_to: NaiveDate,
    returns: &[f64],
    max_drawdowns: &[f64],
) {
    println!(
        "Conditional Forward Return Analytics | Condition: {} | Scope: {}",
        condition, scope_label
    );
    println!("{:=<80}", "");
    println!("  Analytics version:        {}", ANALYTICS_VERSION);
    println!("  History window:           {} ~ {}", window_from, window_to);
    println!("  Occurrences:              {}", returns.len());
    println!("  Horizon:                  {} trading days", horizon);

    if returns.is_empty() {
        println!("  Note:                     Not enough observations. Need more samples.");
        println!("{:=<80}", "");
        println!("Observation tool — does not influence any decision logic");
        return;
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

    println!("  Forward return median:    {:+.1}%", median * 100.0);
    println!("  Forward return mean:      {:+.1}%", mean * 100.0);
    println!("  Forward return best:      {:+.1}%", best * 100.0);
    println!("  Forward return worst:     {:+.1}%", worst * 100.0);
    println!("  Positive ratio:           {:.1}%", positive_ratio * 100.0);
    println!("  Median max drawdown:      {:.1}%", median_max_dd * 100.0);
    println!("{:=<80}", "");
    println!("Observation tool — does not influence any decision logic");
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

    let query = format!(
        "SELECT date, signal_label FROM quant.signal_snapshot \
         WHERE date BETWEEN '{}' AND '{}' AND analysis_scope = '{}' \
         ORDER BY date FORMAT JSONEachRow",
        from,
        to,
        scope.as_str()
    );
    let body = market_store::fetch_clickhouse_text(&context.storage, &query)?;

    let mut strong_buy_by_date: BTreeMap<NaiveDate, usize> = BTreeMap::new();
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line)?;
        let Some(date_str) = row["date"].as_str() else { continue };
        let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else { continue };
        let label = row["signal_label"].as_str().unwrap_or("");
        if label == "StrongBuy" {
            *strong_buy_by_date.entry(date).or_insert(0) += 1;
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
    let query = format!(
        "SELECT date, symbol, momentum_score, rs_120 FROM quant.rotation_rank \
         WHERE date BETWEEN '{}' AND '{}' ORDER BY date FORMAT JSONEachRow",
        from, to
    );
    let body = market_store::fetch_clickhouse_text(&context.storage, &query)?;

    #[derive(Debug, serde::Deserialize)]
    struct RotationRow {
        date: String,
        symbol: String,
        momentum_score: f64,
        rs_120: f64,
    }

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
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let row: RotationRow = serde_json::from_str(line)?;
        let Ok(date) = NaiveDate::parse_from_str(&row.date, "%Y-%m-%d") else { continue };
        if !symbol_in_scope(&row.symbol) {
            continue;
        }
        rotation_by_date
            .entry(date)
            .or_default()
            .push((row.momentum_score, row.rs_120));
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


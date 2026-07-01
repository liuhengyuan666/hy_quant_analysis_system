use anyhow::Result;
use app_service::{AppContext, ReportScope};
use chrono::{Local, NaiveDate};
use core_domain::{AnalysisScope, RotationRankSnapshot};
use std::collections::{BTreeMap, BTreeSet};
use crate::ReportScopeArg;

/// Internal model: all research-relevant data for a single (date, scope).
/// SRD, Stretch, and future Quarterly Review all build on this snapshot.
/// This is an internal abstraction; it is not exposed in any public API or report DTO.
pub struct ResearchSnapshot {
    pub date: NaiveDate,
    pub signals: Vec<core_domain::SignalSnapshot>,
    pub state: Option<core_domain::StrategyStateSnapshot>,
    pub states_history: Vec<core_domain::StrategyStateSnapshot>,
    pub rotations: Vec<RotationRankSnapshot>,
    pub env: Option<core_domain::EnvironmentSnapshot>,
    pub signal_history: BTreeMap<NaiveDate, Vec<(f64, String)>>,
}

impl ResearchSnapshot {
    pub fn strong_buy_count(&self) -> usize {
        use core_domain::SignalLabel;
        self.signals
            .iter()
            .filter(|s| matches!(s.signal_label, SignalLabel::StrongBuy))
            .count()
    }

    pub fn buy_count(&self) -> usize {
        use core_domain::SignalLabel;
        self.signals
            .iter()
            .filter(|s| matches!(s.signal_label, SignalLabel::Buy))
            .count()
    }

    pub fn average_signal(&self) -> f64 {
        if self.signals.is_empty() {
            0.0
        } else {
            self.signals.iter().map(|s| s.final_score).sum::<f64>() / self.signals.len() as f64
        }
    }

    pub fn state_label(&self) -> String {
        self.state
            .as_ref()
            .map(|s| format!("{:?}", s.state))
            .unwrap_or_else(|| "NO_TRADE".to_string())
    }

    pub fn divergence_duration(&self) -> i64 {
        let is_conservative = |state: &core_domain::StrategyState| -> bool {
            matches!(
                state,
                core_domain::StrategyState::NoTrade
                    | core_domain::StrategyState::DeRisk
                    | core_domain::StrategyState::LeftProbe
            )
        };

        let mut recent_states: Vec<&core_domain::StrategyStateSnapshot> = self
            .states_history
            .iter()
            .filter(|s| s.date <= self.date)
            .collect();
        recent_states.sort_by(|a, b| b.date.cmp(&a.date));

        let mut duration: i64 = 0;
        for state_snapshot in &recent_states {
            if !is_conservative(&state_snapshot.state) {
                break;
            }
            let has_divergent = self
                .signal_history
                .get(&state_snapshot.date)
                .map(|signals| {
                    signals
                        .iter()
                        .any(|(_, label)| label == "STRONG_BUY" || label == "BUY")
                })
                .unwrap_or(false);
            if has_divergent {
                duration += 1;
            } else {
                break;
            }
        }
        duration
    }

    pub fn breadth_trend(&self) -> &'static str {
        match self.env {
            Some(ref env) => {
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
        }
    }

    pub fn rotation_pattern(&self) -> &'static str {
        let mut sorted = self.rotations.clone();
        sorted.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));
        let top_count = sorted.len().min(10);
        let top_10_avg_momentum: f64 = if top_count > 0 {
            sorted.iter().take(top_count).map(|r| r.momentum_score).sum::<f64>() / top_count as f64
        } else {
            0.0
        };

        if top_10_avg_momentum > 1.5 {
            "Technology Dominant"
        } else if top_10_avg_momentum < 0.3 {
            "Defensive"
        } else {
            "Mixed"
        }
    }

    pub fn signal_percentile(&self) -> f64 {
        let avg_signal = self.average_signal();
        let mut all_avg_signals: Vec<f64> = self
            .signal_history
            .values()
            .filter(|signals| !signals.is_empty())
            .map(|signals| {
                signals.iter().map(|(s, _)| s).sum::<f64>() / signals.len() as f64
            })
            .collect();
        all_avg_signals.sort_by(|a, b| a.total_cmp(b));

        if all_avg_signals.is_empty() {
            50.0
        } else {
            let below = all_avg_signals.iter().filter(|&&v| v < avg_signal).count();
            (below as f64 / all_avg_signals.len() as f64) * 100.0
        }
    }

    pub fn stretch_crowding(&self) -> (&'static str, f64, Option<f64>) {
        let total_momentum: f64 = self.rotations.iter().map(|r| r.momentum_score).sum();
        let mut sorted = self.rotations.clone();
        sorted.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));
        let top5_sum: f64 = sorted.iter().take(5).map(|r| r.momentum_score).sum();
        let concentration_pct = if total_momentum > 0.0 {
            (top5_sum / total_momentum) * 100.0
        } else {
            0.0
        };
        let level = classify_level(concentration_pct, 30.0, 50.0, true);
        (level, concentration_pct, None)
    }

    pub fn stretch_momentum(&self) -> (&'static str, f64, f64) {
        let rs120_max = self
            .rotations
            .iter()
            .map(|r| r.rs_120)
            .fold(f64::NEG_INFINITY, f64::max);
        let mut sorted = self.rotations.clone();
        sorted.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score));
        let top5: Vec<&RotationRankSnapshot> = sorted.iter().take(5).collect();
        let top5_rs120_avg = if !top5.is_empty() {
            top5.iter().map(|r| r.rs_120).sum::<f64>() / top5.len() as f64
        } else {
            0.0
        };
        let level = classify_level(rs120_max, 70.0, 85.0, true);
        (level, rs120_max, top5_rs120_avg)
    }

    pub fn stretch_breadth(&self) -> (&'static str, f64, Option<f64>) {
        match self.env {
            Some(ref env) => {
                let bp = env.breadth_pct;
                let sma5 = env.breadth_pct_sma5;
                let level = classify_level(bp, 35.0, 20.0, false);
                (level, bp, sma5)
            }
            None => ("Normal", 0.0, None),
        }
    }

    pub fn stretch_leverage(&self) -> &'static str {
        "Normal"
    }

    pub fn stretch_overall(&self) -> (&'static str, f64) {
        let (crowding_level, _, _) = self.stretch_crowding();
        let (breadth_level, _, _) = self.stretch_breadth();
        let (momentum_level, _, _) = self.stretch_momentum();
        let leverage_level = self.stretch_leverage();
        weighted_stretch_overall(crowding_level, breadth_level, momentum_level, leverage_level)
    }
}

/// Build a ResearchSnapshot for a given (date, scope) by fetching all required data.
pub fn build_research_snapshot(
    context: &AppContext,
    date: NaiveDate,
    scope_arg: ReportScopeArg,
) -> Result<ResearchSnapshot> {
    let scope: AnalysisScope = match scope_arg {
        ReportScopeArg::Global => AnalysisScope::Global,
        ReportScopeArg::Cn => AnalysisScope::Cn,
        ReportScopeArg::Hk => AnalysisScope::Hk,
    };

    // 1. Signals for the target date
    let signals = market_store::fetch_signal_snapshots_for_date_with_scope(
        &context.storage,
        date,
        scope,
    )?;

    // 2. Strategy states (history up to date)
    let states_history = market_store::fetch_strategy_states_for_scope(&context.storage, scope)?;
    let state = states_history.iter().find(|s| s.date == date).cloned();

    // 3. Signal history for percentile and divergence duration
    let lookback = date - chrono::Duration::days(365);
    let signal_query = format!(
        "SELECT date, final_score, signal_label FROM quant.signal_snapshot \
         WHERE date BETWEEN '{}' AND '{}' AND analysis_scope = '{}' \
         ORDER BY date FORMAT JSONEachRow",
        lookback,
        date,
        scope.as_str()
    );
    let signal_body = market_store::fetch_clickhouse_text(&context.storage, &signal_query)?;

    let mut signal_history: BTreeMap<NaiveDate, Vec<(f64, String)>> = BTreeMap::new();
    for line in signal_body.lines().filter(|l| !l.trim().is_empty()) {
        let row: serde_json::Value = serde_json::from_str(line)?;
        let Some(date_str) = row["date"].as_str() else { continue };
        let Ok(parsed_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else { continue };
        let score = row["final_score"].as_f64().unwrap_or(0.0);
        let label = row["signal_label"].as_str().unwrap_or("").to_string();
        signal_history.entry(parsed_date).or_default().push((score, label));
    }

    // 4. Rotation ranks for the target date (with scope filter)
    let mut rotations = market_store::fetch_rotation_ranks_for_date(&context.storage, date)?;
    if !matches!(scope_arg, ReportScopeArg::Global) {
        let instruments = context.seed_universe().unwrap_or_default();
        rotations = rotations
            .into_iter()
            .filter(|row| {
                instruments.iter().any(|inst| {
                    if inst.symbol != row.symbol {
                        return false;
                    }
                    match (scope_arg, &inst.market) {
                        (ReportScopeArg::Cn, core_domain::Market::Cn) => true,
                        (ReportScopeArg::Hk, core_domain::Market::Hk) => true,
                        _ => false,
                    }
                })
            })
            .collect::<Vec<_>>();
    }

    // 5. Environment history for breadth trend and current breadth
    let env_lookback = date - chrono::Duration::days(60);
    let env_history = market_store::fetch_environment_snapshots_for_scope(
        &context.storage,
        scope,
        env_lookback,
        date,
    )?;
    let env = env_history.iter().find(|e| e.date == date).cloned();

    Ok(ResearchSnapshot {
        date,
        signals,
        state,
        states_history,
        rotations,
        env,
        signal_history,
    })
}

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

    // 2. Build research snapshot
    let snapshot = build_research_snapshot(context, target_date, scope_arg)?;

    // 3. Compute SRD metrics from snapshot
    let strong_buy_count = snapshot.strong_buy_count();
    let buy_count = snapshot.buy_count();
    let avg_signal = snapshot.average_signal();
    let duration = snapshot.divergence_duration();
    let breadth_trend = snapshot.breadth_trend();
    let rotation_pattern = snapshot.rotation_pattern();
    let historical_percentile = snapshot.signal_percentile();
    let state_label = snapshot.state_label();

    // 4. Narrative helpers
    let interpretation = srd_interpretation(
        strong_buy_count,
        buy_count,
        breadth_trend,
        rotation_pattern,
        &state_label,
    );
    let confidence = srd_confidence(strong_buy_count, buy_count, breadth_trend, duration);
    let percentile_text = percentile_label(historical_percentile);

    // 5. Output clean text table
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
    scope_arg: ReportScopeArg,
    date: Option<NaiveDate>,
) -> Result<()> {
    use chrono::Duration;

    // 1. Resolve target date
    let report_date = match date {
        Some(d) => d,
        None => market_store::fetch_latest_table_date(&context.storage, "rotation_rank")?
            .unwrap_or_else(|| Local::now().date_naive()),
    };

    // 2. Build research snapshot
    let snapshot = build_research_snapshot(context, report_date, scope_arg)?;
    if snapshot.rotations.is_empty() {
        anyhow::bail!(
            "No rotation data available for date {} and scope {:?}",
            report_date, scope_arg
        );
    }

    // 3. Compute Stretch levels from snapshot
    let (crowding_level, concentration_pct, _) = snapshot.stretch_crowding();
    let (momentum_level, rs120_max, top5_rs120_avg) = snapshot.stretch_momentum();
    let (breadth_level, breadth_pct, breadth_sma5) = snapshot.stretch_breadth();
    let leverage_level = snapshot.stretch_leverage();
    let (overall, _weighted_score) = snapshot.stretch_overall();

    // 4. Historical crowding percentile (120-day lookback)
    let start_date = report_date - Duration::days(120);
    let hist_query = format!(
        "SELECT date,symbol,momentum_score FROM quant.rotation_rank WHERE date >= '{}' ORDER BY date FORMAT JSONEachRow",
        start_date
    );
    let hist_body = market_store::fetch_clickhouse_text(&context.storage, &hist_query)?;
    let hist_all: Vec<serde_json::Value> = market_store::parse_json_each_row(&hist_body, "rotation rank row")?;

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

    // 5. Evidence and narrative
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

    // 6. Output
    let scope_label = match scope_arg {
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
        if label == "STRONG_BUY" {
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

/// Quarterly Review — aggregate SRD/Stretch/Analytics over a window into a Markdown report.
/// This is a synthesis layer: it only observes and reports, never modifies decision logic.
const REVIEW_VERSION: &str = "v1";

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
        match build_research_snapshot(context, d, scope_arg) {
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
    let mut stretch_distribution: HashMap<&'static str, usize> = HashMap::new();
    let mut stretch_extreme_days: Vec<NaiveDate> = Vec::new();
    let mut stretch_elevated_days: Vec<NaiveDate> = Vec::new();
    let mut crowding_distribution: HashMap<&'static str, usize> = HashMap::new();
    let mut momentum_distribution: HashMap<&'static str, usize> = HashMap::new();
    let mut breadth_distribution: HashMap<&'static str, usize> = HashMap::new();

    for (date, snapshot) in &snapshots {
        let (overall, _) = snapshot.stretch_overall();
        *stretch_distribution.entry(overall).or_insert(0) += 1;
        match overall {
            "Extreme" => stretch_extreme_days.push(*date),
            "Elevated" => stretch_elevated_days.push(*date),
            _ => {}
        }

        let (crowding_level, _, _) = snapshot.stretch_crowding();
        *crowding_distribution.entry(crowding_level).or_insert(0) += 1;

        let (momentum_level, _, _) = snapshot.stretch_momentum();
        *momentum_distribution.entry(momentum_level).or_insert(0) += 1;

        let (breadth_level, _, _) = snapshot.stretch_breadth();
        *breadth_distribution.entry(breadth_level).or_insert(0) += 1;
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

    // ---- Build Markdown report ----
    let scope_label = match scope_arg {
        ReportScopeArg::Global => "GLOBAL",
        ReportScopeArg::Cn => "CN",
        ReportScopeArg::Hk => "HK",
    };
    let generated_at = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut md = String::new();
    md.push_str("# Research Quarterly Review\n\n");
    md.push_str(&format!("**Scope**: {}\n\n", scope_label));
    md.push_str(&format!("**Observation Window**: {} ~ {}\n\n", window_from, window_to));
    md.push_str(&format!("**Report Version**: {}\n\n", REVIEW_VERSION));
    md.push_str(&format!("**Generated At**: {}\n\n", generated_at));
    md.push_str("**Status**: Observation-only synthesis. Does not modify any decision logic.\n\n");
    md.push_str("---\n\n");

    // Observation coverage
    md.push_str("## Observation Coverage\n\n");
    md.push_str(&format!("- Days with complete research data: {}\n", snapshots.len()));
    md.push_str(&format!("- Calendar window: {} days\n", (window_to - window_from).num_days() + 1));
    md.push_str("\n");

    // SRD section
    md.push_str("## Signal-Regime Divergence (SRD) Summary\n\n");
    if srd_days.is_empty() {
        md.push_str("No SRD events observed in this window.\n\n");
    } else {
        md.push_str(&format!("- SRD days: {}\n", srd_days.len()));
        md.push_str(&format!(
            "- SRD frequency: {:.1}%\n",
            (srd_days.len() as f64 / snapshots.len() as f64) * 100.0
        ));
        let avg_duration = srd_durations.iter().sum::<i64>() as f64 / srd_durations.len() as f64;
        md.push_str(&format!("- Average divergence duration: {:.1} days\n", avg_duration));
        md.push_str(&format!("- Longest consecutive SRD streak: {} days\n", longest_streak));
        md.push_str("\n**Latest SRD dates**:\n\n");
        for date in srd_days.iter().rev().take(10) {
            md.push_str(&format!("- {}\n", date));
        }
        md.push_str("\n");
    }

    // Stretch section
    md.push_str("## Market Stretch Distribution\n\n");
    md.push_str("### Overall\n\n");
    for level in &["Normal", "Elevated", "Extreme"] {
        let count = stretch_distribution.get(level).copied().unwrap_or(0);
        md.push_str(&format!("- {}: {} days\n", level, count));
    }
    md.push_str("\n### By Dimension\n\n");
    md.push_str("**Crowding**:\n\n");
    for level in &["Normal", "Elevated", "Extreme"] {
        let count = crowding_distribution.get(level).copied().unwrap_or(0);
        md.push_str(&format!("- {}: {} days\n", level, count));
    }
    md.push_str("\n**Momentum**:\n\n");
    for level in &["Normal", "Elevated", "Extreme"] {
        let count = momentum_distribution.get(level).copied().unwrap_or(0);
        md.push_str(&format!("- {}: {} days\n", level, count));
    }
    md.push_str("\n**Breadth**:\n\n");
    for level in &["Normal", "Elevated", "Extreme"] {
        let count = breadth_distribution.get(level).copied().unwrap_or(0);
        md.push_str(&format!("- {}: {} days\n", level, count));
    }
    md.push_str("\n");

    // Analytics section
    md.push_str("## Conditional Forward-Return Analytics\n\n");
    md.push_str("All statistics are historical observations only.\n\n");
    for section in analytics_sections {
        md.push_str(&section);
        md.push_str("\n");
    }

    // Evidence worth ADR review
    md.push_str("## Evidence Worth ADR Review\n\n");
    md.push_str("The following observations are worth tracking during Shadow Production. They do not imply any system change.\n\n");
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
    if review_points.is_empty() {
        md.push_str("No strong evidence requiring ADR review was observed in this window.\n\n");
    } else {
        for c in review_points {
            md.push_str(&format!("- {}\n", c));
        }
        md.push_str("\n");
    }

    // Disclaimer
    md.push_str("---\n\n");
    md.push_str("**Disclaimer**: This report is produced by the Research Layer for evidence accumulation only. ");
    md.push_str("It does not modify Strategy State, Signal, Execution, or Risk logic. ");
    md.push_str("Historical statistics are not predictions of future returns.\n");

    // Write report
    let output_path = output.unwrap_or_else(|| {
        std::path::PathBuf::from(format!(
            "reports/research-quarterly-{}-{}.md",
            scope_label.to_lowercase(),
            window_to
        ))
    });
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output_path, md)?;

    println!("Research Quarterly Review generated:");
    println!("  Scope:   {}", scope_label);
    println!("  Window:  {} ~ {}", window_from, window_to);
    println!("  Days:    {}", snapshots.len());
    println!("  Output:  {}", output_path.display());

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


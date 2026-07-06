use anyhow::Result;
use app_service::{AppContext, ReportScope};
use chrono::{Local, NaiveDate};
use core_domain::MarketRegimeSnapshot;
use market_store::StorageConfig;
use research_validation::{HistoricalValidator, RegimeLabeler, ReportGenerator};
use std::collections::HashMap;
use crate::ReportScopeArg;

// ------------------------------------------------------------------
// Shared helpers
// ------------------------------------------------------------------

fn count_trend_directions(
    observations: &[market_state_extractor::MarketStateObservation],
    short_term: bool,
) -> std::collections::HashMap<String, String> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for obs in observations {
        let dir = if short_term {
            format!("{:?}", obs.trend.short_term)
        } else {
            format!("{:?}", obs.trend.medium_term)
        };
        *counts.entry(dir).or_insert(0) += 1;
    }
    let n = observations.len() as f64;
    counts
        .into_iter()
        .map(|(k, v)| (k, format!("{:.1}%", v as f64 / n * 100.0)))
        .collect()
}

fn inspect_regime_labels(regimes: &[MarketRegimeSnapshot]) -> serde_json::Value {
    use std::collections::HashMap;

    let n = regimes.len();
    if n == 0 {
        return serde_json::json!({"error": "no regime data"});
    }

    // 1. Class Distribution
    let mut class_counts: HashMap<&str, usize> = HashMap::new();
    for r in regimes {
        *class_counts.entry(r.regime_label.as_str()).or_insert(0) += 1;
    }

    let class_distribution: HashMap<String, serde_json::Value> = class_counts
        .iter()
        .map(|(&label, &count)| {
            let pct = count as f64 / n as f64 * 100.0;
            (label.to_string(), serde_json::json!({"count": count, "pct": format!("{:.1}%", pct)}))
        })
        .collect();

    // 2. Transition Matrix
    let mut transitions: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for window in regimes.windows(2) {
        let from = window[0].regime_label.as_str();
        let to = window[1].regime_label.as_str();
        transitions
            .entry(from.to_string())
            .or_default()
            .entry(to.to_string())
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }

    // 3. Episode Statistics
    let mut episodes: HashMap<String, Vec<usize>> = HashMap::new();
    let mut current_label: Option<String> = None;
    let mut current_len = 0usize;

    for r in regimes {
        let label = r.regime_label.clone();
        if Some(&label) == current_label.as_ref() {
            current_len += 1;
        } else {
            if let Some(prev) = current_label.take() {
                episodes.entry(prev).or_default().push(current_len);
            }
            current_label = Some(label);
            current_len = 1;
        }
    }
    if let Some(prev) = current_label.take() {
        episodes.entry(prev).or_default().push(current_len);
    }

    let episode_stats: HashMap<String, serde_json::Value> = episodes
        .iter()
        .map(|(label, durations)| {
            let count = durations.len();
            let total: usize = durations.iter().sum();
            let avg = total as f64 / count as f64;
            let min = *durations.iter().min().unwrap_or(&0);
            let max = *durations.iter().max().unwrap_or(&0);
            (
                label.clone(),
                serde_json::json!({
                    "episode_count": count,
                    "total_days": total,
                    "avg_duration": format!("{:.1}", avg),
                    "min_duration": min,
                    "max_duration": max,
                }),
            )
        })
        .collect();

    // 4. Regime Persistence (churn rate)
    let transitions_total = n.saturating_sub(1);
    let changes = regimes
        .windows(2)
        .filter(|w| w[0].regime_label != w[1].regime_label)
        .count();
    let churn_rate = if transitions_total > 0 {
        changes as f64 / transitions_total as f64
    } else {
        0.0
    };

    // 5. State Imbalance Report
    let max_class = class_counts.values().copied().max().unwrap_or(0);
    let min_class = class_counts.values().copied().min().unwrap_or(0);
    let imbalance_ratio = if min_class > 0 {
        max_class as f64 / min_class as f64
    } else {
        f64::INFINITY
    };

    serde_json::json!({
        "total_samples": n,
        "date_range": {
            "from": regimes.first().map(|r| r.date.to_string()),
            "to": regimes.last().map(|r| r.date.to_string()),
        },
        "class_distribution": class_distribution,
        "transition_matrix": transitions,
        "episode_statistics": episode_stats,
        "regime_persistence": {
            "transitions_total": transitions_total,
            "regime_changes": changes,
            "churn_rate": format!("{:.2}%", churn_rate * 100.0),
            "persistence_ratio": format!("{:.2}%", (1.0 - churn_rate) * 100.0),
        },
        "state_imbalance": {
            "max_class_count": max_class,
            "min_class_count": min_class,
            "imbalance_ratio": if imbalance_ratio.is_finite() { format!("{:.1}x", imbalance_ratio) } else { "infinite".to_string() },
            "assessment": if imbalance_ratio > 50.0 { "severely_imbalanced" } else if imbalance_ratio > 10.0 { "imbalanced" } else { "balanced" },
        },
    })
}

// ------------------------------------------------------------------
// ValidateRegimeAccuracy
// ------------------------------------------------------------------

pub fn handle_validate_regime_accuracy(
    _context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
    benchmark: Option<String>,
    lookforward_days: usize,
    risk_on_threshold: f64,
    risk_off_threshold: f64,
    output_dir: String,
) -> Result<()> {
    let storage = StorageConfig::default();
    let report_scope: ReportScope = scope.into();
    let scope_str = report_scope.as_str();

    // Determine benchmark symbol
    let benchmark_symbol = benchmark.unwrap_or_else(|| match scope {
        ReportScopeArg::Cn => "000300".to_string(),
        ReportScopeArg::Hk => "HSCEI".to_string(),
        ReportScopeArg::Global => "000300".to_string(),
    });

    eprintln!(
        "[ground-truth] Validating {} regime predictions against {} forward returns ({}-day)",
        scope_str, benchmark_symbol, lookforward_days
    );

    // 1. Fetch daily bars for benchmark
    let bars = market_store::fetch_daily_bars(&storage, &benchmark_symbol)?;
    let bars_in_window: Vec<_> = bars
        .into_iter()
        .filter(|bar| bar.date >= from && bar.date <= to)
        .collect();
    if bars_in_window.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", benchmark_symbol, from, to);
    }

    // 2. Fetch market regimes and filter by scope
    let regimes = market_store::fetch_market_regimes(&storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    // 3. Label ground truth from forward returns
    let labeler = RegimeLabeler {
        lookforward_days,
        risk_on_threshold,
        risk_off_threshold,
    };
    let ground_truth = labeler.label(&bars_in_window);
    if ground_truth.is_empty() {
        anyhow::bail!(
            "Not enough forward data to label ground truth (need {} days beyond {})",
            lookforward_days,
            to
        );
    }

    // 4. Build predictions map from stored regimes
    let mut predictions = HashMap::new();
    for regime in &regimes_in_window {
        predictions.insert((regime.date, benchmark_symbol.clone()), regime.regime_label.clone());
    }

    // 5. Validate
    let validation_result = HistoricalValidator::validate(&ground_truth, &predictions);

    // 6. Generate report
    let report = ReportGenerator::generate(
        &format!("market-regime-{}-vs-{}", scope_str, benchmark_symbol),
        &validation_result,
    );

    // 7. Write outputs
    let output_path = std::path::PathBuf::from(&output_dir);
    std::fs::create_dir_all(&output_path)?;
    let base_name = format!(
        "ground-truth-{}-{}-{}-to-{}",
        scope_str.to_lowercase(),
        benchmark_symbol.to_lowercase(),
        from,
        to
    );

    let json_path = output_path.join(format!("{}.json", base_name));
    let md_path = output_path.join(format!("{}.md", base_name));

    let json_report = ReportGenerator::to_json(&report)?;
    std::fs::write(&json_path, json_report)?;

    let md_report = ReportGenerator::to_markdown(&report);
    std::fs::write(&md_path, md_report)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": scope_str,
            "benchmark": benchmark_symbol,
            "window": {
                "from": from.to_string(),
                "to": to.to_string(),
            },
            "summary": {
                "total_samples": report.total_samples,
                "correct_predictions": report.correct_predictions,
                "accuracy": report.accuracy,
                "macro_precision": report.macro_precision,
                "macro_recall": report.macro_recall,
                "macro_f1": report.macro_f1,
            },
            "outputs": {
                "json": json_path.to_string_lossy(),
                "markdown": md_path.to_string_lossy(),
            }
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// InspectGroundTruth
// ------------------------------------------------------------------

pub fn handle_inspect_ground_truth(
    _context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let storage = StorageConfig::default();
    let report_scope: ReportScope = scope.into();
    let scope_str = report_scope.as_str();

    // Fetch regimes and filter
    let regimes = market_store::fetch_market_regimes(&storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();

    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let inspection = inspect_regime_labels(&regimes_in_window);
    println!("{}", serde_json::to_string_pretty(&inspection)?);
    Ok(())
}

// ------------------------------------------------------------------
// GenerateRegimeLabels
// ------------------------------------------------------------------

pub fn handle_generate_regime_labels(
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
    min_days: usize,
    confirmation_days: usize,
    use_percentile: bool,
) -> Result<()> {
    let storage = StorageConfig::default();
    let report_scope: ReportScope = scope.into();
    let scope_str = report_scope.as_str();

    // Fetch regimes and filter
    let regimes = market_store::fetch_market_regimes(&storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();

    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    // Fetch environment snapshots for real breadth and proxy scores
    let env_snapshots = market_store::fetch_environment_snapshots_for_scope(
        &storage,
        core_domain::AnalysisScope::Global, // use GLOBAL env as proxy if scope-specific unavailable
        from,
        to,
    ).unwrap_or_default();
    let env_by_date: std::collections::HashMap<_, _> = env_snapshots
        .into_iter()
        .map(|e| (e.date, e))
        .collect();

    // Build observations using real environment data where available
    let observations: Vec<core_domain::RegimeObservation> = regimes_in_window
        .iter()
        .map(|r| {
            let env = env_by_date.get(&r.date);
            let breadth = env.map(|e| e.breadth_pct).unwrap_or(50.0);
            let liquidity = env.map(|e| e.liquidity_proxy_score).unwrap_or(r.liquidity_score);
            let volatility = env.map(|e| e.stress_proxy_score).unwrap_or(r.risk_score);
            research_validation::RegimeLabelGenerator::build_observation(
                r.date,
                &r.market,
                r.trend_score,
                breadth,
                liquidity,
                volatility,
            )
        })
        .collect();

    // Compute percentile thresholds if requested
    let thresholds = if use_percentile {
        Some(research_validation::RegimeLabelGenerator::compute_percentile_thresholds(&observations))
    } else {
        None
    };

    // Apply persistence filter
    let config = research_validation::PersistenceConfig {
        min_days,
        confirmation_days,
    };
    let mut filter = if let Some(t) = thresholds {
        research_validation::PersistenceFilter::with_thresholds(config, t)
    } else {
        research_validation::PersistenceFilter::new(config)
    };
    let stable_labels = filter.process_sequence(&observations);

    // Build pseudo MarketRegimeSnapshot rows for audit
    let pseudo_regimes: Vec<MarketRegimeSnapshot> = stable_labels
        .iter()
        .map(|(date, label)| MarketRegimeSnapshot {
            date: *date,
            macro_as_of_date: *date,
            market: scope_str.to_string(),
            trend_score: 50.0,
            liquidity_score: 50.0,
            risk_score: 50.0,
            regime_label: label.clone(),
        })
        .collect();

    let inspection = inspect_regime_labels(&pseudo_regimes);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": scope_str,
            "window": { "from": from.to_string(), "to": to.to_string() },
            "config": {
                "min_days": min_days,
                "confirmation_days": confirmation_days,
            },
            "total_observations": observations.len(),
            "stable_labels": stable_labels.len(),
            "audit": inspection,
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// AuditGtRegime
// ------------------------------------------------------------------

pub fn handle_audit_gt_regime(
    _context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
    confirmation_days: usize,
    min_days: usize,
) -> Result<()> {
    let storage = StorageConfig::default();
    let scope_str = match scope {
        ReportScopeArg::Global => "GLOBAL",
        ReportScopeArg::Cn => "CN",
        ReportScopeArg::Hk => "HK",
    };

    let anchor_symbol = match scope {
        ReportScopeArg::Cn => "000300",
        ReportScopeArg::Hk => "HSCEI",
        ReportScopeArg::Global => "000300",
    };

    let bars = market_store::fetch_daily_bars(&storage, anchor_symbol)?;
    let bars_in_window: Vec<_> = bars
        .into_iter()
        .filter(|bar| bar.date >= from && bar.date <= to)
        .collect();
    if bars_in_window.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars_in_window);
    if indicators.len() != bars_in_window.len() {
        anyhow::bail!("Indicator count mismatch: {} bars vs {} indicators", bars_in_window.len(), indicators.len());
    }

    let observations = market_state_extractor::build_market_state_observations(
        &bars_in_window,
        &indicators,
        scope_str,
    );

    let config = gt_regime_generator::PersistenceConfig {
        min_days,
        confirmation_days,
    };
    let mut pipeline = gt_regime_generator::RegimePipeline::with_config(scope_str, config);
    let labels = pipeline.process_sequence(&observations);

    if labels.is_empty() {
        anyhow::bail!("No regime labels generated");
    }

    let audit_report = regime_audit::audit_regime_labels_default(&labels);

    let episode_distribution = &audit_report.persistence.distribution;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": if audit_report.passed { "passed" } else { "failed" },
            "scope": scope_str,
            "anchor_symbol": anchor_symbol,
            "window": {
                "from": from.to_string(),
                "to": to.to_string(),
                "total_days": audit_report.total_days,
            },
            "persistence": {
                "avg_episode_days": audit_report.persistence.avg_episode_days,
                "median_episode_days": audit_report.persistence.median_episode_days,
                "distribution": {
                    "p25": episode_distribution.p25,
                    "p50": episode_distribution.p50,
                    "p75": episode_distribution.p75,
                    "p95": episode_distribution.p95,
                },
                "churn_rate": format!("{:.2}%", audit_report.persistence.churn_rate * 100.0),
                "transition_stability": audit_report.persistence.transition_stability,
            },
            "coverage": {
                "risk_on_pct": format!("{:.1}%", audit_report.coverage.risk_on_pct * 100.0),
                "neutral_pct": format!("{:.1}%", audit_report.coverage.neutral_pct * 100.0),
                "risk_off_pct": format!("{:.1}%", audit_report.coverage.risk_off_pct * 100.0),
                "imbalance_ratio": format!("{:.1}x", audit_report.coverage.imbalance_ratio),
            },
            "episodes": {
                "total_episodes": audit_report.episode_count,
                "transition_count": audit_report.transition_count,
                "direct_swing_count": audit_report.direct_swing_count,
            },
            "violations": audit_report.violations,
            "gates": {
                "min_avg_episode_days": 20.0,
                "min_median_episode_days": 15.0,
                "max_churn_rate": "5%",
                "min_transition_stability": 0.90,
                "max_imbalance_ratio": "5x",
                "min_risk_on_pct": "5%",
                "min_risk_off_pct": "5%",
            }
        }))?
    );

    if !audit_report.passed {
        std::process::exit(1);
    }
    Ok(())
}

// ------------------------------------------------------------------
// AuditGtTransitions
// ------------------------------------------------------------------

pub fn handle_audit_gt_transitions(
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
    confirmation_days: usize,
    min_days: usize,
) -> Result<()> {
    let storage = StorageConfig::default();
    let scope_str = match scope {
        ReportScopeArg::Global => "GLOBAL",
        ReportScopeArg::Cn => "CN",
        ReportScopeArg::Hk => "HK",
    };

    let anchor_symbol = match scope {
        ReportScopeArg::Cn => "000300",
        ReportScopeArg::Hk => "HSCEI",
        ReportScopeArg::Global => "000300",
    };

    let bars = market_store::fetch_daily_bars(&storage, anchor_symbol)?;
    let bars_in_window: Vec<_> = bars
        .into_iter()
        .filter(|bar| bar.date >= from && bar.date <= to)
        .collect();
    if bars_in_window.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars_in_window);
    let observations = market_state_extractor::build_market_state_observations(
        &bars_in_window,
        &indicators,
        scope_str,
    );

    let config = gt_regime_generator::PersistenceConfig {
        min_days,
        confirmation_days,
    };
    let mut pipeline = gt_regime_generator::RegimePipeline::with_config(scope_str, config);
    let labels = pipeline.process_sequence(&observations);

    if labels.is_empty() {
        anyhow::bail!("No regime labels generated");
    }

    let transition_report = regime_audit::audit_transitions(&labels);

    let swings_json: Vec<serde_json::Value> = transition_report
        .direct_swings
        .iter()
        .map(|s| {
            serde_json::json!({
                "date": s.date.to_string(),
                "from_regime": format!("{:?}", s.from_regime).to_lowercase(),
                "to_regime": format!("{:?}", s.to_regime).to_lowercase(),
                "from_candidate": format!("{:?}", s.from_candidate).to_lowercase(),
                "to_candidate": format!("{:?}", s.to_candidate).to_lowercase(),
                "days_in_previous_regime": s.days_in_previous_regime,
            })
        })
        .collect();

    let paths_json: Vec<serde_json::Value> = transition_report
        .transition_paths
        .iter()
        .map(|p| {
            serde_json::json!({
                "from": p.from,
                "to": p.to,
                "count": p.count,
                "pct": format!("{:.1}%", p.pct),
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": scope_str,
            "anchor_symbol": anchor_symbol,
            "window": { "from": from.to_string(), "to": to.to_string(), "total_days": labels.len() },
            "candidate_distribution": transition_report.candidate_distribution,
            "regime_distribution": transition_report.regime_distribution,
            "transitions": {
                "total": transition_report.total_transitions,
                "direct_swings": transition_report.direct_swing_count,
                "direct_swing_ratio": format!("{:.1}%", transition_report.direct_swing_ratio * 100.0),
                "paths": paths_json,
            },
            "swing_events": swings_json,
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// AuditGtCandidates
// ------------------------------------------------------------------

pub fn handle_audit_gt_candidates(
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let storage = StorageConfig::default();
    let scope_str = match scope {
        ReportScopeArg::Global => "GLOBAL",
        ReportScopeArg::Cn => "CN",
        ReportScopeArg::Hk => "HK",
    };

    let anchor_symbol = match scope {
        ReportScopeArg::Cn => "000300",
        ReportScopeArg::Hk => "HSCEI",
        ReportScopeArg::Global => "000300",
    };

    let bars = market_store::fetch_daily_bars(&storage, anchor_symbol)?;
    let bars_in_window: Vec<_> = bars
        .into_iter()
        .filter(|bar| bar.date >= from && bar.date <= to)
        .collect();
    if bars_in_window.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars_in_window);
    let observations = market_state_extractor::build_market_state_observations(
        &bars_in_window,
        &indicators,
        scope_str,
    );

    let report = regime_audit::audit_factor_attribution(&observations);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": scope_str,
            "anchor_symbol": anchor_symbol,
            "window": { "from": from.to_string(), "to": to.to_string(), "total_days": observations.len() },
            "trend": {
                "short_term": report.trend_short_distribution,
                "medium_term": report.trend_medium_distribution,
            },
            "volatility": report.volatility_distribution,
            "liquidity": report.liquidity_distribution,
            "drawdown": {
                "avg": format!("{:.1}%", report.drawdown_stats.avg),
                "median": format!("{:.1}%", report.drawdown_stats.median),
                "p10": format!("{:.1}%", report.drawdown_stats.p10),
                "p90": format!("{:.1}%", report.drawdown_stats.p90),
                "max": format!("{:.1}%", report.drawdown_stats.max),
            },
            "risk_on_triggers": report.risk_on_trigger_breakdown,
            "risk_off_triggers": report.risk_off_trigger_breakdown,
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// ValidateGtRegimes
// ------------------------------------------------------------------

pub fn handle_validate_gt_regimes(
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
    confirmation_days: usize,
    min_days: usize,
) -> Result<()> {
    let storage = StorageConfig::default();
    let scope_str = match scope {
        ReportScopeArg::Global => "GLOBAL",
        ReportScopeArg::Cn => "CN",
        ReportScopeArg::Hk => "HK",
    };

    let anchor_symbol = match scope {
        ReportScopeArg::Cn => "000300",
        ReportScopeArg::Hk => "HSCEI",
        ReportScopeArg::Global => "000300",
    };

    let bars = market_store::fetch_daily_bars(&storage, anchor_symbol)?;
    let bars_in_window: Vec<_> = bars
        .into_iter()
        .filter(|bar| bar.date >= from && bar.date <= to)
        .collect();
    if bars_in_window.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars_in_window);
    let observations = market_state_extractor::build_market_state_observations(
        &bars_in_window,
        &indicators,
        scope_str,
    );

    let config = gt_regime_generator::PersistenceConfig {
        min_days,
        confirmation_days,
    };
    let mut pipeline = gt_regime_generator::RegimePipeline::with_config(scope_str, config);
    let labels = pipeline.process_sequence(&observations);

    if labels.is_empty() {
        anyhow::bail!("No regime labels generated");
    }

    let report = regime_audit::external_validation::validate_regimes_economically(
        &labels,
        &bars_in_window,
        scope_str,
        anchor_symbol,
    );

    let stats_json: HashMap<String, serde_json::Value> = report
        .stats
        .iter()
        .map(|(regime, stat)| {
            (
                regime.clone(),
                serde_json::json!({
                    "count": stat.count,
                    "pct": format!("{:.1}%", stat.pct * 100.0),
                    "forward_return_20d_median": format!("{:.2}%", stat.forward_return_20d_median * 100.0),
                    "forward_return_60d_median": format!("{:.2}%", stat.forward_return_60d_median * 100.0),
                    "forward_return_20d_mean": format!("{:.2}%", stat.forward_return_20d_mean * 100.0),
                    "forward_return_60d_mean": format!("{:.2}%", stat.forward_return_60d_mean * 100.0),
                    "max_drawdown_median": format!("{:.2}%", stat.max_drawdown_median * 100.0),
                    "volatility_median": format!("{:.2}%", stat.volatility_median * 100.0),
                    "sharpe_median": format!("{:.2}", stat.sharpe_median),
                    "win_rate_20d": format!("{:.1}%", stat.win_rate_20d * 100.0),
                    "win_rate_60d": format!("{:.1}%", stat.win_rate_60d * 100.0),
                }),
            )
        })
        .collect();

    let gates = &report.separation_score.gate_results;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": scope_str,
            "anchor_symbol": anchor_symbol,
            "window": {
                "from": from.to_string(),
                "to": to.to_string(),
                "total_days": report.total_days,
            },
            "assessment": report.assessment,
            "separation_score": report.separation_score.overall_score,
            "gates": gates,
            "regime_stats": stats_json,
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// AuditObservationLayer
// ------------------------------------------------------------------

pub fn handle_audit_observation_layer(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };

    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars);
    let observations = market_state_extractor::build_market_state_observations(
        &bars,
        &indicators,
        scope_str,
    );

    let mut short_directions: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut medium_directions: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for obs in &observations {
        *short_directions.entry(format!("{:?}", obs.trend.short_term)).or_insert(0) += 1;
        *medium_directions.entry(format!("{:?}", obs.trend.medium_term)).or_insert(0) += 1;
    }

    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let mut actual_short_slopes: Vec<f64> = Vec::new();
    let mut actual_medium_slopes: Vec<f64> = Vec::new();
    for i in 0..closes.len() {
        if i >= 19 {
            actual_short_slopes.push(market_state_extractor::slope_approx(&closes, i, 20));
        }
        if i >= 59 {
            actual_medium_slopes.push(market_state_extractor::slope_approx(&closes, i, 60));
        }
    }

    fn percentile(sorted: &[f64], p: f64) -> f64 {
        if sorted.is_empty() { return 0.0; }
        let idx = ((sorted.len() - 1) as f64 * p) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    actual_short_slopes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    actual_medium_slopes.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = observations.len() as f64;
    let short_dist: std::collections::HashMap<String, String> = short_directions
        .iter()
        .map(|(k, v)| (k.clone(), format!("{:.1}%", *v as f64 / n * 100.0)))
        .collect();
    let medium_dist: std::collections::HashMap<String, String> = medium_directions
        .iter()
        .map(|(k, v)| (k.clone(), format!("{:.1}%", *v as f64 / n * 100.0)))
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": scope_str,
            "anchor_symbol": anchor_symbol,
            "window": {
                "from": from.to_string(),
                "to": to.to_string(),
                "total_days": observations.len(),
            },
            "short_term_slope_distribution": short_dist,
            "medium_term_slope_distribution": medium_dist,
            "actual_slope_percentiles": {
                "short_p5": format!("{:.4}", percentile(&actual_short_slopes, 0.05)),
                "short_p25": format!("{:.4}", percentile(&actual_short_slopes, 0.25)),
                "short_p50": format!("{:.4}", percentile(&actual_short_slopes, 0.50)),
                "short_p75": format!("{:.4}", percentile(&actual_short_slopes, 0.75)),
                "short_p95": format!("{:.4}", percentile(&actual_short_slopes, 0.95)),
                "medium_p5": format!("{:.4}", percentile(&actual_medium_slopes, 0.05)),
                "medium_p25": format!("{:.4}", percentile(&actual_medium_slopes, 0.25)),
                "medium_p50": format!("{:.4}", percentile(&actual_medium_slopes, 0.50)),
                "medium_p75": format!("{:.4}", percentile(&actual_medium_slopes, 0.75)),
                "medium_p95": format!("{:.4}", percentile(&actual_medium_slopes, 0.95)),
            },
            "threshold_analysis": {
                "current_threshold": "0.001 (absolute)",
                "recommendation": "threshold should be relative to price level or use percentile-based approach",
            },
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// ReplayTrendSensitivity
// ------------------------------------------------------------------

pub fn handle_replay_trend_sensitivity(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };

    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars);

    let methods = [
        ("Baseline", market_state_extractor::TrendDirectionMethod::Baseline),
        ("RelativeSlope", market_state_extractor::TrendDirectionMethod::RelativeSlope),
        ("Percentile", market_state_extractor::TrendDirectionMethod::Percentile),
        ("ZScore", market_state_extractor::TrendDirectionMethod::ZScore),
    ];

    let mut results = Vec::new();

    for (name, method) in &methods {
        let observations = market_state_extractor::extract_market_state_observations_with_method(
            &bars,
            &indicators,
            scope_str,
            *method,
        );

        let config = gt_regime_generator::PersistenceConfig {
            min_days: 5,
            confirmation_days: 10,
        };
        let mut pipeline = gt_regime_generator::RegimePipeline::with_config(scope_str, config);
        let labels = pipeline.process_sequence(&observations);

        let audit_report = regime_audit::audit_regime_labels_default(&labels);

        let val_report = if labels.is_empty() {
            None
        } else {
            Some(regime_audit::external_validation::validate_regimes_economically(
                &labels,
                &bars,
                scope_str,
                anchor_symbol,
            ))
        };

        let result = serde_json::json!({
            "variant": name,
            "observation_distribution": {
                "short_term": count_trend_directions(&observations, true),
                "medium_term": count_trend_directions(&observations, false),
            },
            "audit": {
                "passed": audit_report.passed,
                "violations": audit_report.violations,
                "avg_episode_days": audit_report.persistence.avg_episode_days,
                "median_episode_days": audit_report.persistence.median_episode_days,
                "churn_rate": audit_report.persistence.churn_rate,
                "transition_stability": audit_report.persistence.transition_stability,
                "coverage": {
                    "risk_on_pct": audit_report.coverage.risk_on_pct,
                    "neutral_pct": audit_report.coverage.neutral_pct,
                    "risk_off_pct": audit_report.coverage.risk_off_pct,
                    "imbalance_ratio": audit_report.coverage.imbalance_ratio,
                },
            },
            "validation": val_report.as_ref().map(|r| serde_json::json!({
                "assessment": r.assessment,
                "separation_score": r.separation_score.overall_score,
                "regime_stats": {
                    "riskon": r.stats.get("riskon").map(|s| serde_json::json!({
                        "count": s.count,
                        "pct": format!("{:.1}%", s.pct * 100.0),
                        "return_60d": format!("{:.2}%", s.forward_return_60d_mean * 100.0),
                        "sharpe": format!("{:.2}", s.sharpe_median),
                        "drawdown": format!("{:.2}%", s.max_drawdown_median * 100.0),
                        "winrate_60d": format!("{:.1}%", s.win_rate_60d * 100.0),
                    })),
                    "neutral": r.stats.get("neutral").map(|s| serde_json::json!({
                        "count": s.count,
                        "pct": format!("{:.1}%", s.pct * 100.0),
                        "return_60d": format!("{:.2}%", s.forward_return_60d_mean * 100.0),
                        "sharpe": format!("{:.2}", s.sharpe_median),
                        "drawdown": format!("{:.2}%", s.max_drawdown_median * 100.0),
                        "winrate_60d": format!("{:.1}%", s.win_rate_60d * 100.0),
                    })),
                    "riskoff": r.stats.get("riskoff").map(|s| serde_json::json!({
                        "count": s.count,
                        "pct": format!("{:.1}%", s.pct * 100.0),
                        "return_60d": format!("{:.2}%", s.forward_return_60d_mean * 100.0),
                        "sharpe": format!("{:.2}", s.sharpe_median),
                        "drawdown": format!("{:.2}%", s.max_drawdown_median * 100.0),
                        "winrate_60d": format!("{:.1}%", s.win_rate_60d * 100.0),
                    })),
                },
            })),
        });
        results.push(result);
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": scope_str,
            "anchor_symbol": anchor_symbol,
            "window": {
                "from": from.to_string(),
                "to": to.to_string(),
                "total_days": bars.len(),
            },
            "variants": results,
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// GtSensitivityReplay
// ------------------------------------------------------------------

pub fn handle_gt_sensitivity_replay(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };

    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars);

    let report = regime_audit::sensitivity_replay::run_sensitivity_replay(
        &bars,
        &indicators,
        scope_str,
        anchor_symbol,
    );

    let comparison_json: Vec<serde_json::Value> = report
        .comparison
        .iter()
        .map(|r| {
            serde_json::json!({
                "variant": r.variant,
                "separation_score": r.separation_score,
                "gates_passed": r.gates_passed,
                "riskon_return_60d": r.riskon_return_60d,
                "riskoff_return_60d": r.riskoff_return_60d,
                "churn_rate": r.churn_rate,
                "imbalance_ratio": r.imbalance_ratio,
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "scope": report.scope,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
            },
            "comparison": comparison_json,
            "recommendation": report.recommendation,
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// AuditAttribution
// ------------------------------------------------------------------

pub fn handle_audit_attribution(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };

    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars);
    let observations = market_state_extractor::build_market_state_observations(
        &bars,
        &indicators,
        scope_str,
    );

    // Audit 1: Candidate Coverage
    let coverage = regime_audit::attribution::audit_candidate_coverage(
        &observations,
        &bars,
        scope_str,
    );

    // Audit 2: Trigger Attribution
    let triggers = regime_audit::attribution::audit_trigger_attribution(
        &observations,
        &bars,
        scope_str,
    );

    // Audit 3: Confusion Against Returns
    let confusion = regime_audit::attribution::audit_confusion_against_returns(
        &observations,
        &bars,
        scope_str,
    );

    let mut coverage_map = serde_json::Map::new();
    for (candidate, stat) in &coverage.stats {
        coverage_map.insert(
            candidate.clone(),
            serde_json::json!({
                "count": stat.count,
                "pct": format!("{:.1}%", stat.pct * 100.0),
                "forward_return_20d_mean": format!("{:.2}%", stat.forward_return_20d_mean * 100.0),
                "forward_return_60d_mean": format!("{:.2}%", stat.forward_return_60d_mean * 100.0),
                "max_drawdown_median": format!("{:.2}%", stat.max_drawdown_median * 100.0),
                "sharpe_median": format!("{:.2}", stat.sharpe_median),
            }),
        );
    }
    let coverage_json = serde_json::Value::Object(coverage_map);

    let mut trigger_map = serde_json::Map::new();
    for (candidate, triggers) in &triggers.trigger_breakdown {
        let mut inner = serde_json::Map::new();
        for (name, stat) in triggers {
            inner.insert(
                name.clone(),
                serde_json::json!({
                    "count": stat.count,
                    "pct": format!("{:.1}%", stat.pct_of_regime * 100.0),
                    "avg_60d_return": format!("{:.2}%", stat.avg_60d_return * 100.0),
                }),
            );
        }
        trigger_map.insert(candidate.clone(), serde_json::Value::Object(inner));
    }
    let trigger_json = serde_json::Value::Object(trigger_map);

    let confusion_json = serde_json::json!({
        "top_quartile": confusion.top_quartile_distribution.iter().map(|(k, v)| (k.clone(), format!("{:.1}%", v * 100.0))).collect::<std::collections::HashMap<String, String>>(),
        "bottom_quartile": confusion.bottom_quartile_distribution.iter().map(|(k, v)| (k.clone(), format!("{:.1}%", v * 100.0))).collect::<std::collections::HashMap<String, String>>(),
        "top_quartile_return": format!("{:.2}%", confusion.top_quartile_return * 100.0),
        "bottom_quartile_return": format!("{:.2}%", confusion.bottom_quartile_return * 100.0),
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "scope": scope_str,
            "window": {
                "from": from.to_string(),
                "to": to.to_string(),
            },
            "audit_1_candidate_coverage": coverage_json,
            "audit_2_trigger_attribution": trigger_json,
            "audit_3_confusion_against_returns": confusion_json,
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// AuditPersistenceSensitivity
// ------------------------------------------------------------------

pub fn handle_audit_persistence_sensitivity(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };

    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let indicators = indicator_engine::build_indicator_snapshots(&bars);

    let report = regime_audit::sensitivity_replay::run_persistence_sensitivity_audit(
        &bars,
        &indicators,
        scope_str,
        anchor_symbol,
    );

    let matrix_json: Vec<serde_json::Value> = report
        .comparison
        .iter()
        .map(|r| {
            serde_json::json!({
                "confirmation_days": r.confirmation_days,
                "min_days": r.min_days,
                "separation_score": r.separation_score,
                "gates_passed": r.gates_passed,
                "riskon_return_60d": format!("{:.2}%", r.riskon_return_60d * 100.0),
                "neutral_return_60d": format!("{:.2}%", r.neutral_return_60d * 100.0),
                "riskoff_return_60d": format!("{:.2}%", r.riskoff_return_60d * 100.0),
                "churn_rate": format!("{:.2}%", r.churn_rate * 100.0),
                "imbalance_ratio": format!("{:.2}x", r.imbalance_ratio),
                "riskon_pct": format!("{:.1}%", r.riskon_pct * 100.0),
                "neutral_pct": format!("{:.1}%", r.neutral_pct * 100.0),
                "riskoff_pct": format!("{:.1}%", r.riskoff_pct * 100.0),
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "scope": report.scope,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
            },
            "matrix": matrix_json,
            "recommendation": report.recommendation,
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// AuditMarketStructure
// ------------------------------------------------------------------

pub fn handle_audit_market_structure(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };

    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let report = regime_audit::market_structure::audit_market_structure(
        &bars,
        scope_str,
        anchor_symbol,
    );

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
                "total_months": report.total_months,
                "total_quarters": report.total_quarters,
            },
            "annualized_return": format!("{:.2}%", report.annualized_return * 100.0),
            "annualized_volatility": format!("{:.2}%", report.annualized_volatility * 100.0),
            "max_drawdown": format!("{:.2}%", report.max_drawdown * 100.0),
            "positive_month_ratio": format!("{:.1}%", report.positive_month_ratio * 100.0),
            "positive_quarter_ratio": format!("{:.1}%", report.positive_quarter_ratio * 100.0),
            "up_months": report.up_months,
            "down_months": report.down_months,
            "up_quarters": report.up_quarters,
            "down_quarters": report.down_quarters,
            "drawdown_profile": {
                "dd_over_10_pct": format!("{:.1}%", report.drawdown_profile.dd_over_10_pct * 100.0),
                "dd_over_20_pct": format!("{:.1}%", report.drawdown_profile.dd_over_20_pct * 100.0),
                "dd_over_30_pct": format!("{:.1}%", report.drawdown_profile.dd_over_30_pct * 100.0),
                "avg_drawdown": format!("{:.2}%", report.drawdown_profile.avg_drawdown * 100.0),
                "max_drawdown_date": report.drawdown_profile.max_drawdown_date.to_string(),
            },
        }))?
    );
    Ok(())
}

// ------------------------------------------------------------------
// AuditRegimeAlignment
// ------------------------------------------------------------------

pub fn handle_audit_regime_alignment(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
    tolerance_days: usize,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };

    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let score = regime_audit::state_alignment::compute_state_alignment(
        &regimes_in_window,
        &bars,
        tolerance_days,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute state alignment"))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": if score.overall_passed { "passed" } else { "failed" },
            "scope": score.scope,
            "anchor_symbol": anchor_symbol,
            "window": {
                "from": score.window_from.to_string(),
                "to": score.window_to.to_string(),
                "total_days": score.total_days,
            },
            "drawdown_alignment": {
                "dd10": {
                    "precision": format!("{:.2}", score.drawdown_alignment.dd10_precision),
                    "recall": format!("{:.2}", score.drawdown_alignment.dd10_recall),
                    "f1": format!("{:.2}", score.drawdown_alignment.dd10_f1),
                },
                "dd20": {
                    "precision": format!("{:.2}", score.drawdown_alignment.dd20_precision),
                    "recall": format!("{:.2}", score.drawdown_alignment.dd20_recall),
                    "f1": format!("{:.2}", score.drawdown_alignment.dd20_f1),
                },
                "dd30": {
                    "precision": format!("{:.2}", score.drawdown_alignment.dd30_precision),
                    "recall": format!("{:.2}", score.drawdown_alignment.dd30_recall),
                    "f1": format!("{:.2}", score.drawdown_alignment.dd30_f1),
                },
            },
            "trend_alignment": {
                "riskon_precision": format!("{:.2}", score.trend_alignment.riskon_precision),
                "riskon_recall": format!("{:.2}", score.trend_alignment.riskon_recall),
                "riskon_f1": format!("{:.2}", score.trend_alignment.riskon_f1),
            },
            "change_detection": {
                "precision": format!("{:.2}", score.change_detection.precision),
                "recall": format!("{:.2}", score.change_detection.recall),
                "avg_latency_days": format!("{:.1}", score.change_detection.avg_latency_days),
            },
            "information_score": {
                "entropy": format!("{:.3}", score.information_score.entropy),
                "normalized_entropy": format!("{:.3}", score.information_score.normalized_entropy),
                "effective_states": format!("{:.2}", score.information_score.effective_states),
            },
            "overall": {
                "alignment": format!("{:.3}", score.overall_alignment),
                "information": format!("{:.3}", score.overall_information),
                "passed": score.overall_passed,
            },
            "gate_criteria": {
                "alignment_threshold": 0.75,
                "information_threshold": 0.60,
            },
        }))?);

    if !score.overall_passed {
        std::process::exit(1);
    }
    Ok(())
}

// ------------------------------------------------------------------
// AuditFactorAlignment
// ------------------------------------------------------------------

pub fn handle_audit_factor_alignment(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::factor_alignment::compute_factor_alignment(&regimes_in_window, &bars)
        .ok_or_else(|| anyhow::anyhow!("Failed to compute factor alignment"))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "trend": {
                "dd10_f1": format!("{:.2}", report.trend_alignment.dd10_f1),
                "dd20_f1": format!("{:.2}", report.trend_alignment.dd20_f1),
                "dd30_f1": format!("{:.2}", report.trend_alignment.dd30_f1),
                "uptrend_f1": format!("{:.2}", report.trend_alignment.uptrend_f1),
                "information": format!("{:.3}", report.trend_alignment.information_score.normalized_entropy),
            },
            "risk": {
                "dd10_f1": format!("{:.2}", report.risk_alignment.dd10_f1),
                "dd20_f1": format!("{:.2}", report.risk_alignment.dd20_f1),
                "dd30_f1": format!("{:.2}", report.risk_alignment.dd30_f1),
                "uptrend_f1": format!("{:.2}", report.risk_alignment.uptrend_f1),
                "information": format!("{:.3}", report.risk_alignment.information_score.normalized_entropy),
            },
            "liquidity": {
                "dd10_f1": format!("{:.2}", report.liquidity_alignment.dd10_f1),
                "dd20_f1": format!("{:.2}", report.liquidity_alignment.dd20_f1),
                "dd30_f1": format!("{:.2}", report.liquidity_alignment.dd30_f1),
                "uptrend_f1": format!("{:.2}", report.liquidity_alignment.uptrend_f1),
                "information": format!("{:.3}", report.liquidity_alignment.information_score.normalized_entropy),
            },
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditFalsePositiveBreakdown
// ------------------------------------------------------------------

pub fn handle_audit_false_positive_breakdown(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let fp = regime_audit::factor_alignment::compute_false_positive_breakdown(&regimes_in_window, &bars)
        .ok_or_else(|| anyhow::anyhow!("Failed to compute false positive breakdown"))?;
    let fn_ = regime_audit::factor_alignment::compute_false_negative_breakdown(&regimes_in_window, &bars)
        .ok_or_else(|| anyhow::anyhow!("Failed to compute false negative breakdown"))?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": scope_str,
            "anchor_symbol": anchor_symbol,
            "window": { "from": from.to_string(), "to": to.to_string() },
            "false_positive": {
                "total_riskoff_days": fp.total_riskoff_days,
                "false_positive_days": fp.false_positive_days,
                "fp_rate": format!("{:.1}%", if fp.total_riskoff_days > 0 { fp.false_positive_days as f64 / fp.total_riskoff_days as f64 * 100.0 } else { 0.0 }),
                "caused_by_trend_only": fp.caused_by_trend_only,
                "caused_by_risk_only": fp.caused_by_risk_only,
                "caused_by_both": fp.caused_by_both,
            },
            "false_negative": {
                "total_dd20_days": fn_.total_dd20_days,
                "missed_by_trend": fn_.missed_by_trend,
                "missed_by_risk": fn_.missed_by_risk,
                "missed_by_liquidity": fn_.missed_by_liquidity,
                "missed_by_all": fn_.missed_by_all,
                "fn_rate": format!("{:.1}%", if fn_.total_dd20_days > 0 { (fn_.missed_by_trend + fn_.missed_by_risk + fn_.missed_by_liquidity + fn_.missed_by_all) as f64 / fn_.total_dd20_days as f64 * 100.0 } else { 0.0 }),
            },
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditCounterfactualRegime
// ------------------------------------------------------------------

pub fn handle_audit_counterfactual_regime(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::factor_alignment::compute_counterfactual_replay(&regimes_in_window, &bars)
        .ok_or_else(|| anyhow::anyhow!("Failed to compute counterfactual replay"))?;

    let variants_json: Vec<serde_json::Value> = report.variants.iter().map(|v| {
        serde_json::json!({
            "name": v.name,
            "regime_distribution": v.regime_distribution,
            "alignment": format!("{:.3}", v.alignment),
            "information": format!("{:.3}", v.information),
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "variants": variants_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditEconomicReplay
// ------------------------------------------------------------------

pub fn handle_audit_economic_replay(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::economic_replay::compute_economic_replay(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute economic replay"))?;

    let variants_json: Vec<serde_json::Value> = report.variants.iter().map(|v| {
        let stats_json: serde_json::Map<String, serde_json::Value> = v.economic_stats.iter().map(|(k, s)| {
            (k.clone(), serde_json::json!({
                "count": s.count,
                "pct": format!("{:.1}%", s.pct * 100.0),
                "fwd_ret_20d_mean": format!("{:.2}%", s.forward_return_20d_mean * 100.0),
                "fwd_ret_60d_mean": format!("{:.2}%", s.forward_return_60d_mean * 100.0),
                "max_dd_median": format!("{:.2}%", s.max_drawdown_median * 100.0),
                "vol_median": format!("{:.2}%", s.volatility_median * 100.0),
                "sharpe": format!("{:.2}", s.sharpe_median),
                "win_rate_20d": format!("{:.1}%", s.win_rate_20d * 100.0),
                "win_rate_60d": format!("{:.1}%", s.win_rate_60d * 100.0),
            }))
        }).collect();

        serde_json::json!({
            "name": v.name,
            "regime_distribution": v.regime_distribution,
            "alignment": format!("{:.3}", v.alignment),
            "information": format!("{:.3}", v.information),
            "economic_stats": stats_json,
            "separation_score": {
                "overall": format!("{:.1}", v.separation_score.overall_score),
                "gates": v.separation_score.gate_results,
            },
            "assessment": v.assessment,
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "variants": variants_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditEconomicAttribution
// ------------------------------------------------------------------

pub fn handle_audit_economic_attribution(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::economic_attribution::compute_economic_attribution(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute economic attribution"))?;

    let factors_json: Vec<serde_json::Value> = report.factor_attributions.iter().map(|f| {
        let per_regime: serde_json::Map<String, serde_json::Value> = f.per_regime_corr.iter().map(|(k, v)| {
            (k.clone(), serde_json::json!(format!("{:.3}", v)))
        }).collect();

        serde_json::json!({
            "factor": f.factor_name,
            "pearson_20d": format!("{:.3}", f.pearson_corr_20d),
            "pearson_60d": format!("{:.3}", f.pearson_corr_60d),
            "spearman_20d": format!("{:.3}", f.spearman_corr_20d),
            "spearman_60d": format!("{:.3}", f.spearman_corr_60d),
            "mutual_info_20d": format!("{:.3}", f.mutual_information_20d),
            "mutual_info_60d": format!("{:.3}", f.mutual_information_60d),
            "per_regime_corr": per_regime,
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "dominant_factor": report.dominant_factor,
            "economic_vs_alignment_divergence": report.economic_vs_alignment_divergence,
            "factors": factors_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditParetoFrontier
// ------------------------------------------------------------------

pub fn handle_audit_pareto_frontier(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::pareto_frontier::compute_pareto_frontier(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute pareto frontier"))?;

    let points_json: Vec<serde_json::Value> = report.points.iter().map(|p| {
        serde_json::json!({
            "variant": p.variant,
            "alignment": format!("{:.3}", p.alignment),
            "separation": format!("{:.1}", p.separation_score),
            "information": format!("{:.3}", p.information),
            "pareto_optimal": p.is_pareto_optimal,
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "correlation": format!("{:.3}", report.correlation),
            "trade_off_detected": report.trade_off_detected,
            "pareto_optimal_variants": report.pareto_optimal_variants,
            "points": points_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditEconomicRegimePrototype
// ------------------------------------------------------------------

pub fn handle_audit_economic_regime_prototype(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::economic_regime_prototype::compute_economic_regime_prototype(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute economic regime prototype"))?;

    let distribution_json: serde_json::Map<String, serde_json::Value> = report.state_distribution.iter().map(|(k, v)| {
        (k.clone(), serde_json::json!(format!("{:.1}%", v * 100.0)))
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "state_distribution": distribution_json,
            "economic_separation": format!("{:.1}", report.economic_separation),
            "validation_status": report.validation_status,
            "note": "TASK-029 Prototype: Independent Economic Layer (Favorable/Neutral/Unfavorable)",
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditDualLayerValidation
// ------------------------------------------------------------------

pub fn handle_audit_dual_layer_validation(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::dual_layer_validation::compute_dual_layer_validation(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute dual layer validation"))?;

    let matrix_json: Vec<serde_json::Value> = report.cross_matrix.iter().map(|c| {
        serde_json::json!({
            "state_regime": c.state_regime,
            "economic_regime": c.economic_regime,
            "count": c.count,
            "pct": format!("{:.1}%", c.pct * 100.0),
            "fwd_ret_20d_mean": format!("{:.2}%", c.fwd_ret_20d_mean * 100.0),
            "fwd_ret_60d_mean": format!("{:.2}%", c.fwd_ret_60d_mean * 100.0),
            "sharpe": format!("{:.2}", c.sharpe),
            "max_dd_median": format!("{:.2}%", c.max_dd_median * 100.0),
            "win_rate": format!("{:.1}%", c.win_rate * 100.0),
        })
    }).collect();

    let stability_json: Vec<serde_json::Value> = report.stability_results.iter().map(|s| {
        serde_json::json!({
            "window": s.window_label,
            "from": s.window_from.to_string(),
            "to": s.window_to.to_string(),
            "economic_separation": format!("{:.1}", s.economic_separation),
            "cramer_v": format!("{:.3}", s.cramer_v),
            "mutual_information": format!("{:.3}", s.mutual_information),
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "mutual_information": format!("{:.3}", report.mutual_information),
            "cramer_v": format!("{:.3}", report.cramer_v),
            "orthogonality_pass": report.orthogonality_pass,
            "cross_matrix": matrix_json,
            "stability": stability_json,
            "validation_status": report.validation_status,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditAllocationPrototype
// ------------------------------------------------------------------

pub fn handle_audit_allocation_prototype(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::allocation_prototype::compute_allocation_prototype(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute allocation prototype"))?;

    let strategies_json: Vec<serde_json::Value> = report.strategies.iter().map(|s| {
        serde_json::json!({
            "strategy": s.strategy,
            "cagr": format!("{:.2}%", s.cagr * 100.0),
            "sharpe": format!("{:.2}", s.sharpe),
            "sortino": format!("{:.2}", s.sortino),
            "max_drawdown": format!("{:.2}%", s.max_drawdown * 100.0),
            "turnover": format!("{:.2}%", s.turnover * 100.0),
            "final_value": format!("{:.3}", s.final_value),
            "total_return": format!("{:.2}%", s.total_return * 100.0),
            "avg_position": format!("{:.1}%", s.avg_position * 100.0),
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "strategies": strategies_json,
            "dual_better_than_baseline": report.dual_better_than_baseline,
            "dual_better_than_state": report.dual_better_than_state,
            "dual_better_than_economic": report.dual_better_than_economic,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditStateSignalDecomposition
// ------------------------------------------------------------------

pub fn handle_audit_state_signal_decomposition(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::state_signal_decomposition::compute_state_signal_decomposition(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute state signal decomposition"))?;

    let attribution_json: Vec<serde_json::Value> = report.state_attributions.iter().map(|a| {
        serde_json::json!({
            "state": a.state,
            "count": a.count,
            "pct": format!("{:.1}%", a.pct * 100.0),
            "total_return_contribution": format!("{:.2}%", a.total_return_contribution * 100.0),
            "avg_daily_return": format!("{:.3}%", a.avg_daily_return * 100.0),
            "avg_20d_return": format!("{:.2}%", a.avg_20d_return * 100.0),
            "avg_60d_return": format!("{:.2}%", a.avg_60d_return * 100.0),
            "win_rate": format!("{:.1}%", a.win_rate * 100.0),
            "sharpe": format!("{:.2}", a.sharpe),
        })
    }).collect();

    let persistence_json: Vec<serde_json::Value> = report.persistence_comparison.iter().map(|p| {
        serde_json::json!({
            "confirmation_days": p.confirmation_days,
            "cagr": format!("{:.2}%", p.cagr * 100.0),
            "sharpe": format!("{:.2}", p.sharpe),
            "max_drawdown": format!("{:.2}%", p.max_drawdown * 100.0),
            "turnover": format!("{:.2}%", p.turnover * 100.0),
            "final_value": format!("{:.3}", p.final_value),
        })
    }).collect();

    let transition_json: Vec<serde_json::Value> = report.transition_alphas.iter().map(|t| {
        serde_json::json!({
            "transition": format!("{} -> {}", t.from_state, t.to_state),
            "count": t.count,
            "avg_20d_return": format!("{:.2}%", t.avg_20d_return * 100.0),
            "avg_60d_return": format!("{:.2}%", t.avg_60d_return * 100.0),
            "win_rate_20d": format!("{:.1}%", t.win_rate_20d * 100.0),
            "win_rate_60d": format!("{:.1}%", t.win_rate_60d * 100.0),
            "max_dd_median": format!("{:.2}%", t.max_dd_median * 100.0),
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "conclusion": report.conclusion,
            "state_attributions": attribution_json,
            "persistence_comparison": persistence_json,
            "transition_alphas": transition_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditStateTransitions
// ------------------------------------------------------------------

pub fn handle_audit_state_transitions(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let environments = market_store::fetch_all_environment_snapshots(&context.storage)?;
    let envs_in_window: Vec<_> = environments
        .into_iter()
        .filter(|e| e.scope.eq_ignore_ascii_case(scope_str) && e.date >= from && e.date <= to)
        .collect();
    if envs_in_window.is_empty() {
        anyhow::bail!("No environment snapshot rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let mut attributions = Vec::new();
    for regime in &regimes_in_window {
        if let Some(env) = envs_in_window.iter().find(|e| e.date == regime.date) {
            let detailed = macro_engine::build_strategy_state_detailed(regime, env);
            attributions.push(detailed);
        }
    }

    let mut branch_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut trigger_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut cliff_events = Vec::new();
    let mut state_score_by_state: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();

    for attr in &attributions {
        *branch_counts.entry(attr.branch_taken.clone()).or_insert(0) += 1;
        for trigger in &attr.trigger_conditions {
            *trigger_counts.entry(format!("{}: {}", attr.branch_taken, trigger)).or_insert(0) += 1;
        }

        state_score_by_state.entry(format!("{:?}", attr.state)).or_default().push(attr.state_score);

        if attr.branch_taken == "confirm_add" || attr.branch_taken == "derisk" {
            let trend = attr.trend;
            if (50.0..65.0).contains(&trend) {
                cliff_events.push(serde_json::json!({
                    "date": attr.date.to_string(),
                    "trend": trend,
                    "state": format!("{:?}", attr.state),
                    "position_pct": attr.position_pct,
                    "breadth": attr.breadth,
                    "env_score": attr.env_score,
                    "risk": attr.risk,
                    "stress": attr.stress,
                }));
            }
        }
    }

    let total = attributions.len();
    let mut branch_distribution: Vec<serde_json::Value> = branch_counts.iter().map(|(branch, count)| {
        serde_json::json!({
            "branch": branch,
            "count": count,
            "pct": format!("{:.1}%", *count as f64 / total as f64 * 100.0),
        })
    }).collect();
    branch_distribution.sort_by(|a, b| {
        let a_count = a["count"].as_u64().unwrap_or(0);
        let b_count = b["count"].as_u64().unwrap_or(0);
        b_count.cmp(&a_count)
    });

    let mut trigger_distribution: Vec<serde_json::Value> = trigger_counts.iter().map(|(trigger, count)| {
        serde_json::json!({
            "trigger": trigger,
            "count": count,
            "pct": format!("{:.1}%", *count as f64 / total as f64 * 100.0),
        })
    }).collect();
    trigger_distribution.sort_by(|a, b| {
        let a_count = a["count"].as_u64().unwrap_or(0);
        let b_count = b["count"].as_u64().unwrap_or(0);
        b_count.cmp(&a_count)
    });

    let state_score_summary: Vec<serde_json::Value> = state_score_by_state.iter().map(|(state, scores)| {
        let avg = scores.iter().sum::<f64>() / scores.len() as f64;
        let min = scores.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = scores.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        serde_json::json!({
            "state": state,
            "count": scores.len(),
            "avg_state_score": format!("{:.1}", avg),
            "min_state_score": format!("{:.1}", min),
            "max_state_score": format!("{:.1}", max),
        })
    }).collect();

    let report_json = serde_json::json!({
        "scope": scope_str,
        "from": from.to_string(),
        "to": to.to_string(),
        "total_days": total,
        "branch_distribution": branch_distribution,
        "trigger_distribution": trigger_distribution,
        "cliff_events": cliff_events,
        "state_score_summary": state_score_summary,
        "attributions": attributions.iter().map(|a| {
            serde_json::json!({
                "date": a.date.to_string(),
                "branch": a.branch_taken,
                "state": format!("{:?}", a.state),
                "position_pct": a.position_pct,
                "trend": a.trend,
                "risk": a.risk,
                "liquidity": a.liquidity,
                "breadth": a.breadth,
                "env_score": a.env_score,
                "stress": a.stress,
                "state_score": a.state_score,
                "triggers": a.trigger_conditions,
            })
        }).collect::<Vec<_>>(),
    });

    println!("{}", serde_json::to_string_pretty(&report_json)?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditPersistenceFrontier
// ------------------------------------------------------------------

pub fn handle_audit_persistence_frontier(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::persistence_frontier::compute_persistence_frontier(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute persistence frontier"))?;

    let points_json: Vec<serde_json::Value> = report.points.iter().map(|p| {
        serde_json::json!({
            "confirmation_days": p.confirmation_days,
            "alignment": format!("{:.3}", p.alignment),
            "information": format!("{:.3}", p.information),
            "cagr": format!("{:.2}%", p.cagr * 100.0),
            "sharpe": format!("{:.2}", p.sharpe),
            "sortino": format!("{:.2}", p.sortino),
            "max_drawdown": format!("{:.2}%", p.max_drawdown * 100.0),
            "turnover": format!("{:.2}%", p.turnover * 100.0),
            "final_value": format!("{:.3}", p.final_value),
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "optimal_days": report.optimal_days,
            "conclusion": report.conclusion,
            "points": points_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditPersistenceMechanics
// ------------------------------------------------------------------

pub fn handle_audit_persistence_mechanics(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::persistence_mechanics::compute_persistence_mechanics(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute persistence mechanics"))?;

    let points_json: Vec<serde_json::Value> = report.points.iter().map(|p| {
        let episodes_json: Vec<serde_json::Value> = p.episodes.iter().map(|e| {
            serde_json::json!({
                "regime": e.regime,
                "start_date": e.start_date.to_string(),
                "end_date": e.end_date.to_string(),
                "duration_days": e.duration_days,
                "confirmed_at_day": e.confirmed_at_day,
                "delayed_days": e.delayed_days,
                "swallowed": e.swallowed,
            })
        }).collect();
        serde_json::json!({
            "confirmation_days": p.confirmation_days,
            "distribution": {
                "risk_on_days": p.distribution.risk_on_days,
                "neutral_days": p.distribution.neutral_days,
                "risk_off_days": p.distribution.risk_off_days,
                "total_days": p.distribution.total_days,
            },
            "single_day_flips": p.single_day_flips,
            "total_transitions": p.total_transitions,
            "avg_delay_days": format!("{:.1}", p.avg_delay_days),
            "swallowed_regimes": p.swallowed_regimes,
            "merged_regimes": p.merged_regimes,
            "episodes": episodes_json,
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "q1_single_day_flip_count": report.q1_single_day_flip_count,
            "q2_state_distribution_comparison": report.q2_state_distribution_comparison,
            "q3_delayed_confirmation_analysis": report.q3_delayed_confirmation_analysis,
            "conclusion": report.conclusion,
            "points": points_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditEpisodeSurvival
// ------------------------------------------------------------------

pub fn handle_audit_episode_survival(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::episode_survival::compute_episode_survival(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute episode survival"))?;

    let buckets_json: Vec<serde_json::Value> = report.buckets.iter().map(|b| {
        serde_json::json!({
            "bucket": b.bucket_label,
            "count": b.count,
            "percentage": format!("{:.1}%", b.percentage),
        })
    }).collect();

    let survival_json: Vec<serde_json::Value> = report.survival_curve.iter().map(|s| {
        serde_json::json!({
            "confirmation_days": s.confirmation_days,
            "survival_rate": format!("{:.1}%", s.survival_rate),
            "swallowed": s.swallowed_count,
            "survived": s.survived_count,
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "episode_stats": {
                "total_episodes": report.total_episodes,
                "avg_days": format!("{:.1}", report.avg_episode_days),
                "median_days": format!("{:.1}", report.median_episode_days),
                "p25_days": format!("{:.1}", report.p25_episode_days),
                "p75_days": format!("{:.1}", report.p75_episode_days),
                "p95_days": format!("{:.1}", report.p95_episode_days),
            },
            "buckets": buckets_json,
            "survival_curve": survival_json,
            "recommendation": report.recommendation,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditLabelDistribution
// ------------------------------------------------------------------

pub fn handle_audit_label_distribution(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::label_distribution::compute_label_distribution(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute label distribution"))?;

    let points_json: Vec<serde_json::Value> = report.points.iter().map(|p| {
        serde_json::json!({
            "persistence_days": p.persistence_days,
            "risk_on_pct": format!("{:.1}%", p.risk_on_pct),
            "neutral_pct": format!("{:.1}%", p.neutral_pct),
            "risk_off_pct": format!("{:.1}%", p.risk_off_pct),
            "effective_states": p.effective_states,
            "information_score": format!("{:.3}", p.information_score),
            "episode_count": p.episode_count,
            "median_episode_days": format!("{:.1}", p.median_episode_days),
            "avg_episode_days": format!("{:.1}", p.avg_episode_days),
            "alignment_score": format!("{:.3}", p.alignment_score),
            "transition_count": p.transition_count,
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "conclusion": report.conclusion,
            "points": points_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditScoreDistribution
// ------------------------------------------------------------------

pub fn handle_audit_score_distribution(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::score_distribution::compute_score_distribution(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute score distribution"))?;

    let dist_json = |d: &core_domain::ScoreDistribution| serde_json::json!({
        "metric": d.metric,
        "mean": format!("{:.1}", d.mean),
        "median": format!("{:.1}", d.median),
        "std": format!("{:.1}", d.std),
        "min": format!("{:.1}", d.min),
        "max": format!("{:.1}", d.max),
        "buckets": d.buckets.iter().map(|b| serde_json::json!({
            "range": b.range,
            "count": b.count,
            "percentage": format!("{:.1}%", b.percentage),
        })).collect::<Vec<_>>(),
    });

    let hits_json: Vec<serde_json::Value> = report.threshold_hits.iter().map(|h| {
        serde_json::json!({
            "condition": h.condition,
            "days_met": h.days_met,
            "percentage": format!("{:.1}%", h.percentage),
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "trend_distribution": dist_json(&report.trend_distribution),
            "risk_distribution": dist_json(&report.risk_distribution),
            "liquidity_distribution": dist_json(&report.liquidity_distribution),
            "threshold_hits": hits_json,
            "conclusion": report.conclusion,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditWave8Revalidation
// ------------------------------------------------------------------

pub fn handle_audit_wave8_revalidation(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::wave8_revalidation::compute_wave8_revalidation(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute wave8 revalidation"))?;

    let comparisons_json: Vec<serde_json::Value> = report.comparisons.iter().map(|p| {
        serde_json::json!({
            "persistence_days": p.persistence_days,
            "alignment_score": format!("{:.3}", p.alignment_score),
            "information_score": format!("{:.3}", p.information_score),
            "economic_separation": format!("{:.1}", p.economic_separation),
            "state_only_cagr": format!("{:.2}%", p.state_only_cagr * 100.0),
            "state_only_sharpe": format!("{:.2}", p.state_only_sharpe),
            "dual_layer_cagr": format!("{:.2}%", p.dual_layer_cagr * 100.0),
            "dual_layer_sharpe": format!("{:.2}", p.dual_layer_sharpe),
            "baseline_cagr": format!("{:.2}%", p.baseline_cagr * 100.0),
            "baseline_sharpe": format!("{:.2}", p.baseline_sharpe),
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "conclusion": report.conclusion,
            "comparisons": comparisons_json,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditGroundTruth
// ------------------------------------------------------------------

pub fn handle_audit_ground_truth(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScopeArg,
    persistence_days: usize,
) -> Result<()> {
    let scope_enum: ReportScope = scope.into();
    let scope_str = match scope_enum {
        ReportScope::Global => "GLOBAL",
        ReportScope::Cn => "CN",
        ReportScope::Hk => "HK",
    };
    let anchor_symbol = match scope_str {
        "CN" => "000300",
        "HK" => "HSCEI",
        _ => "000300",
    };

    let bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &[anchor_symbol.to_string()],
        from,
        to,
    )?;
    if bars.is_empty() {
        anyhow::bail!("No daily bars found for {} in range {} to {}", anchor_symbol, from, to);
    }

    let regimes = market_store::fetch_market_regimes(&context.storage)?;
    let regimes_in_window: Vec<_> = regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case(scope_str) && r.date >= from && r.date <= to)
        .collect();
    if regimes_in_window.is_empty() {
        anyhow::bail!("No market regime rows found for scope {} in range {} to {}", scope_str, from, to);
    }

    let report = regime_audit::ground_truth_audit::compute_ground_truth_audit(
        &regimes_in_window,
        &bars,
        scope_str,
        anchor_symbol,
        persistence_days,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to compute ground truth audit"))?;

    let pred_dist_json: Vec<serde_json::Value> = report.predicted_distribution.iter().map(|d| {
        serde_json::json!({
            "label": d.label,
            "count": d.count,
            "percentage": format!("{:.1}%", d.percentage),
        })
    }).collect();

    let actual_dist_json: Vec<serde_json::Value> = report.actual_distribution.iter().map(|d| {
        serde_json::json!({
            "label": d.label,
            "count": d.count,
            "percentage": format!("{:.1}%", d.percentage),
        })
    }).collect();

    let confusion_json: Vec<serde_json::Value> = report.confusion_matrix.iter().map(|c| {
        serde_json::json!({
            "predicted": c.predicted,
            "actual": c.actual,
            "count": c.count,
            "percentage": format!("{:.1}%", c.percentage),
        })
    }).collect();

    let metrics_json: Vec<serde_json::Value> = report.class_metrics.iter().map(|m| {
        serde_json::json!({
            "class": m.class,
            "precision": format!("{:.3}", m.precision),
            "recall": format!("{:.3}", m.recall),
            "f1": format!("{:.3}", m.f1),
            "support": m.support,
        })
    }).collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "scope": report.scope,
            "anchor_symbol": report.anchor_symbol,
            "persistence_days": persistence_days,
            "window": {
                "from": report.window_from.to_string(),
                "to": report.window_to.to_string(),
                "total_days": report.total_days,
            },
            "predicted_distribution": pred_dist_json,
            "actual_distribution": actual_dist_json,
            "confusion_matrix": confusion_json,
            "class_metrics": metrics_json,
            "overall_accuracy": format!("{:.1}%", report.overall_accuracy * 100.0),
            "macro_f1": format!("{:.3}", report.macro_f1),
            "conclusion": report.conclusion,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditForwardReturnDistribution
// ------------------------------------------------------------------

pub fn handle_audit_forward_return_distribution(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<()> {
    let cn_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["000300".to_string()],
        from,
        to,
    )?;
    if cn_bars.is_empty() {
        anyhow::bail!("No daily bars found for 000300 in range {} to {}", from, to);
    }

    let hk_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["HSCEI".to_string()],
        from,
        to,
    )?;
    if hk_bars.is_empty() {
        anyhow::bail!("No daily bars found for HSCEI in range {} to {}", from, to);
    }

    let report = regime_audit::forward_return_distribution::audit_forward_return_distribution(
        &cn_bars,
        &hk_bars,
    );

    let format_dist = |d: &regime_audit::forward_return_distribution::ForwardReturnDistribution| {
        serde_json::json!({
            "market": d.market,
            "horizon_days": d.horizon_days,
            "sample_count": d.sample_count,
            "percentiles": {
                "p01": format!("{:.4}", d.p01),
                "p05": format!("{:.4}", d.p05),
                "p10": format!("{:.4}", d.p10),
                "p25": format!("{:.4}", d.p25),
                "p50": format!("{:.4}", d.p50),
                "p75": format!("{:.4}", d.p75),
                "p90": format!("{:.4}", d.p90),
                "p95": format!("{:.4}", d.p95),
            },
            "mean": format!("{:.4}", d.mean),
            "std": format!("{:.4}", d.std),
            "min": format!("{:.4}", d.min),
            "max": format!("{:.4}", d.max),
        })
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "task": "TASK-060A.1",
            "description": "Forward Return Distribution Audit",
            "cn": report.cn_distributions.iter().map(format_dist).collect::<Vec<_>>(),
            "hk": report.hk_distributions.iter().map(format_dist).collect::<Vec<_>>(),
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// GenerateGroundTruthLabels
// ------------------------------------------------------------------

pub fn handle_generate_ground_truth_labels(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    horizon: usize,
) -> Result<()> {
    let cn_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["000300".to_string()],
        from,
        to,
    )?;
    if cn_bars.is_empty() {
        anyhow::bail!("No daily bars found for 000300 in range {} to {}", from, to);
    }

    let hk_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["HSCEI".to_string()],
        from,
        to,
    )?;
    if hk_bars.is_empty() {
        anyhow::bail!("No daily bars found for HSCEI in range {} to {}", from, to);
    }

    let report = regime_audit::ground_truth_generator::generate_ground_truth_labels(
        &cn_bars,
        &hk_bars,
        horizon,
    );

    let format_set = |set: &regime_audit::ground_truth_generator::GroundTruthSet| {
        let (risk_off_count, neutral_count, risk_on_count) =
            regime_audit::ground_truth_generator::compute_label_distribution(&set.labels);
        let (risk_off_mean, neutral_mean, risk_on_mean) =
            regime_audit::ground_truth_generator::compute_mean_return_by_label(&set.labels);
        let total = set.labels.len() as f64;

        serde_json::json!({
            "market": set.market,
            "scheme": set.scheme.name,
            "horizon_days": set.horizon_days,
            "thresholds": {
                "risk_off_pct": format!("{:.0}%", set.scheme.risk_off_pct * 100.0),
                "risk_on_pct": format!("{:.0}%", (1.0 - set.scheme.risk_on_pct) * 100.0),
            },
            "distribution": {
                "risk_off": {
                    "count": risk_off_count,
                    "percentage": format!("{:.1}%", risk_off_count as f64 / total * 100.0),
                    "mean_return": format!("{:.2}%", risk_off_mean * 100.0),
                },
                "neutral": {
                    "count": neutral_count,
                    "percentage": format!("{:.1}%", neutral_count as f64 / total * 100.0),
                    "mean_return": format!("{:.2}%", neutral_mean * 100.0),
                },
                "risk_on": {
                    "count": risk_on_count,
                    "percentage": format!("{:.1}%", risk_on_count as f64 / total * 100.0),
                    "mean_return": format!("{:.2}%", risk_on_mean * 100.0),
                },
            },
            "total_samples": set.labels.len(),
        })
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "task": "TASK-060B",
            "description": "Generate Ground Truth Label Sets",
            "horizon_days": horizon,
            "cn": report.cn_sets.iter().map(format_set).collect::<Vec<_>>(),
            "hk": report.hk_sets.iter().map(format_set).collect::<Vec<_>>(),
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditAlignmentRedesign
// ------------------------------------------------------------------

pub fn handle_audit_alignment_redesign(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    horizon: usize,
) -> Result<()> {
    let cn_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["000300".to_string()],
        from,
        to,
    )?;
    let hk_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["HSCEI".to_string()],
        from,
        to,
    )?;

    let all_regimes = market_store::fetch_market_regimes(&context.storage)?;
    let cn_regimes: Vec<_> = all_regimes
        .clone()
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case("CN") && r.date >= from && r.date <= to)
        .collect();
    let hk_regimes: Vec<_> = all_regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case("HK") && r.date >= from && r.date <= to)
        .collect();

    let schemes = vec![
        ("GT-25".to_string(), 0.25, 0.75),
        ("GT-33".to_string(), 0.33, 0.67),
        ("GT-10".to_string(), 0.10, 0.90),
    ];

    let mut cn_reports = Vec::new();
    let mut hk_reports = Vec::new();

    if !cn_bars.is_empty() && !cn_regimes.is_empty() {
        cn_reports = regime_audit::alignment_redesign::compute_forward_return_alignment(
            "CN", &cn_regimes, &cn_bars, horizon, &schemes,
        );
    }
    if !hk_bars.is_empty() && !hk_regimes.is_empty() {
        hk_reports = regime_audit::alignment_redesign::compute_forward_return_alignment(
            "HK", &hk_regimes, &hk_bars, horizon, &schemes,
        );
    }

    let mut old_technical = Vec::new();
    if !cn_bars.is_empty() && !cn_regimes.is_empty() {
        old_technical.push(regime_audit::alignment_redesign::compute_technical_ground_truth_alignment(
            "CN", &cn_regimes, &cn_bars,
        ));
    }
    if !hk_bars.is_empty() && !hk_regimes.is_empty() {
        old_technical.push(regime_audit::alignment_redesign::compute_technical_ground_truth_alignment(
            "HK", &hk_regimes, &hk_bars,
        ));
    }

    let format_report = |r: &regime_audit::alignment_redesign::AlignmentReport| {
        let class_metrics: Vec<serde_json::Value> = r.class_metrics.iter().map(|m| {
            serde_json::json!({
                "class": m.class.clone(),
                "precision": format!("{:.3}", m.precision),
                "recall": format!("{:.3}", m.recall),
                "f1": format!("{:.3}", m.f1),
                "support": m.support,
            })
        }).collect();

        serde_json::json!({
            "market": r.market.clone(),
            "gt_scheme": r.gt_scheme.clone(),
            "total_samples": r.total_samples,
            "accuracy": format!("{:.3}", r.accuracy),
            "macro_precision": format!("{:.3}", r.macro_precision),
            "macro_recall": format!("{:.3}", r.macro_recall),
            "macro_f1": format!("{:.3}", r.macro_f1),
            "information_score": format!("{:.3}", r.information_score),
            "class_metrics": class_metrics,
        })
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "task": "TASK-060C",
            "description": "Alignment Redesign Comparison",
            "horizon_days": horizon,
            "old_technical_gt": old_technical.iter().map(format_report).collect::<Vec<_>>(),
            "forward_return_gt": {
                "cn": cn_reports.iter().map(format_report).collect::<Vec<_>>(),
                "hk": hk_reports.iter().map(format_report).collect::<Vec<_>>(),
            },
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// AuditStatePersistenceEconomics
// ------------------------------------------------------------------

pub fn handle_audit_state_persistence_economics(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<()> {
    let cn_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["000300".to_string()],
        from,
        to,
    )?;
    let hk_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["HSCEI".to_string()],
        from,
        to,
    )?;

    let all_regimes = market_store::fetch_market_regimes(&context.storage)?;
    let cn_regimes: Vec<_> = all_regimes
        .clone()
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case("CN") && r.date >= from && r.date <= to)
        .collect();
    let hk_regimes: Vec<_> = all_regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case("HK") && r.date >= from && r.date <= to)
        .collect();

    let (cn_report, hk_report) = regime_audit::state_persistence_economics::audit_state_persistence_economics(
        &cn_regimes, &cn_bars, &hk_regimes, &hk_bars,
    );

    let format_state = |s: &regime_audit::state_persistence_economics::StateEconomics| {
        serde_json::json!({
            "state": s.state.clone(),
            "sample_count": s.sample_count,
            "forward_return_20d": {
                "mean": format!("{:.2}%", s.fwd_return_20d_mean * 100.0),
                "std": format!("{:.2}%", s.fwd_return_20d_std * 100.0),
                "min": format!("{:.2}%", s.fwd_return_20d_min * 100.0),
                "max": format!("{:.2}%", s.fwd_return_20d_max * 100.0),
                "win_rate": format!("{:.1}%", s.fwd_return_20d_win_rate * 100.0),
            },
            "forward_return_60d": {
                "mean": format!("{:.2}%", s.fwd_return_60d_mean * 100.0),
                "std": format!("{:.2}%", s.fwd_return_60d_std * 100.0),
                "min": format!("{:.2}%", s.fwd_return_60d_min * 100.0),
                "max": format!("{:.2}%", s.fwd_return_60d_max * 100.0),
                "win_rate": format!("{:.1}%", s.fwd_return_60d_win_rate * 100.0),
            },
            "forward_return_120d": {
                "mean": format!("{:.2}%", s.fwd_return_120d_mean * 100.0),
                "std": format!("{:.2}%", s.fwd_return_120d_std * 100.0),
                "min": format!("{:.2}%", s.fwd_return_120d_min * 100.0),
                "max": format!("{:.2}%", s.fwd_return_120d_max * 100.0),
                "win_rate": format!("{:.1}%", s.fwd_return_120d_win_rate * 100.0),
            },
            "max_drawdown": {
                "mean": format!("{:.2}%", s.max_drawdown_mean * 100.0),
                "std": format!("{:.2}%", s.max_drawdown_std * 100.0),
                "min": format!("{:.2}%", s.max_drawdown_min * 100.0),
                "max": format!("{:.2}%", s.max_drawdown_max * 100.0),
            },
            "volatility": {
                "mean": format!("{:.2}%", s.volatility_mean * 100.0),
                "std": format!("{:.2}%", s.volatility_std * 100.0),
            },
        })
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "task": "TASK-070B",
            "description": "State Persistence Economics",
            "cn": cn_report.states.iter().map(format_state).collect::<Vec<_>>(),
            "hk": hk_report.states.iter().map(format_state).collect::<Vec<_>>(),
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// ValidateStateLayerGt
// ------------------------------------------------------------------

pub fn handle_validate_state_layer_gt(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<()> {
    let cn_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["000300".to_string()],
        from,
        to,
    )?;
    let hk_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["HSCEI".to_string()],
        from,
        to,
    )?;

    let all_regimes = market_store::fetch_market_regimes(&context.storage)?;
    let cn_regimes: Vec<_> = all_regimes
        .clone()
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case("CN") && r.date >= from && r.date <= to)
        .collect();
    let hk_regimes: Vec<_> = all_regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case("HK") && r.date >= from && r.date <= to)
        .collect();

    let macro_snapshots = market_store::fetch_macro_snapshots_in_range(&context.storage, from, to)?;

    let cn_comparison = if !cn_bars.is_empty() && !cn_regimes.is_empty() {
        Some(regime_audit::state_gt_validation::compute_state_layer_gt_alignment(
            "CN", &cn_regimes, &macro_snapshots, &cn_bars,
        ))
    } else {
        None
    };

    let hk_comparison = if !hk_bars.is_empty() && !hk_regimes.is_empty() {
        Some(regime_audit::state_gt_validation::compute_state_layer_gt_alignment(
            "HK", &hk_regimes, &macro_snapshots, &hk_bars,
        ))
    } else {
        None
    };

    let format_report = |r: &regime_audit::state_gt_validation::StateGtReport| {
        let class_metrics: Vec<serde_json::Value> = r.class_metrics.iter().map(|m| {
            serde_json::json!({
                "class": m.class.clone(),
                "precision": format!("{:.3}", m.precision),
                "recall": format!("{:.3}", m.recall),
                "f1": format!("{:.3}", m.f1),
                "support": m.support,
            })
        }).collect();

        serde_json::json!({
            "market": r.market.clone(),
            "total_samples": r.total_samples,
            "gt_distribution": {
                "risk_off": r.gt_distribution.0,
                "neutral": r.gt_distribution.1,
                "risk_on": r.gt_distribution.2,
            },
            "accuracy": format!("{:.3}", r.accuracy),
            "macro_precision": format!("{:.3}", r.macro_precision),
            "macro_recall": format!("{:.3}", r.macro_recall),
            "macro_f1": format!("{:.3}", r.macro_f1),
            "information_score": format!("{:.3}", r.information_score),
            "class_metrics": class_metrics,
        })
    };

    let mut outputs = Vec::new();

    if let Some(comp) = cn_comparison {
        if let Some(old) = &comp.old_technical {
            outputs.push(serde_json::json!({
                "market": "CN",
                "comparison": "Old Technical GT",
                "report": format_report(old),
            }));
        }
        outputs.push(serde_json::json!({
            "market": "CN",
            "comparison": "New State GT",
            "report": format_report(&comp.new_state_gt),
        }));
    }

    if let Some(comp) = hk_comparison {
        if let Some(old) = &comp.old_technical {
            outputs.push(serde_json::json!({
                "market": "HK",
                "comparison": "Old Technical GT",
                "report": format_report(old),
            }));
        }
        outputs.push(serde_json::json!({
            "market": "HK",
            "comparison": "New State GT",
            "report": format_report(&comp.new_state_gt),
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "task": "TASK-071A",
            "description": "State Layer Ground Truth Validation Demo",
            "results": outputs,
        }))?);
    Ok(())
}

// ------------------------------------------------------------------
// RotationRanking
// ------------------------------------------------------------------

pub fn handle_rotation_ranking(
    context: &AppContext,
    date: Option<NaiveDate>,
    scope: ReportScopeArg,
) -> Result<()> {
    use report_builder::{
        AuditReportBuilder, RotationRankingReportInput, RotationRankingInputRow,
    };
    use reporting::ReportingSnapshot;
    use reporting::Formatter;
    use report_renderer::{render, TextFormatter};

    let report_date = date.unwrap_or_else(|| {
        market_store::fetch_latest_table_date(&context.storage, "rotation_rank")
            .unwrap_or(None)
            .unwrap_or_else(|| Local::now().date_naive())
    });
    let mut rows = market_store::fetch_rotation_ranks_for_date(&context.storage, report_date)?;

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

    rows.sort_by(|a, b| b.momentum_score.total_cmp(&a.momentum_score).then(a.symbol.cmp(&b.symbol)));

    // Build instrument name lookup
    let instruments = context.seed_universe().unwrap_or_default();
    let name_map: HashMap<&str, &str> = instruments
        .iter()
        .map(|i| (i.symbol.as_str(), i.name.as_str()))
        .collect();

    let scope_analysis: ReportScope = scope.into();

    // Fetch signals for enrichment
    let signals = market_store::fetch_signal_snapshots_for_date_with_scope(
        &context.storage,
        report_date,
        scope_analysis,
    )?;
    let signal_map: HashMap<&str, &core_domain::SignalSnapshot> = signals
        .iter()
        .map(|s| (s.symbol.as_str(), s))
        .collect();

    // Build RotationRankingInputRows
    let detail_rows: Vec<RotationRankingInputRow> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let name = name_map
                .get(row.symbol.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| row.symbol.clone());
            let signal = signal_map.get(row.symbol.as_str());
            RotationRankingInputRow {
                rank: (i + 1) as i32,
                symbol: row.symbol.clone(),
                name,
                momentum_score: row.momentum_score,
                rs_120: row.rs_120,
                signal_label: signal
                    .map(|s| s.signal_label.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
                final_score: signal.map(|s| s.final_score).unwrap_or(0.0),
            }
        })
        .collect();

    // Build ReportingSnapshot from ResearchContext
    let research_ctx =
        context.build_research_context_for_date(report_date, scope_analysis)?;
    let snapshot = ReportingSnapshot {
        generated_at: chrono::Utc::now(),
        research: research_ctx,
    };

    let input = RotationRankingReportInput {
        rows: detail_rows,
    };

    let doc = AuditReportBuilder::build_rotation_ranking(&snapshot, &input)?;

    let mut fmt = TextFormatter::new();
    render(&mut fmt, &doc);
    println!("{}", fmt.finalize());
    println!(
        "Total: {} symbols | Research Surface (not in Shadow Production chain)",
        rows.len()
    );
    Ok(())
}

// ------------------------------------------------------------------
// SymbolDiagnostics
// ------------------------------------------------------------------

pub fn handle_symbol_diagnostics(
    context: &AppContext,
    symbol: String,
    date: Option<NaiveDate>,
    scope: ReportScopeArg,
) -> Result<()> {
    let report_date = date.unwrap_or_else(|| {
        market_store::fetch_latest_table_date(&context.storage, "signal_snapshot")
            .unwrap_or(None)
            .unwrap_or_else(|| Local::now().date_naive())
    });
    let scope: ReportScope = scope.into();

    let signal = market_store::fetch_signal_snapshot_for_symbol(&context.storage, report_date, &symbol, scope)?;
    let strategy_pref = market_store::fetch_strategy_preference_for_symbol(&context.storage, report_date, &symbol, scope)?;
    let rotations = market_store::fetch_rotation_ranks_for_date(&context.storage, report_date)?;
    let rotation = rotations.iter().find(|r| r.symbol == symbol);
    let state = market_store::fetch_strategy_states_for_scope(&context.storage, scope)?
        .into_iter().find(|s| s.date == report_date);

    println!("Symbol Diagnostics | {} | Date: {} | Scope: {}", symbol, report_date, scope.as_str());
    println!("{:=<80}", "");

    if let Some(ref signal) = signal {
        println!("Final Score: {:.2} | Label: {}", signal.final_score, signal.signal_label);
        println!("{:-<80}", "");

        println!("Attribution Breakdown");
        let reason = &signal.reason;
        println!("  Strategy       +{:>6.1} (45% x {:>6.1}  {:?})", reason.strategy_contribution, reason.strategy_score, reason.best_strategy);
        println!("  Alignment      +{:>6.1} (15% x {:>6.1}  {} aligned)", reason.alignment_contribution, reason.alignment as f64, reason.alignment);
        println!("  Regime         +{:>6.1} (20% x {:>6.1}  Trend={:.1} Risk={:.1})", reason.regime.contribution, reason.regime.combined_score, reason.regime.trend_score, reason.regime.risk_score);
        println!("  Rotation       +{:>6.1} (20% x {:>6.1}  Momentum={:.1})", reason.rotation.contribution, reason.rotation.combined_score, reason.rotation.momentum_score);
        println!("  {:-<76}", "");
        println!("  Total          {:>7.1}", signal.final_score);
        println!("{:-<80}", "");

        println!("Aligned Strategies: {:?}", reason.aligned_strategies);
        println!("{:-<80}", "");
    } else {
        println!("Signal: NOT FOUND");
    }

    if let Some(ref pref) = strategy_pref {
        println!("Strategy Raw Scores");
        let best = format!("{:?}", pref.best_strategy);
        println!("  ValueLeft:       {:>6.1}{}", pref.value_left_score, if best == "ValueLeft" { "  ← Best" } else { "" });
        println!("  TrendPullback:   {:>6.1}{}", pref.trend_pullback_score, if best == "TrendPullback" { "  ← Best" } else { "" });
        println!("  TrendBreakout:   {:>6.1}{}", pref.trend_breakout_score, if best == "TrendBreakout" { "  ← Best" } else { "" });
        println!("  MomentumRight:   {:>6.1}{}", pref.momentum_right_score, if best == "MomentumRight" { "  ← Best" } else { "" });
        println!("{:-<80}", "");
    }

    if let Some(ref rot) = rotation {
        println!("Rotation");
        println!("  Rank: #{} | Momentum: {:.2}", rot.rank, rot.momentum_score);
        println!("  RS20: {:.2} | RS60: {:.2} | RS120: {:.2}", rot.rs_20, rot.rs_60, rot.rs_120);
        println!("{:-<80}", "");
    }

    if let Some(ref state) = state {
        println!("Global State");
        println!("  State: {} | Position: {:.0}%", state.state, state.recommended_position_pct);
        if !state.transition_reason.is_empty() {
            println!("  Reason: {}", state.transition_reason);
        }
        println!("{:-<80}", "");
    }

    println!("Explainability Layer (TASK-092) | Does not create decisions, only explains them.");
    Ok(())
}

// ------------------------------------------------------------------
// SymbolScoreboard
// ------------------------------------------------------------------

pub fn handle_symbol_scoreboard(
    context: &AppContext,
    date: Option<NaiveDate>,
    scope: ReportScopeArg,
) -> Result<()> {
    use report_builder::{
        AuditReportBuilder, ScoreboardReportInput, ScoreboardInputRow,
    };
    use reporting::ReportingSnapshot;
    use reporting::Formatter;
    use report_renderer::{render, TextFormatter};

    let report_date = date.unwrap_or_else(|| {
        market_store::fetch_latest_table_date(&context.storage, "signal_snapshot")
            .unwrap_or(None)
            .unwrap_or_else(|| Local::now().date_naive())
    });
    let scope: ReportScope = scope.into();

    let signals = market_store::fetch_signal_snapshots_for_date_with_scope(&context.storage, report_date, scope)?;
    let rotations = market_store::fetch_rotation_ranks_for_date(&context.storage, report_date)?;
    let state = market_store::fetch_strategy_states_for_scope(&context.storage, scope)?
        .into_iter().find(|s| s.date == report_date);

    // Build instrument name lookup
    let instruments = context.seed_universe().unwrap_or_default();
    let name_map: HashMap<&str, &str> = instruments
        .iter()
        .map(|i| (i.symbol.as_str(), i.name.as_str()))
        .collect();

    let mut sorted = signals.clone();
    sorted.sort_by(|a, b| b.final_score.total_cmp(&a.final_score).then(a.symbol.cmp(&b.symbol)));

    let detail_rows: Vec<ScoreboardInputRow> = sorted
        .iter()
        .map(|signal| {
            let rot = rotations.iter().find(|r| r.symbol == signal.symbol);
            let momentum = rot.map(|r| r.momentum_score).unwrap_or(0.0);
            let name = name_map
                .get(signal.symbol.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| signal.symbol.clone());
            let state_label = state
                .as_ref()
                .map(|s| format!("{:?}", s.state))
                .unwrap_or_else(|| "DeRisk".to_string());
            ScoreboardInputRow {
                symbol: signal.symbol.clone(),
                name,
                final_score: signal.final_score,
                signal_label: signal.signal_label.to_string(),
                momentum_score: momentum,
                regime_label: state_label,
            }
        })
        .collect();

    // Build ReportingSnapshot from ResearchContext
    let research_ctx = context.build_research_context_for_date(report_date, scope)?;
    let snapshot = ReportingSnapshot {
        generated_at: chrono::Utc::now(),
        research: research_ctx,
    };

    let input = ScoreboardReportInput {
        rows: detail_rows,
    };

    let doc = AuditReportBuilder::build_scoreboard(&snapshot, &input)?;

    let mut fmt = TextFormatter::new();
    render(&mut fmt, &doc);
    println!("{}", fmt.finalize());
    println!(
        "Explainability Layer (TASK-092) | Research Surface — not in Shadow Production chain."
    );
    Ok(())
}

// ------------------------------------------------------------------
// AuditLeadLag
// ------------------------------------------------------------------

pub fn handle_audit_lead_lag(
    context: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<()> {
    let cn_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["000300".to_string()],
        from,
        to,
    )?;
    let hk_bars = market_store::fetch_daily_bars_for_symbols_in_range(
        &context.storage,
        &["HSCEI".to_string()],
        from,
        to,
    )?;

    let all_regimes = market_store::fetch_market_regimes(&context.storage)?;
    let cn_regimes: Vec<_> = all_regimes
        .clone()
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case("CN") && r.date >= from && r.date <= to)
        .collect();
    let hk_regimes: Vec<_> = all_regimes
        .into_iter()
        .filter(|r| r.market.eq_ignore_ascii_case("HK") && r.date >= from && r.date <= to)
        .collect();

    let mut all_outputs = Vec::new();

    if !cn_bars.is_empty() && !cn_regimes.is_empty() {
        let report = regime_audit::lead_lag_analysis::analyze_lead_lag("CN", &cn_regimes, &cn_bars);
        let summary = regime_audit::lead_lag_analysis::aggregate_by_state(&report.episodes);

        let format_summary = |s: &regime_audit::lead_lag_analysis::StateLeadLagSummary| {
            serde_json::json!({
                "state": s.state.clone(),
                "episode_count": s.episode_count,
                "avg_duration_days": format!("{:.1}", s.avg_duration_days),
                "avg_return": {
                    "before_20d": format!("{:.2}%", s.avg_before_20d * 100.0),
                    "before_60d": format!("{:.2}%", s.avg_before_60d * 100.0),
                    "during": format!("{:.2}%", s.avg_during * 100.0),
                    "after_20d": format!("{:.2}%", s.avg_after_20d * 100.0),
                    "after_60d": format!("{:.2}%", s.avg_after_60d * 100.0),
                }
            })
        };

        all_outputs.push(serde_json::json!({
            "market": "CN",
            "total_episodes": report.episodes.len(),
            "summary": summary.iter().map(format_summary).collect::<Vec<_>>(),
        }));
    }

    if !hk_bars.is_empty() && !hk_regimes.is_empty() {
        let report = regime_audit::lead_lag_analysis::analyze_lead_lag("HK", &hk_regimes, &hk_bars);
        let summary = regime_audit::lead_lag_analysis::aggregate_by_state(&report.episodes);

        let format_summary = |s: &regime_audit::lead_lag_analysis::StateLeadLagSummary| {
            serde_json::json!({
                "state": s.state.clone(),
                "episode_count": s.episode_count,
                "avg_duration_days": format!("{:.1}", s.avg_duration_days),
                "avg_return": {
                    "before_20d": format!("{:.2}%", s.avg_before_20d * 100.0),
                    "before_60d": format!("{:.2}%", s.avg_before_60d * 100.0),
                    "during": format!("{:.2}%", s.avg_during * 100.0),
                    "after_20d": format!("{:.2}%", s.avg_after_20d * 100.0),
                    "after_60d": format!("{:.2}%", s.avg_after_60d * 100.0),
                }
            })
        };

        all_outputs.push(serde_json::json!({
            "market": "HK",
            "total_episodes": report.episodes.len(),
            "summary": summary.iter().map(format_summary).collect::<Vec<_>>(),
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "status": "ok",
            "task": "TASK-071B",
            "description": "State Lead/Lag Analysis",
            "results": all_outputs,
        }))?);
    Ok(())
}

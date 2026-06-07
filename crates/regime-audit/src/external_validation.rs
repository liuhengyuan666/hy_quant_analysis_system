use chrono::NaiveDate;
use core_domain::DailyBar;
use gt_regime_generator::RegimeLabel;
use std::collections::HashMap;

// ============================================================
// External Validation (TASK-018A)
// Validates that GT regimes have economic meaning.
// ============================================================

#[derive(Debug, Clone)]
pub struct RegimeValidationStat {
    pub count: usize,
    pub pct: f64,
    pub forward_return_20d_median: f64,
    pub forward_return_60d_median: f64,
    pub forward_return_20d_mean: f64,
    pub forward_return_60d_mean: f64,
    pub max_drawdown_median: f64,
    pub volatility_median: f64,
    pub sharpe_median: f64,
    pub win_rate_20d: f64,
    pub win_rate_60d: f64,
}

#[derive(Debug, Clone)]
pub struct RegimeValidationReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub stats: HashMap<String, RegimeValidationStat>,
    pub assessment: String,
    pub separation_score: EconomicSeparationScore,
}

/// Economic Separation Score: 0-100
/// Evaluates how well GT regimes separate across economic dimensions.
/// Higher = better separation (RiskOn > Neutral > RiskOff for positive metrics).
#[derive(Debug, Clone)]
pub struct EconomicSeparationScore {
    pub overall_score: f64,
    pub gate_results: HashMap<String, bool>,
    pub rank_scores: HashMap<String, f64>,
}

/// Validate GT regimes against forward returns and risk metrics.
///
/// For each day with a regime label, computes:
/// - Forward returns (20d and 60d)
/// - Max drawdown within forward window
/// - Realized volatility within forward window
/// - Sharpe ratio within forward window
///
/// Then groups by regime and reports medians.
pub fn validate_regimes_economically(
    labels: &[RegimeLabel],
    bars: &[DailyBar],
    scope: &str,
    anchor_symbol: &str,
) -> RegimeValidationReport {
    if labels.is_empty() || bars.is_empty() {
        return RegimeValidationReport {
            scope: scope.to_string(),
            anchor_symbol: anchor_symbol.to_string(),
            window_from: labels.first().map(|l| l.date).unwrap_or(NaiveDate::MIN),
            window_to: labels.last().map(|l| l.date).unwrap_or(NaiveDate::MIN),
            total_days: 0,
            stats: HashMap::new(),
            assessment: "no data".to_string(),
            separation_score: EconomicSeparationScore {
                overall_score: 0.0,
                gate_results: HashMap::new(),
                rank_scores: HashMap::new(),
            },
        };
    }

    // Build date -> close map from bars
    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let mut regime_data: HashMap<String, Vec<(f64, f64, f64, f64)>> = HashMap::new();
    // Each entry: (forward_return_20d, forward_return_60d, max_dd_20d, realized_vol_20d)

    for (index, label) in labels.iter().enumerate() {
        let Some(current_close) = close_by_date.get(&label.date) else {
            continue;
        };
        if *current_close <= 0.0 {
            continue;
        }

        let regime_key = format!("{:?}", label.regime).to_lowercase();

        // Find forward returns
        let forward_20_close = labels
            .get(index + 20)
            .and_then(|l| close_by_date.get(&l.date));
        let forward_60_close = labels
            .get(index + 60)
            .and_then(|l| close_by_date.get(&l.date));

        let ret_20 = forward_20_close.map(|c| (c - current_close) / current_close);
        let ret_60 = forward_60_close.map(|c| (c - current_close) / current_close);

        // Compute max drawdown and volatility in 20-day forward window
        let forward_closes: Vec<f64> = (1..=20)
            .filter_map(|offset| {
                labels
                    .get(index + offset)
                    .and_then(|l| close_by_date.get(&l.date))
            })
            .copied()
            .collect();

        let (max_dd, vol) = if forward_closes.len() >= 10 {
            let max_dd = calculate_max_drawdown(*current_close, &forward_closes);
            let vol = calculate_realized_volatility_from_closes(*current_close, &forward_closes);
            (max_dd, vol)
        } else {
            (0.0, 0.0)
        };

        if let (Some(r20), Some(r60)) = (ret_20, ret_60) {
            regime_data
                .entry(regime_key)
                .or_default()
                .push((r20, r60, max_dd, vol));
        }
    }

    let mut stats = HashMap::new();
    for (regime_key, data) in &regime_data {
        if data.is_empty() {
            continue;
        }

        let count = data.len();
        let mut rets_20: Vec<f64> = data.iter().map(|(r20, _, _, _)| *r20).collect();
        let mut rets_60: Vec<f64> = data.iter().map(|(_, r60, _, _)| *r60).collect();
        let mut dds: Vec<f64> = data.iter().map(|(_, _, dd, _)| *dd).collect();
        let mut vols: Vec<f64> = data.iter().map(|(_, _, _, vol)| *vol).collect();

        rets_20.sort_by(|a, b| a.partial_cmp(b).unwrap());
        rets_60.sort_by(|a, b| a.partial_cmp(b).unwrap());
        dds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        vols.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let win_20 = rets_20.iter().filter(|r| **r > 0.0).count() as f64 / count as f64;
        let win_60 = rets_60.iter().filter(|r| **r > 0.0).count() as f64 / count as f64;

        // Sharpe = mean return / std dev (annualized)
        let mean_20 = rets_20.iter().sum::<f64>() / count as f64;
        let variance = rets_20.iter().map(|r| (r - mean_20).powi(2)).sum::<f64>() / count as f64;
        let sharpe = if variance > 0.0 {
            (mean_20 * 12.0) / (variance.sqrt() * (12.0_f64).sqrt())
        } else {
            0.0
        };

        stats.insert(
            regime_key.clone(),
            RegimeValidationStat {
                count,
                pct: count as f64 / labels.len() as f64,
                forward_return_20d_median: percentile(&rets_20, 0.50),
                forward_return_60d_median: percentile(&rets_60, 0.50),
                forward_return_20d_mean: mean_20,
                forward_return_60d_mean: rets_60.iter().sum::<f64>() / count as f64,
                max_drawdown_median: percentile(&dds, 0.50),
                volatility_median: percentile(&vols, 0.50),
                sharpe_median: sharpe,
                win_rate_20d: win_20,
                win_rate_60d: win_60,
            },
        );
    }

    let assessment = generate_assessment(&stats);
    let separation = compute_separation_score(&stats);

    RegimeValidationReport {
        scope: scope.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from: labels.first().unwrap().date,
        window_to: labels.last().unwrap().date,
        total_days: labels.len(),
        stats,
        assessment,
        separation_score: separation,
    }
}

/// Compute Economic Separation Score (0-100)
/// Evaluates how well regimes separate across return, sharpe, drawdown, and winrate.
///
/// Ranking formula for each metric:
///   - Rank regimes by that metric (1=best, 3=worst)
///   - Score = 100 - (rank_error * 33.3)
///   - rank_error = sum of |actual_rank - ideal_rank| for each regime
///   - Average across all metrics = overall_score
///
/// Ideal ranking: RiskOn rank=1, Neutral rank=2, RiskOff rank=3
/// (for drawdown: lower is better, so RiskOn gets rank 1, RiskOff rank 3)
pub fn compute_separation_score(stats: &HashMap<String, RegimeValidationStat>) -> EconomicSeparationScore {
    let risk_on = stats.get("riskon");
    let risk_off = stats.get("riskoff");
    let neutral = stats.get("neutral");

    let mut gate_results = HashMap::new();

    // Gate 1: Sharpe(RiskOn) > Sharpe(Neutral) > Sharpe(RiskOff)
    let gate1 = match (risk_on, neutral, risk_off) {
        (Some(ro), Some(neu), Some(rf)) => {
            ro.sharpe_median > neu.sharpe_median && neu.sharpe_median > rf.sharpe_median
        }
        (Some(ro), None, Some(rf)) => ro.sharpe_median > rf.sharpe_median,
        _ => false,
    };
    gate_results.insert("sharpe_ranking".to_string(), gate1);

    // Gate 2: Return(RiskOn) > Return(Neutral) > Return(RiskOff)
    let gate2 = match (risk_on, neutral, risk_off) {
        (Some(ro), Some(neu), Some(rf)) => {
            ro.forward_return_60d_mean > neu.forward_return_60d_mean
                && neu.forward_return_60d_mean > rf.forward_return_60d_mean
        }
        (Some(ro), None, Some(rf)) => ro.forward_return_60d_mean > rf.forward_return_60d_mean,
        _ => false,
    };
    gate_results.insert("return_ranking".to_string(), gate2);

    // Gate 3: Drawdown(RiskOn) < Drawdown(RiskOff)
    let gate3 = match (risk_on, risk_off) {
        (Some(ro), Some(rf)) => ro.max_drawdown_median < rf.max_drawdown_median,
        _ => false,
    };
    gate_results.insert("drawdown_ranking".to_string(), gate3);

    // Gate 4: RiskOff Return <= 0 (or at least significantly lower than RiskOn)
    let gate4 = match (risk_on, risk_off) {
        (Some(ro), Some(rf)) => {
            rf.forward_return_60d_mean <= 0.0
                || (ro.forward_return_60d_mean - rf.forward_return_60d_mean) > 0.03
        }
        _ => false,
    };
    gate_results.insert("riskoff_return".to_string(), gate4);

    // Compute rank-based scores for each metric
    // For positive metrics (return, sharpe, win_rate): higher value = rank 1
    // For drawdown: lower value = rank 1
    let return_score = compute_metric_rank_score(stats, |s| s.forward_return_60d_mean, true);
    let sharpe_score = compute_metric_rank_score(stats, |s| s.sharpe_median, true);
    let winrate_score = compute_metric_rank_score(stats, |s| s.win_rate_60d, true);
    let drawdown_score = compute_metric_rank_score(stats, |s| s.max_drawdown_median, false);

    let overall_score = (return_score + sharpe_score + drawdown_score + winrate_score) / 4.0;

    let mut rank_scores = HashMap::new();
    rank_scores.insert("return_60d".to_string(), return_score);
    rank_scores.insert("sharpe".to_string(), sharpe_score);
    rank_scores.insert("drawdown".to_string(), drawdown_score);
    rank_scores.insert("win_rate".to_string(), winrate_score);

    EconomicSeparationScore {
        overall_score: overall_score.clamp(0.0, 100.0),
        gate_results,
        rank_scores,
    }
}

/// Compute a rank-based score for a single metric.
///
/// - `extractor`: extracts the metric value from RegimeValidationStat
/// - `higher_better`: true for return/sharpe/win_rate, false for drawdown
///
/// Assigns ranks to available regimes (1=best), computes rank_error as sum of
/// |actual - ideal|, and returns 100 - (rank_error * 33.3).
///
/// Ideal ranks: RiskOn=1, Neutral=2, RiskOff=3
fn compute_metric_rank_score(
    stats: &HashMap<String, RegimeValidationStat>,
    extractor: fn(&RegimeValidationStat) -> f64,
    higher_better: bool,
) -> f64 {
    let regimes = ["riskoff", "neutral", "riskon"];
    let ideal_ranks: HashMap<&str, f64> = [
        ("riskoff", 3.0),
        ("neutral", 2.0),
        ("riskon", 1.0),
    ]
    .into_iter()
    .collect();

    // Collect available regimes with their metric values
    let mut values: Vec<(String, f64)> = regimes
        .iter()
        .filter_map(|&r| stats.get(r).map(|s| (r.to_string(), extractor(s))))
        .collect();

    if values.len() < 2 {
        return 50.0; // Not enough regimes to rank
    }

    // Sort to assign ranks (1=best)
    if higher_better {
        values.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // descending: highest = rank 1
    } else {
        values.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap()); // ascending: lowest = rank 1
    }

    // Compute rank error
    let mut rank_error = 0.0;
    for (i, (name, _)) in values.iter().enumerate() {
        let actual_rank = (i + 1) as f64;
        let ideal_rank = ideal_ranks
            .get(name.as_str())
            .copied()
            .unwrap_or(2.0);
        rank_error += (actual_rank - ideal_rank).abs();
    }

    (100.0 - rank_error * 33.3).clamp(0.0, 100.0)
}

fn calculate_max_drawdown(start_close: f64, forward_closes: &[f64]) -> f64 {
    let mut peak = start_close;
    let mut max_dd = 0.0;
    for close in forward_closes {
        if *close > peak {
            peak = *close;
        }
        let dd = (peak - *close) / peak;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    max_dd
}

fn calculate_realized_volatility_from_closes(start_close: f64, forward_closes: &[f64]) -> f64 {
    let mut prices = vec![start_close];
    prices.extend_from_slice(forward_closes);

    if prices.len() < 5 {
        return 0.0;
    }

    let mut log_returns = Vec::with_capacity(prices.len() - 1);
    for window in prices.windows(2) {
        if window[0] > 0.0 {
            log_returns.push((window[1] / window[0]).ln());
        }
    }

    if log_returns.len() < 2 {
        return 0.0;
    }

    let mean = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
    let variance = log_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / log_returns.len() as f64;

    variance.sqrt() * (252.0_f64).sqrt()
}

fn generate_assessment(stats: &HashMap<String, RegimeValidationStat>) -> String {
    let risk_on = stats.get("riskon");
    let risk_off = stats.get("riskoff");
    let neutral = stats.get("neutral");

    match (risk_on, risk_off, neutral) {
        (Some(ro), Some(rf), Some(neu)) => {
            let ro_ret = ro.forward_return_60d_mean;
            let rf_ret = rf.forward_return_60d_mean;
            let neu_ret = neu.forward_return_60d_mean;

            if ro_ret > neu_ret && neu_ret > rf_ret && ro_ret - rf_ret > 0.05 {
                "strong_economic_meaning".to_string()
            } else if ro_ret > rf_ret && ro_ret - rf_ret > 0.02 {
                "moderate_economic_meaning".to_string()
            } else {
                "weak_economic_meaning".to_string()
            }
        }
        (Some(ro), Some(rf), None) => {
            if ro.forward_return_60d_mean > rf.forward_return_60d_mean + 0.02 {
                "bipolar_valid".to_string()
            } else {
                "bipolar_weak".to_string()
            }
        }
        _ => "insufficient_data".to_string(),
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}

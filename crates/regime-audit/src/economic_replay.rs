use chrono::NaiveDate;
use core_domain::{
    DailyBar, EconomicRegimeStat, EconomicReplayReport, EconomicReplayVariant,
    EconomicSeparationScore, MarketRegimeSnapshot,
};
use std::collections::HashMap;

// ============================================================
// TASK-027: Economic Replay Validation
// Computes forward-return economic metrics for regime variants.
// ============================================================

fn classify_counterfactual(
    trend_score: f64,
    risk_score: f64,
    liquidity_score: f64,
    variant: &str,
) -> String {
    match variant {
        "baseline" => {
            if trend_score >= 60.0 && liquidity_score >= 50.0 && risk_score >= 55.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 || risk_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "cn_trend_dominant" => {
            if trend_score >= 60.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "hk_risk_dominant" => {
            if risk_score >= 55.0 {
                "risk_on".to_string()
            } else if risk_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "hybrid_scope_aware" => {
            // This is a placeholder that would need actual scope knowledge
            // For now, same as baseline
            if trend_score >= 60.0 && liquidity_score >= 50.0 && risk_score >= 55.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 || risk_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        _ => "neutral".to_string(),
    }
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

fn calculate_realized_volatility(start_close: f64, forward_closes: &[f64]) -> f64 {
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
    let variance = log_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / log_returns.len() as f64;
    variance.sqrt() * (252.0_f64).sqrt()
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}

fn compute_economic_metrics(
    labels: &[(NaiveDate, String)],
    close_by_date: &HashMap<NaiveDate, f64>,
) -> HashMap<String, Vec<(f64, f64, f64, f64)>> {
    let mut regime_data: HashMap<String, Vec<(f64, f64, f64, f64)>> = HashMap::new();

    for (index, (date, label)) in labels.iter().enumerate() {
        let Some(current_close) = close_by_date.get(date) else {
            continue;
        };
        if *current_close <= 0.0 {
            continue;
        }

        let forward_20_close = labels
            .get(index + 20)
            .and_then(|(d, _)| close_by_date.get(d));
        let forward_60_close = labels
            .get(index + 60)
            .and_then(|(d, _)| close_by_date.get(d));

        let ret_20 = forward_20_close.map(|c| (c - current_close) / current_close);
        let ret_60 = forward_60_close.map(|c| (c - current_close) / current_close);

        let forward_closes: Vec<f64> = (1..=20)
            .filter_map(|offset| {
                labels
                    .get(index + offset)
                    .and_then(|(d, _)| close_by_date.get(d))
            })
            .copied()
            .collect();

        let (max_dd, vol) = if forward_closes.len() >= 10 {
            let max_dd = calculate_max_drawdown(*current_close, &forward_closes);
            let vol = calculate_realized_volatility(*current_close, &forward_closes);
            (max_dd, vol)
        } else {
            (0.0, 0.0)
        };

        if let (Some(r20), Some(r60)) = (ret_20, ret_60) {
            regime_data
                .entry(label.clone())
                .or_default()
                .push((r20, r60, max_dd, vol));
        }
    }

    regime_data
}

fn build_economic_stats(
    regime_data: &HashMap<String, Vec<(f64, f64, f64, f64)>>,
    total_days: usize,
) -> HashMap<String, EconomicRegimeStat> {
    let mut stats = HashMap::new();
    for (regime_key, data) in regime_data {
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

        let mean_20 = rets_20.iter().sum::<f64>() / count as f64;
        let variance = rets_20.iter().map(|r| (r - mean_20).powi(2)).sum::<f64>() / count as f64;
        let sharpe = if variance > 0.0 {
            (mean_20 * 12.0) / (variance.sqrt() * (12.0_f64).sqrt())
        } else {
            0.0
        };

        stats.insert(
            regime_key.clone(),
            EconomicRegimeStat {
                count,
                pct: count as f64 / total_days as f64,
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
    stats
}

fn compute_separation_score(stats: &HashMap<String, EconomicRegimeStat>) -> EconomicSeparationScore {
    let risk_on = stats.get("risk_on");
    let risk_off = stats.get("risk_off");
    let neutral = stats.get("neutral");

    let mut gate_results = HashMap::new();

    let gate1 = match (risk_on, neutral, risk_off) {
        (Some(ro), Some(neu), Some(rf)) => {
            ro.sharpe_median > neu.sharpe_median && neu.sharpe_median > rf.sharpe_median
        }
        (Some(ro), None, Some(rf)) => ro.sharpe_median > rf.sharpe_median,
        _ => false,
    };
    gate_results.insert("sharpe_ranking".to_string(), gate1);

    let gate2 = match (risk_on, neutral, risk_off) {
        (Some(ro), Some(neu), Some(rf)) => {
            ro.forward_return_60d_mean > neu.forward_return_60d_mean
                && neu.forward_return_60d_mean > rf.forward_return_60d_mean
        }
        (Some(ro), None, Some(rf)) => ro.forward_return_60d_mean > rf.forward_return_60d_mean,
        _ => false,
    };
    gate_results.insert("return_ranking".to_string(), gate2);

    let gate3 = match (risk_on, risk_off) {
        (Some(ro), Some(rf)) => ro.max_drawdown_median < rf.max_drawdown_median,
        _ => false,
    };
    gate_results.insert("drawdown_ranking".to_string(), gate3);

    let gate4 = match (risk_on, risk_off) {
        (Some(ro), Some(rf)) => {
            rf.forward_return_60d_mean <= 0.0
                || (ro.forward_return_60d_mean - rf.forward_return_60d_mean) > 0.03
        }
        _ => false,
    };
    gate_results.insert("riskoff_return".to_string(), gate4);

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

fn compute_metric_rank_score(
    stats: &HashMap<String, EconomicRegimeStat>,
    extractor: fn(&EconomicRegimeStat) -> f64,
    higher_better: bool,
) -> f64 {
    let regimes = ["risk_off", "neutral", "risk_on"];
    let ideal_ranks: HashMap<&str, f64> = [
        ("risk_off", 3.0),
        ("neutral", 2.0),
        ("risk_on", 1.0),
    ]
    .into_iter()
    .collect();

    let mut values: Vec<(String, f64)> = regimes
        .iter()
        .filter_map(|&r| stats.get(r).map(|s| (r.to_string(), extractor(s))))
        .collect();

    if values.len() < 2 {
        return 50.0;
    }

    if higher_better {
        values.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    } else {
        values.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    }

    let mut rank_error = 0.0;
    for (i, (name, _)) in values.iter().enumerate() {
        let actual_rank = (i + 1) as f64;
        let ideal_rank = ideal_ranks.get(name.as_str()).copied().unwrap_or(2.0);
        rank_error += (actual_rank - ideal_rank).abs();
    }

    (100.0 - rank_error * 33.3).clamp(0.0, 100.0)
}

fn generate_assessment(stats: &HashMap<String, EconomicRegimeStat>) -> String {
    let risk_on = stats.get("risk_on");
    let risk_off = stats.get("risk_off");
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

// Compute alignment (same formula as state_alignment)
fn compute_alignment(labels: &[(NaiveDate, String)], close_by_date: &HashMap<NaiveDate, f64>) -> f64 {
    let mut dd20_tp = 0usize;
    let mut dd20_fp = 0usize;
    let mut dd20_fn = 0usize;
    let mut uptrend_tp = 0usize;
    let mut uptrend_fp = 0usize;
    let mut uptrend_fn = 0usize;

    // Build ordered dates for forward lookup
    let dates: Vec<NaiveDate> = labels.iter().map(|(d, _)| *d).collect();
    let mut closes: Vec<Option<f64>> = Vec::new();
    let mut ma20_vec: Vec<Option<f64>> = Vec::new();
    let mut ma60_vec: Vec<Option<f64>> = Vec::new();

    for (i, date) in dates.iter().enumerate() {
        closes.push(close_by_date.get(date).copied());
        if i >= 19 {
            let window: Vec<f64> = closes[i - 19..=i].iter().filter_map(|&c| c).collect();
            if window.len() == 20 {
                ma20_vec.push(Some(window.iter().sum::<f64>() / 20.0));
            } else {
                ma20_vec.push(None);
            }
        } else {
            ma20_vec.push(None);
        }
        if i >= 59 {
            let window: Vec<f64> = closes[i - 59..=i].iter().filter_map(|&c| c).collect();
            if window.len() == 60 {
                ma60_vec.push(Some(window.iter().sum::<f64>() / 60.0));
            } else {
                ma60_vec.push(None);
            }
        } else {
            ma60_vec.push(None);
        }
    }

    for (i, (_date, label)) in labels.iter().enumerate() {
        let is_riskoff = label.eq_ignore_ascii_case("risk_off");
        let is_riskon = label.eq_ignore_ascii_case("risk_on");

        let close = closes.get(i).copied().flatten().unwrap_or(0.0);
        let recent_high = closes[..=i].iter().filter_map(|&c| c).fold(0.0, f64::max);
        let dd = if recent_high > 0.0 {
            ((close - recent_high) / recent_high * 100.0).clamp(-100.0, 0.0)
        } else {
            0.0
        };
        let is_dd20 = dd < -20.0;

        let is_uptrend = if let (Some(m20), Some(m60)) = (ma20_vec[i], ma60_vec[i]) {
            close > m20 && m20 > m60
        } else {
            false
        };

        if is_riskoff {
            if is_dd20 {
                dd20_tp += 1;
            } else {
                dd20_fp += 1;
            }
        } else if is_dd20 {
            dd20_fn += 1;
        }

        if is_riskon {
            if is_uptrend {
                uptrend_tp += 1;
            } else {
                uptrend_fp += 1;
            }
        } else if is_uptrend {
            uptrend_fn += 1;
        }
    }

    let (_, _, dd20_f1) = {
        let tp = dd20_tp;
        let fp = dd20_fp;
        let fn_ = dd20_fn;
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };
        (precision, recall, f1)
    };

    let (_, _, uptrend_f1) = {
        let tp = uptrend_tp;
        let fp = uptrend_fp;
        let fn_ = uptrend_fn;
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };
        (precision, recall, f1)
    };

    (dd20_f1 + uptrend_f1) / 2.0
}

fn compute_information(labels: &[(NaiveDate, String)]) -> f64 {
    let total = labels.len() as f64;
    if total == 0.0 {
        return 0.0;
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (_, label) in labels {
        *counts.entry(label.clone()).or_insert(0) += 1;
    }
    let mut entropy = 0.0;
    for (_, count) in counts {
        let p = count as f64 / total;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    let max_entropy = (3.0f64).log2();
    (entropy / max_entropy).clamp(0.0, 1.0)
}

pub fn compute_economic_replay(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<EconomicReplayReport> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let regimes_filtered: Vec<_> = regimes
        .iter()
        .filter(|r| close_by_date.contains_key(&r.date))
        .collect();

    if regimes_filtered.is_empty() {
        return None;
    }

    let total_days = regimes_filtered.len();
    let window_from = regimes_filtered.first().map(|r| r.date).unwrap_or(bars[0].date);
    let window_to = regimes_filtered.last().map(|r| r.date).unwrap_or(bars[bars.len() - 1].date);

    let variants = vec!["baseline", "cn_trend_dominant", "hk_risk_dominant", "hybrid_scope_aware"];
    let mut variant_results = Vec::new();

    for variant in variants {
        let labels: Vec<(NaiveDate, String)> = regimes_filtered
            .iter()
            .map(|r| {
                let label = classify_counterfactual(r.trend_score, r.risk_score, r.liquidity_score, variant);
                (r.date, label)
            })
            .collect();

        let alignment = compute_alignment(&labels, &close_by_date);
        let information = compute_information(&labels);

        let regime_data = compute_economic_metrics(&labels, &close_by_date);
        let economic_stats = build_economic_stats(&regime_data, total_days);
        let separation_score = compute_separation_score(&economic_stats);
        let assessment = generate_assessment(&economic_stats);

        let mut regime_distribution: HashMap<String, f64> = HashMap::new();
        for (_, label) in &labels {
            *regime_distribution.entry(label.clone()).or_insert(0.0) += 1.0 / total_days as f64;
        }

        variant_results.push(EconomicReplayVariant {
            name: variant.to_string(),
            regime_distribution,
            alignment,
            information,
            economic_stats,
            separation_score,
            assessment,
        });
    }

    Some(EconomicReplayReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        variants: variant_results,
    })
}

use chrono::NaiveDate;
use core_domain::{
    DailyBar, MarketRegimeSnapshot, ParetoFrontierReport, ParetoPoint,
};
use std::collections::HashMap;

// ============================================================
// TASK-028B: Pareto Frontier Analysis
// Maps the Alignment vs Economic Separation trade-off.
// ============================================================

fn classify_variant(
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
        "trend_only" => {
            if trend_score >= 60.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "risk_only" => {
            if risk_score >= 55.0 {
                "risk_on".to_string()
            } else if risk_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "liquidity_only" => {
            if liquidity_score >= 50.0 {
                "risk_on".to_string()
            } else if liquidity_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "trend_and_risk" => {
            if trend_score >= 60.0 && risk_score >= 55.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 && risk_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "symmetric_and" => {
            if trend_score >= 60.0 && risk_score >= 55.0 && liquidity_score >= 50.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 && risk_score < 40.0 && liquidity_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "trend_dominant" => {
            if trend_score >= 60.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "macro_dominant" => {
            if risk_score >= 55.0 && liquidity_score >= 50.0 {
                "risk_on".to_string()
            } else if risk_score < 40.0 || liquidity_score < 40.0 {
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
        "hk_liquidity_dominant" => {
            if liquidity_score >= 50.0 {
                "risk_on".to_string()
            } else if liquidity_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        _ => "neutral".to_string(),
    }
}

// Reuse alignment computation from state_alignment.rs logic
fn compute_alignment(labels: &[(NaiveDate, String)], close_by_date: &HashMap<NaiveDate, f64>) -> f64 {
    let mut dd20_tp = 0usize;
    let mut dd20_fp = 0usize;
    let mut dd20_fn = 0usize;
    let mut uptrend_tp = 0usize;
    let mut uptrend_fp = 0usize;
    let mut uptrend_fn = 0usize;

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

    let dd20_f1 = {
        let tp = dd20_tp;
        let fp = dd20_fp;
        let fn_ = dd20_fn;
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        }
    };

    let uptrend_f1 = {
        let tp = uptrend_tp;
        let fp = uptrend_fp;
        let fn_ = uptrend_fn;
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        }
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

// Economic metrics helpers (from economic_replay.rs)
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

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}

fn compute_separation_score(labels: &[(NaiveDate, String)], close_by_date: &HashMap<NaiveDate, f64>) -> f64 {
    let mut regime_data: HashMap<String, Vec<(f64, f64, f64)>> = HashMap::new();

    for (index, (date, label)) in labels.iter().enumerate() {
        let Some(current_close) = close_by_date.get(date) else {
            continue;
        };
        if *current_close <= 0.0 {
            continue;
        }

        let forward_60_close = labels
            .get(index + 60)
            .and_then(|(d, _)| close_by_date.get(d));
        let ret_60 = forward_60_close.map(|c| (c - current_close) / current_close);

        let forward_closes: Vec<f64> = (1..=20)
            .filter_map(|offset| {
                labels
                    .get(index + offset)
                    .and_then(|(d, _)| close_by_date.get(d))
            })
            .copied()
            .collect();

        let max_dd = if forward_closes.len() >= 10 {
            calculate_max_drawdown(*current_close, &forward_closes)
        } else {
            0.0
        };

        if let Some(r60) = ret_60 {
            regime_data.entry(label.clone()).or_default().push((r60, max_dd, 0.0));
        }
    }

    if regime_data.len() < 2 {
        return 50.0;
    }

    let mut stats: HashMap<String, (f64, f64, f64)> = HashMap::new();
    for (regime_key, data) in &regime_data {
        if data.is_empty() {
            continue;
        }
        let mut rets: Vec<f64> = data.iter().map(|(r, _, _)| *r).collect();
        let mut dds: Vec<f64> = data.iter().map(|(_, dd, _)| *dd).collect();
        rets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        dds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean_ret = rets.iter().sum::<f64>() / rets.len() as f64;
        let median_dd = percentile(&dds, 0.50);
        let win_rate = rets.iter().filter(|r| **r > 0.0).count() as f64 / rets.len() as f64;
        stats.insert(regime_key.clone(), (mean_ret, median_dd, win_rate));
    }

    // Compute rank-based separation score
    let regimes = ["risk_off", "neutral", "risk_on"];
    let ideal_ranks: HashMap<&str, f64> = [
        ("risk_off", 3.0),
        ("neutral", 2.0),
        ("risk_on", 1.0),
    ]
    .into_iter()
    .collect();

    let mut return_score = 50.0;
    let mut drawdown_score = 50.0;
    let mut winrate_score = 50.0;

    // Return score: higher is better, ideal = RiskOn > Neutral > RiskOff
    {
        let mut values: Vec<(String, f64)> = regimes
            .iter()
            .filter_map(|&r| stats.get(r).map(|s| (r.to_string(), s.0)))
            .collect();
        if values.len() >= 2 {
            values.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let mut rank_error = 0.0;
            for (i, (name, _)) in values.iter().enumerate() {
                let actual_rank = (i + 1) as f64;
                let ideal_rank = ideal_ranks.get(name.as_str()).copied().unwrap_or(2.0);
                rank_error += (actual_rank - ideal_rank).abs();
            }
            return_score = (100.0 - rank_error * 33.3).clamp(0.0, 100.0);
        }
    }

    // Drawdown score: lower is better, ideal = RiskOn < Neutral < RiskOff
    {
        let mut values: Vec<(String, f64)> = regimes
            .iter()
            .filter_map(|&r| stats.get(r).map(|s| (r.to_string(), s.1)))
            .collect();
        if values.len() >= 2 {
            values.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            let mut rank_error = 0.0;
            for (i, (name, _)) in values.iter().enumerate() {
                let actual_rank = (i + 1) as f64;
                let ideal_rank = ideal_ranks.get(name.as_str()).copied().unwrap_or(2.0);
                rank_error += (actual_rank - ideal_rank).abs();
            }
            drawdown_score = (100.0 - rank_error * 33.3).clamp(0.0, 100.0);
        }
    }

    // Win rate score: higher is better
    {
        let mut values: Vec<(String, f64)> = regimes
            .iter()
            .filter_map(|&r| stats.get(r).map(|s| (r.to_string(), s.2)))
            .collect();
        if values.len() >= 2 {
            values.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let mut rank_error = 0.0;
            for (i, (name, _)) in values.iter().enumerate() {
                let actual_rank = (i + 1) as f64;
                let ideal_rank = ideal_ranks.get(name.as_str()).copied().unwrap_or(2.0);
                rank_error += (actual_rank - ideal_rank).abs();
            }
            winrate_score = (100.0 - rank_error * 33.3).clamp(0.0, 100.0);
        }
    }

    (return_score + drawdown_score + winrate_score) / 3.0
}

fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len()) as f64;
    if n < 2.0 {
        return 0.0;
    }
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut den_x = 0.0;
    let mut den_y = 0.0;
    for i in 0..x.len().min(y.len()) {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        num += dx * dy;
        den_x += dx * dx;
        den_y += dy * dy;
    }
    let den = den_x.sqrt() * den_y.sqrt();
    if den == 0.0 {
        0.0
    } else {
        (num / den).clamp(-1.0, 1.0)
    }
}

fn identify_pareto_optimal(points: &mut [ParetoPoint]) {
    // Sort by alignment descending, then by separation descending
    points.sort_by(|a, b| {
        b.alignment
            .partial_cmp(&a.alignment)
            .unwrap()
            .then_with(|| b.separation_score.partial_cmp(&a.separation_score).unwrap())
    });

    let mut max_separation = f64::NEG_INFINITY;
    for point in points.iter_mut() {
        if point.separation_score >= max_separation {
            point.is_pareto_optimal = true;
            max_separation = point.separation_score;
        }
    }
}

pub fn compute_pareto_frontier(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<ParetoFrontierReport> {
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

    let variants = vec![
        "baseline",
        "trend_only",
        "risk_only",
        "liquidity_only",
        "trend_and_risk",
        "symmetric_and",
        "trend_dominant",
        "macro_dominant",
        "cn_trend_dominant",
        "hk_risk_dominant",
        "hk_liquidity_dominant",
    ];

    let mut points = Vec::new();

    for variant in variants {
        let labels: Vec<(NaiveDate, String)> = regimes_filtered
            .iter()
            .map(|r| {
                let label = classify_variant(r.trend_score, r.risk_score, r.liquidity_score, variant);
                (r.date, label)
            })
            .collect();

        let alignment = compute_alignment(&labels, &close_by_date);
        let information = compute_information(&labels);
        let separation = compute_separation_score(&labels, &close_by_date);

        points.push(ParetoPoint {
            variant: variant.to_string(),
            alignment,
            separation_score: separation,
            information,
            is_pareto_optimal: false,
        });
    }

    identify_pareto_optimal(&mut points);

    // Compute correlation between alignment and separation
    let alignments: Vec<f64> = points.iter().map(|p| p.alignment).collect();
    let separations: Vec<f64> = points.iter().map(|p| p.separation_score).collect();
    let correlation = pearson_correlation(&alignments, &separations);

    // Trade-off detected if correlation is strongly negative
    let trade_off_detected = correlation < -0.3;

    let pareto_optimal_variants: Vec<String> = points
        .iter()
        .filter(|p| p.is_pareto_optimal)
        .map(|p| p.variant.clone())
        .collect();

    Some(ParetoFrontierReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        points,
        correlation,
        trade_off_detected,
        pareto_optimal_variants,
    })
}

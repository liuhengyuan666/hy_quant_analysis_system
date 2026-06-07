use chrono::NaiveDate;
use core_domain::{
    CrossMatrixCell, DailyBar, DualLayerValidationReport, MarketRegimeSnapshot,
    StabilityWindowResult,
};
use std::collections::HashMap;

// ============================================================
// TASK-030: Dual Layer Validation
// Validates orthogonality between State Layer and Economic Layer.
// ============================================================

fn classify_economic_state(liquidity_score: f64, risk_score: f64, scope: &str) -> String {
    match scope {
        "HK" => {
            if liquidity_score >= 55.0 {
                "favorable".to_string()
            } else if liquidity_score < 40.0 {
                "unfavorable".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "CN" => {
            if liquidity_score >= 50.0 {
                "favorable".to_string()
            } else if liquidity_score < 35.0 {
                "unfavorable".to_string()
            } else {
                "neutral".to_string()
            }
        }
        _ => {
            let composite = (liquidity_score + risk_score) / 2.0;
            if composite >= 55.0 {
                "favorable".to_string()
            } else if composite < 40.0 {
                "unfavorable".to_string()
            } else {
                "neutral".to_string()
            }
        }
    }
}

fn normalize_regime_label(label: &str) -> String {
    label.to_lowercase().replace("risk_on", "riskon").replace("risk_off", "riskoff")
}

fn discretize(values: &[f64], n_bins: usize) -> Vec<usize> {
    if values.is_empty() || n_bins == 0 {
        return Vec::new();
    }
    let min_val = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_val = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;
    if range.abs() < f64::EPSILON {
        return vec![0; values.len()];
    }
    values
        .iter()
        .map(|&v| {
            let bin = ((v - min_val) / range * n_bins as f64).floor() as usize;
            bin.min(n_bins - 1)
        })
        .collect()
}

fn mutual_information(x_bins: &[usize], y_bins: &[usize]) -> f64 {
    let n = x_bins.len().min(y_bins.len()) as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mut joint_counts: HashMap<(usize, usize), usize> = HashMap::new();
    let mut x_counts: HashMap<usize, usize> = HashMap::new();
    let mut y_counts: HashMap<usize, usize> = HashMap::new();

    for i in 0..x_bins.len().min(y_bins.len()) {
        *joint_counts.entry((x_bins[i], y_bins[i])).or_insert(0) += 1;
        *x_counts.entry(x_bins[i]).or_insert(0) += 1;
        *y_counts.entry(y_bins[i]).or_insert(0) += 1;
    }

    let mut mi = 0.0;
    for ((x_bin, y_bin), joint_count) in joint_counts {
        let p_xy = joint_count as f64 / n;
        let p_x = *x_counts.get(&x_bin).unwrap_or(&1) as f64 / n;
        let p_y = *y_counts.get(&y_bin).unwrap_or(&1) as f64 / n;
        if p_xy > 0.0 && p_x > 0.0 && p_y > 0.0 {
            mi += p_xy * (p_xy / (p_x * p_y)).ln();
        }
    }
    mi.max(0.0)
}

fn normalize_mutual_information(mi: f64, x_bins: &[usize], y_bins: &[usize]) -> f64 {
    let n = x_bins.len().min(y_bins.len()) as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mut x_counts: HashMap<usize, usize> = HashMap::new();
    let mut y_counts: HashMap<usize, usize> = HashMap::new();
    for i in 0..x_bins.len().min(y_bins.len()) {
        *x_counts.entry(x_bins[i]).or_insert(0) += 1;
        *y_counts.entry(y_bins[i]).or_insert(0) += 1;
    }
    let mut hx = 0.0;
    for (_, count) in x_counts {
        let p = count as f64 / n;
        if p > 0.0 {
            hx -= p * p.ln();
        }
    }
    let mut hy = 0.0;
    for (_, count) in y_counts {
        let p = count as f64 / n;
        if p > 0.0 {
            hy -= p * p.ln();
        }
    }
    let max_h = hx.max(hy);
    if max_h > 0.0 {
        (mi / max_h).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn compute_cramers_v(
    state_labels: &[String],
    economic_labels: &[String],
) -> f64 {
    let n = state_labels.len().min(economic_labels.len()) as f64;
    if n < 2.0 {
        return 0.0;
    }

    let mut joint_counts: HashMap<(String, String), usize> = HashMap::new();
    let mut state_counts: HashMap<String, usize> = HashMap::new();
    let mut econ_counts: HashMap<String, usize> = HashMap::new();

    for i in 0..state_labels.len().min(economic_labels.len()) {
        let s = state_labels[i].clone();
        let e = economic_labels[i].clone();
        *joint_counts.entry((s.clone(), e.clone())).or_insert(0) += 1;
        *state_counts.entry(s).or_insert(0) += 1;
        *econ_counts.entry(e).or_insert(0) += 1;
    }

    // Chi-squared
    let mut chi2 = 0.0;
    for ((s, e), observed) in &joint_counts {
        let expected = (*state_counts.get(s).unwrap_or(&1) as f64)
            * (*econ_counts.get(e).unwrap_or(&1) as f64)
            / n;
        if expected > 0.0 {
            chi2 += (*observed as f64 - expected).powi(2) / expected;
        }
    }

    let r = state_counts.len() as f64;
    let c = econ_counts.len() as f64;
    let min_dim = (r - 1.0).min(c - 1.0);

    if min_dim > 0.0 && n > 0.0 {
        (chi2 / (n * min_dim)).sqrt().clamp(0.0, 1.0)
    } else {
        0.0
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

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}

fn compute_cross_matrix(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
) -> Vec<CrossMatrixCell> {
    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let mut cells: HashMap<(String, String), Vec<(f64, f64, f64)>> = HashMap::new();

    for (index, regime) in regimes.iter().enumerate() {
        let Some(current_close) = close_by_date.get(&regime.date) else {
            continue;
        };
        if *current_close <= 0.0 {
            continue;
        }

        let state_label = normalize_regime_label(&regime.regime_label);
        let econ_label = classify_economic_state(regime.liquidity_score, regime.risk_score, scope_str);

        let forward_20_close = regimes
            .get(index + 20)
            .and_then(|r| close_by_date.get(&r.date));
        let forward_60_close = regimes
            .get(index + 60)
            .and_then(|r| close_by_date.get(&r.date));

        let ret_20 = forward_20_close.map(|c| (c - current_close) / current_close);
        let ret_60 = forward_60_close.map(|c| (c - current_close) / current_close);

        let forward_closes: Vec<f64> = (1..=20)
            .filter_map(|offset| {
                regimes
                    .get(index + offset)
                    .and_then(|r| close_by_date.get(&r.date))
            })
            .copied()
            .collect();

        let max_dd = if forward_closes.len() >= 10 {
            calculate_max_drawdown(*current_close, &forward_closes)
        } else {
            0.0
        };

        if let (Some(r20), Some(r60)) = (ret_20, ret_60) {
            cells
                .entry((state_label.clone(), econ_label.clone()))
                .or_default()
                .push((r20, r60, max_dd));
        }
    }

    let mut result = Vec::new();
    for ((state, econ), data) in cells {
        if data.is_empty() {
            continue;
        }
        let count = data.len();
        let mut rets_20: Vec<f64> = data.iter().map(|(r, _, _)| *r).collect();
        let mut rets_60: Vec<f64> = data.iter().map(|(_, r, _)| *r).collect();
        let mut dds: Vec<f64> = data.iter().map(|(_, _, dd)| *dd).collect();

        rets_20.sort_by(|a, b| a.partial_cmp(b).unwrap());
        rets_60.sort_by(|a, b| a.partial_cmp(b).unwrap());
        dds.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mean_20 = rets_20.iter().sum::<f64>() / count as f64;
        let mean_60 = rets_60.iter().sum::<f64>() / count as f64;
        let win_rate = rets_20.iter().filter(|r| **r > 0.0).count() as f64 / count as f64;

        let variance = rets_20.iter().map(|r| (r - mean_20).powi(2)).sum::<f64>() / count as f64;
        let sharpe = if variance > 0.0 {
            (mean_20 * 12.0) / (variance.sqrt() * (12.0_f64).sqrt())
        } else {
            0.0
        };

        result.push(CrossMatrixCell {
            state_regime: state,
            economic_regime: econ,
            count,
            pct: count as f64 / regimes.len() as f64,
            fwd_ret_20d_mean: mean_20,
            fwd_ret_60d_mean: mean_60,
            sharpe,
            max_dd_median: percentile(&dds, 0.50),
            win_rate,
        });
    }

    result
}

fn compute_economic_separation(
    economic_labels: &[String],
    close_by_date: &HashMap<NaiveDate, f64>,
    dates: &[NaiveDate],
) -> f64 {
    let mut regime_data: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

    for (index, date) in dates.iter().enumerate() {
        let Some(current_close) = close_by_date.get(date) else {
            continue;
        };
        if *current_close <= 0.0 {
            continue;
        }

        let label = economic_labels.get(index).cloned().unwrap_or_default();

        let forward_60_close = dates
            .get(index + 60)
            .and_then(|d| close_by_date.get(d));
        let ret_60 = forward_60_close.map(|c| (c - current_close) / current_close);

        let forward_closes: Vec<f64> = (1..=20)
            .filter_map(|offset| {
                dates.get(index + offset).and_then(|d| close_by_date.get(d))
            })
            .copied()
            .collect();

        let max_dd = if forward_closes.len() >= 10 {
            calculate_max_drawdown(*current_close, &forward_closes)
        } else {
            0.0
        };

        if let Some(r60) = ret_60 {
            regime_data.entry(label).or_default().push((r60, max_dd));
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
        let mut rets: Vec<f64> = data.iter().map(|(r, _)| *r).collect();
        let mut dds: Vec<f64> = data.iter().map(|(_, dd)| *dd).collect();
        rets.sort_by(|a, b| a.partial_cmp(b).unwrap());
        dds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean_ret = rets.iter().sum::<f64>() / rets.len() as f64;
        let median_dd = percentile(&dds, 0.50);
        let win_rate = rets.iter().filter(|r| **r > 0.0).count() as f64 / rets.len() as f64;
        stats.insert(regime_key.clone(), (mean_ret, median_dd, win_rate));
    }

    let states = ["unfavorable", "neutral", "favorable"];
    let ideal_ranks: HashMap<&str, f64> = [
        ("unfavorable", 3.0),
        ("neutral", 2.0),
        ("favorable", 1.0),
    ]
    .into_iter()
    .collect();

    let mut return_score = 50.0;
    let mut drawdown_score = 50.0;
    let mut winrate_score = 50.0;

    {
        let mut values: Vec<(String, f64)> = states
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

    {
        let mut values: Vec<(String, f64)> = states
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

    {
        let mut values: Vec<(String, f64)> = states
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

fn run_window_analysis(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    from: NaiveDate,
    to: NaiveDate,
    label: &str,
) -> Option<StabilityWindowResult> {
    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let window_regimes: Vec<_> = regimes
        .iter()
        .filter(|r| r.date >= from && r.date <= to)
        .collect();

    if window_regimes.len() < 30 {
        return None;
    }

    let state_labels: Vec<String> = window_regimes
        .iter()
        .map(|r| normalize_regime_label(&r.regime_label))
        .collect();
    let economic_labels: Vec<String> = window_regimes
        .iter()
        .map(|r| classify_economic_state(r.liquidity_score, r.risk_score, scope_str))
        .collect();

    let state_bins = discretize(
        &state_labels.iter().map(|s| match s.as_str() {
            "riskon" => 2.0,
            "neutral" => 1.0,
            "riskoff" => 0.0,
            _ => 1.0,
        }).collect::<Vec<_>>(),
        3,
    );
    let econ_bins = discretize(
        &economic_labels.iter().map(|s| match s.as_str() {
            "favorable" => 2.0,
            "neutral" => 1.0,
            "unfavorable" => 0.0,
            _ => 1.0,
        }).collect::<Vec<_>>(),
        3,
    );

    let mi_raw = mutual_information(&state_bins, &econ_bins);
    let mi = normalize_mutual_information(mi_raw, &state_bins, &econ_bins);
    let cramer_v = compute_cramers_v(&state_labels, &economic_labels);

    let dates: Vec<NaiveDate> = window_regimes.iter().map(|r| r.date).collect();
    let separation = compute_economic_separation(&economic_labels, &close_by_date, &dates);

    Some(StabilityWindowResult {
        window_label: label.to_string(),
        window_from: from,
        window_to: to,
        economic_separation: separation,
        cramer_v,
        mutual_information: mi,
    })
}

pub fn compute_dual_layer_validation(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<DualLayerValidationReport> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let total_days = regimes.len();
    let window_from = regimes.first().map(|r| r.date).unwrap_or(bars[0].date);
    let window_to = regimes.last().map(|r| r.date).unwrap_or(bars[bars.len() - 1].date);

    // TASK-030C: Cross-Matrix Economic Audit
    let cross_matrix = compute_cross_matrix(regimes, bars, scope_str);

    // TASK-030B: Mutual Information / Cramer's V
    let state_labels: Vec<String> = regimes
        .iter()
        .map(|r| normalize_regime_label(&r.regime_label))
        .collect();
    let economic_labels: Vec<String> = regimes
        .iter()
        .map(|r| classify_economic_state(r.liquidity_score, r.risk_score, scope_str))
        .collect();

    let state_bins = discretize(
        &state_labels.iter().map(|s| match s.as_str() {
            "riskon" => 2.0,
            "neutral" => 1.0,
            "riskoff" => 0.0,
            _ => 1.0,
        }).collect::<Vec<_>>(),
        3,
    );
    let econ_bins = discretize(
        &economic_labels.iter().map(|s| match s.as_str() {
            "favorable" => 2.0,
            "neutral" => 1.0,
            "unfavorable" => 0.0,
            _ => 1.0,
        }).collect::<Vec<_>>(),
        3,
    );

    let mi_raw = mutual_information(&state_bins, &econ_bins);
    let mutual_information = normalize_mutual_information(mi_raw, &state_bins, &econ_bins);
    let cramer_v = compute_cramers_v(&state_labels, &economic_labels);

    // Orthogonality pass if MI < 0.20 and Cramer's V < 0.30
    let orthogonality_pass = mutual_information < 0.20 && cramer_v < 0.30;

    // TASK-030D: Stability Audit across windows
    let mut stability_results = Vec::new();

    // Window 1: first third
    let mid1 = window_from + chrono::Duration::days((total_days as i64) / 3);
    let mid2 = window_from + chrono::Duration::days((2 * total_days as i64) / 3);

    if let Some(result) = run_window_analysis(regimes, bars, scope_str, window_from, mid1, "early") {
        stability_results.push(result);
    }
    if let Some(result) = run_window_analysis(regimes, bars, scope_str, mid1, mid2, "mid") {
        stability_results.push(result);
    }
    if let Some(result) = run_window_analysis(regimes, bars, scope_str, mid2, window_to, "late") {
        stability_results.push(result);
    }

    let validation_status = if orthogonality_pass {
        "pass".to_string()
    } else {
        "fail".to_string()
    };

    Some(DualLayerValidationReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        cross_matrix,
        mutual_information,
        cramer_v,
        orthogonality_pass,
        stability_results,
        validation_status,
    })
}

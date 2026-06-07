use chrono::NaiveDate;
use core_domain::{
    DailyBar, EconomicAttributionReport, FactorAttribution, MarketRegimeSnapshot,
};
use std::collections::HashMap;

// ============================================================
// TASK-028A: Economic Attribution Audit
// Determines which raw factor score truly predicts future returns.
// ============================================================

/// Compute Pearson correlation coefficient
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

/// Compute Spearman rank correlation
fn spearman_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    let rank = |v: &[f64]| {
        let mut indexed: Vec<(usize, f64)> = v.iter().enumerate().map(|(i, &val)| (i, val)).collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        let mut ranks = vec![0.0; v.len()];
        let mut i = 0;
        while i < indexed.len() {
            let mut j = i;
            while j < indexed.len() && (indexed[j].1 - indexed[i].1).abs() < f64::EPSILON {
                j += 1;
            }
            let avg_rank = (i + j + 1) as f64 / 2.0;
            for k in i..j {
                ranks[indexed[k].0] = avg_rank;
            }
            i = j;
        }
        ranks
    };

    let x_ranks = rank(x);
    let y_ranks = rank(y);
    pearson_correlation(&x_ranks, &y_ranks)
}

/// Discretize continuous values into bins for mutual information
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

/// Compute mutual information between two discrete distributions
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

fn compute_factor_attribution(
    factor_scores: &[f64],
    returns_20d: &[f64],
    returns_60d: &[f64],
    regime_labels: &[String],
    factor_name: &str,
) -> FactorAttribution {
    let pearson_20 = pearson_correlation(factor_scores, returns_20d);
    let pearson_60 = pearson_correlation(factor_scores, returns_60d);
    let spearman_20 = spearman_correlation(factor_scores, returns_20d);
    let spearman_60 = spearman_correlation(factor_scores, returns_60d);

    // Mutual information with 5 bins
    let factor_bins = discretize(factor_scores, 5);
    let ret20_bins = discretize(returns_20d, 5);
    let ret60_bins = discretize(returns_60d, 5);

    let mi_20_raw = mutual_information(&factor_bins, &ret20_bins);
    let mi_60_raw = mutual_information(&factor_bins, &ret60_bins);
    let mi_20 = normalize_mutual_information(mi_20_raw, &factor_bins, &ret20_bins);
    let mi_60 = normalize_mutual_information(mi_60_raw, &factor_bins, &ret60_bins);

    // Per-regime correlation
    let mut per_regime_corr: HashMap<String, f64> = HashMap::new();
    let regimes = ["risk_on", "risk_off", "neutral"];
    for regime in regimes {
        let mut regime_scores = Vec::new();
        let mut regime_returns = Vec::new();
        for i in 0..factor_scores.len().min(returns_60d.len()).min(regime_labels.len()) {
            if regime_labels[i].eq_ignore_ascii_case(regime) {
                regime_scores.push(factor_scores[i]);
                regime_returns.push(returns_60d[i]);
            }
        }
        if regime_scores.len() >= 5 {
            let corr = pearson_correlation(&regime_scores, &regime_returns);
            per_regime_corr.insert(regime.to_string(), corr);
        }
    }

    FactorAttribution {
        factor_name: factor_name.to_string(),
        pearson_corr_20d: pearson_20,
        pearson_corr_60d: pearson_60,
        spearman_corr_20d: spearman_20,
        spearman_corr_60d: spearman_60,
        mutual_information_20d: mi_20,
        mutual_information_60d: mi_60,
        per_regime_corr,
    }
}

pub fn compute_economic_attribution(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<EconomicAttributionReport> {
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

    // Prepare aligned data
    let mut trend_scores = Vec::new();
    let mut risk_scores = Vec::new();
    let mut liquidity_scores = Vec::new();
    let mut returns_20d = Vec::new();
    let mut returns_60d = Vec::new();
    let mut regime_labels = Vec::new();

    for (index, regime) in regimes_filtered.iter().enumerate() {
        let current_close = close_by_date.get(&regime.date).copied()?;
        if current_close <= 0.0 {
            continue;
        }

        let ret_20 = regimes_filtered
            .get(index + 20)
            .and_then(|r| close_by_date.get(&r.date))
            .map(|c| (c - current_close) / current_close);
        let ret_60 = regimes_filtered
            .get(index + 60)
            .and_then(|r| close_by_date.get(&r.date))
            .map(|c| (c - current_close) / current_close);

        if let (Some(r20), Some(r60)) = (ret_20, ret_60) {
            trend_scores.push(regime.trend_score);
            risk_scores.push(regime.risk_score);
            liquidity_scores.push(regime.liquidity_score);
            returns_20d.push(r20);
            returns_60d.push(r60);
            regime_labels.push(regime.regime_label.clone());
        }
    }

    if trend_scores.len() < 10 {
        return None;
    }

    let trend_attr = compute_factor_attribution(
        &trend_scores,
        &returns_20d,
        &returns_60d,
        &regime_labels,
        "trend",
    );
    let risk_attr = compute_factor_attribution(
        &risk_scores,
        &returns_20d,
        &returns_60d,
        &regime_labels,
        "risk",
    );
    let liquidity_attr = compute_factor_attribution(
        &liquidity_scores,
        &returns_20d,
        &returns_60d,
        &regime_labels,
        "liquidity",
    );

    // Determine dominant factor by mutual information (60d)
    let mut attributions = vec![trend_attr, risk_attr, liquidity_attr];
    attributions.sort_by(|a, b| {
        b.mutual_information_60d
            .partial_cmp(&a.mutual_information_60d)
            .unwrap()
    });

    let dominant = attributions[0].factor_name.clone();

    // Check divergence: does the alignment-best factor differ from economic-best?
    // For CN, Trend has best alignment; for HK, Risk has best alignment
    // If dominant != the factor that had best alignment, we have divergence
    let alignment_best = match scope_str {
        "CN" => "trend",
        "HK" => "risk",
        _ => "trend",
    };
    let divergence = dominant != alignment_best;

    Some(EconomicAttributionReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        factor_attributions: attributions,
        dominant_factor: dominant,
        economic_vs_alignment_divergence: divergence,
    })
}

use chrono::NaiveDate;
use core_domain::{
    CounterfactualReport, CounterfactualVariant, DailyBar, FactorAlignment,
    FactorAlignmentReport, FalseNegativeBreakdown, FalsePositiveBreakdown,
    MarketRegimeSnapshot, RegimeInformationScore,
};
use std::collections::HashMap;

// ============================================================
// TASK-026: Macro Factor Alignment Audit
// ============================================================
// 026A: Per-factor alignment (F1 + information score)
// 026B: False Positive / False Negative breakdown
// 026C: Counterfactual replay (7 variants)
// ============================================================

// ------------------------------------------------------------------
// Shared helpers (from state_alignment.rs)
// ------------------------------------------------------------------

fn compute_rolling_drawdown(bars: &[DailyBar]) -> Vec<(NaiveDate, f64)> {
    let mut result = Vec::new();
    let mut recent_high = 0.0;
    for bar in bars {
        if bar.high > recent_high {
            recent_high = bar.high;
        }
        if recent_high > 0.0 {
            let dd = ((bar.close - recent_high) / recent_high * 100.0).clamp(-100.0, 0.0);
            result.push((bar.date, dd));
        } else {
            result.push((bar.date, 0.0));
        }
    }
    result
}

fn sma(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut result = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        if i + 1 < period {
            result.push(None);
        } else {
            let window = &values[i + 1 - period..=i];
            let avg = window.iter().sum::<f64>() / period as f64;
            result.push(Some(avg));
        }
    }
    result
}

fn compute_market_states(bars: &[DailyBar]) -> HashMap<NaiveDate, (f64, bool)> {
    let mut result = HashMap::new();
    if bars.is_empty() {
        return result;
    }
    let drawdowns = compute_rolling_drawdown(bars);
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let ma20 = sma(&closes, 20);
    let ma60 = sma(&closes, 60);
    for (i, bar) in bars.iter().enumerate() {
        let dd = drawdowns.get(i).map(|(_, v)| *v).unwrap_or(0.0);
        let is_uptrend = if let (Some(m20), Some(m60)) = (ma20[i], ma60[i]) {
            bar.close > m20 && m20 > m60
        } else {
            false
        };
        result.insert(bar.date, (dd, is_uptrend));
    }
    result
}

fn binary_metrics(tp: usize, fp: usize, fn_: usize) -> (f64, f64, f64) {
    let precision = if tp + fp > 0 {
        tp as f64 / (tp + fp) as f64
    } else {
        0.0
    };
    let recall = if tp + fn_ > 0 {
        tp as f64 / (tp + fn_) as f64
    } else {
        0.0
    };
    let f1 = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    (precision, recall, f1)
}

fn compute_information_score(binary_labels: &[bool]) -> RegimeInformationScore {
    let total = binary_labels.len() as f64;
    if total == 0.0 {
        return RegimeInformationScore {
            entropy: 0.0,
            normalized_entropy: 0.0,
            effective_states: 0.0,
        };
    }
    let true_count = binary_labels.iter().filter(|&&x| x).count() as f64;
    let false_count = total - true_count;
    let p_true = true_count / total;
    let p_false = false_count / total;
    let entropy = -[p_true, p_false]
        .iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| p * p.log2())
        .sum::<f64>();
    let max_entropy = 1.0; // log2(2)
    let normalized_entropy = (entropy / max_entropy).clamp(0.0, 1.0);
    let effective_states = 2.0f64.powf(entropy);
    RegimeInformationScore {
        entropy,
        normalized_entropy,
        effective_states,
    }
}

// ------------------------------------------------------------------
// TASK-026A: Factor Alignment Audit
// ------------------------------------------------------------------

fn compute_single_factor_alignment(
    factor_name: &str,
    factor_values: &[(NaiveDate, f64)],
    market_states: &HashMap<NaiveDate, (f64, bool)>,
    risk_threshold: f64,
    uptrend_threshold: f64,
) -> FactorAlignment {
    let mut dd10_tp = 0usize;
    let mut dd10_fp = 0usize;
    let mut dd10_fn = 0usize;
    let mut dd20_tp = 0usize;
    let mut dd20_fp = 0usize;
    let mut dd20_fn = 0usize;
    let mut dd30_tp = 0usize;
    let mut dd30_fp = 0usize;
    let mut dd30_fn = 0usize;
    let mut uptrend_tp = 0usize;
    let mut uptrend_fp = 0usize;
    let mut uptrend_fn = 0usize;
    let mut binary_labels = Vec::new();

    for (date, score) in factor_values {
        let (dd, is_uptrend) = market_states.get(date).copied().unwrap_or((0.0, false));
        let is_risk = *score < risk_threshold;
        binary_labels.push(is_risk);

        // DD10
        let is_dd10 = dd < -10.0;
        match (is_risk, is_dd10) {
            (true, true) => dd10_tp += 1,
            (true, false) => dd10_fp += 1,
            (false, true) => dd10_fn += 1,
            _ => {}
        }

        // DD20
        let is_dd20 = dd < -20.0;
        match (is_risk, is_dd20) {
            (true, true) => dd20_tp += 1,
            (true, false) => dd20_fp += 1,
            (false, true) => dd20_fn += 1,
            _ => {}
        }

        // DD30
        let is_dd30 = dd < -30.0;
        match (is_risk, is_dd30) {
            (true, true) => dd30_tp += 1,
            (true, false) => dd30_fp += 1,
            (false, true) => dd30_fn += 1,
            _ => {}
        }

        // Uptrend
        let is_predicted_uptrend = *score >= uptrend_threshold;
        match (is_predicted_uptrend, is_uptrend) {
            (true, true) => uptrend_tp += 1,
            (true, false) => uptrend_fp += 1,
            (false, true) => uptrend_fn += 1,
            _ => {}
        }
    }

    let (_, _, dd10_f1) = binary_metrics(dd10_tp, dd10_fp, dd10_fn);
    let (_, _, dd20_f1) = binary_metrics(dd20_tp, dd20_fp, dd20_fn);
    let (_, _, dd30_f1) = binary_metrics(dd30_tp, dd30_fp, dd30_fn);
    let (_, _, uptrend_f1) = binary_metrics(uptrend_tp, uptrend_fp, uptrend_fn);

    let info = compute_information_score(&binary_labels);

    FactorAlignment {
        factor_name: factor_name.to_string(),
        dd10_f1,
        dd20_f1,
        dd30_f1,
        uptrend_f1,
        information_score: info,
    }
}

pub fn compute_factor_alignment(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> Option<FactorAlignmentReport> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let market_states = compute_market_states(bars);

    // Filter regimes to dates with bar data
    let regimes_filtered: Vec<_> = regimes
        .iter()
        .filter(|r| market_states.contains_key(&r.date))
        .collect();

    if regimes_filtered.is_empty() {
        return None;
    }

    let total_days = regimes_filtered.len();
    let window_from = regimes_filtered.first().map(|r| r.date).unwrap_or(bars[0].date);
    let window_to = regimes_filtered.last().map(|r| r.date).unwrap_or(bars[bars.len() - 1].date);
    let scope = regimes_filtered.first().map(|r| r.market.clone()).unwrap_or_default();

    let trend_values: Vec<(NaiveDate, f64)> = regimes_filtered.iter().map(|r| (r.date, r.trend_score)).collect();
    let risk_values: Vec<(NaiveDate, f64)> = regimes_filtered.iter().map(|r| (r.date, r.risk_score)).collect();
    let liquidity_values: Vec<(NaiveDate, f64)> = regimes_filtered.iter().map(|r| (r.date, r.liquidity_score)).collect();

    let trend_alignment = compute_single_factor_alignment("trend", &trend_values, &market_states, 40.0, 60.0);
    let risk_alignment = compute_single_factor_alignment("risk", &risk_values, &market_states, 40.0, 55.0);
    let liquidity_alignment = compute_single_factor_alignment("liquidity", &liquidity_values, &market_states, 50.0, 50.0);

    Some(FactorAlignmentReport {
        scope,
        window_from,
        window_to,
        total_days,
        trend_alignment,
        risk_alignment,
        liquidity_alignment,
    })
}

// ------------------------------------------------------------------
// TASK-026B: False Positive / False Negative Breakdown
// ------------------------------------------------------------------

pub fn compute_false_positive_breakdown(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> Option<FalsePositiveBreakdown> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let market_states = compute_market_states(bars);

    let mut total_riskoff_days = 0usize;
    let mut false_positive_days = 0usize;
    let mut caused_by_trend_only = 0usize;
    let mut caused_by_risk_only = 0usize;
    let mut caused_by_both = 0usize;

    for regime in regimes {
        let (dd, _) = market_states.get(&regime.date).copied().unwrap_or((0.0, false));
        let is_dd20 = dd < -20.0;

        if regime.regime_label.eq_ignore_ascii_case("risk_off") {
            total_riskoff_days += 1;
            if !is_dd20 {
                false_positive_days += 1;
                let trend_triggered = regime.trend_score < 40.0;
                let risk_triggered = regime.risk_score < 40.0;
                match (trend_triggered, risk_triggered) {
                    (true, false) => caused_by_trend_only += 1,
                    (false, true) => caused_by_risk_only += 1,
                    (true, true) => caused_by_both += 1,
                    _ => {}
                }
            }
        }
    }

    Some(FalsePositiveBreakdown {
        total_riskoff_days,
        false_positive_days,
        caused_by_trend_only,
        caused_by_risk_only,
        caused_by_both,
        risk_fp_by_vix: 0,        // Would need macro_snapshot data
        risk_fp_by_dollar_index: 0,
        risk_fp_by_both_macro: 0,
    })
}

pub fn compute_false_negative_breakdown(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> Option<FalseNegativeBreakdown> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let market_states = compute_market_states(bars);

    let mut total_dd20_days = 0usize;
    let mut missed_by_trend = 0usize;
    let mut missed_by_risk = 0usize;
    let mut missed_by_liquidity = 0usize;
    let mut missed_by_all = 0usize;

    for regime in regimes {
        let (dd, _) = market_states.get(&regime.date).copied().unwrap_or((0.0, false));
        let is_dd20 = dd < -20.0;

        if is_dd20 {
            total_dd20_days += 1;
            let is_riskoff = regime.regime_label.eq_ignore_ascii_case("risk_off");
            if !is_riskoff {
                let trend_ok = regime.trend_score >= 40.0;
                let risk_ok = regime.risk_score >= 40.0;
                let liquidity_ok = regime.liquidity_score >= 50.0;
                match (trend_ok, risk_ok, liquidity_ok) {
                    (false, true, true) => missed_by_trend += 1,
                    (true, false, true) => missed_by_risk += 1,
                    (true, true, false) => missed_by_liquidity += 1,
                    (true, true, true) => missed_by_all += 1,
                    _ => {}
                }
            }
        }
    }

    Some(FalseNegativeBreakdown {
        total_dd20_days,
        missed_by_trend,
        missed_by_risk,
        missed_by_liquidity,
        missed_by_all,
    })
}

// ------------------------------------------------------------------
// TASK-026C: Counterfactual Replay
// ------------------------------------------------------------------

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
        "trend_and_risk" => {
            if trend_score >= 60.0 && risk_score >= 55.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 || risk_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        "symmetric_and" => {
            if trend_score >= 60.0 && risk_score >= 55.0 {
                "risk_on".to_string()
            } else if trend_score < 40.0 && risk_score < 40.0 {
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
            if risk_score >= 60.0 {
                "risk_on".to_string()
            } else if risk_score < 40.0 {
                "risk_off".to_string()
            } else {
                "neutral".to_string()
            }
        }
        _ => "neutral".to_string(),
    }
}

fn evaluate_counterfactual(
    regimes: &[MarketRegimeSnapshot],
    market_states: &HashMap<NaiveDate, (f64, bool)>,
    variant: &str,
) -> CounterfactualVariant {
    let mut labels = Vec::new();
    let mut regime_counts: HashMap<String, usize> = HashMap::new();

    for regime in regimes {
        let label = classify_counterfactual(regime.trend_score, regime.risk_score, regime.liquidity_score, variant);
        labels.push((regime.date, label.clone()));
        *regime_counts.entry(label).or_insert(0) += 1;
    }

    let total = labels.len() as f64;
    let regime_distribution: HashMap<String, f64> = regime_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v as f64 / total))
        .collect();

    // Compute alignment (same as state_alignment: avg of DD20 F1 + RiskOn F1)
    let mut dd20_tp = 0usize;
    let mut dd20_fp = 0usize;
    let mut dd20_fn = 0usize;
    let mut uptrend_tp = 0usize;
    let mut uptrend_fp = 0usize;
    let mut uptrend_fn = 0usize;

    for (date, label) in &labels {
        let (dd, is_uptrend) = market_states.get(date).copied().unwrap_or((0.0, false));
        let is_riskoff = label.eq_ignore_ascii_case("risk_off");
        let is_riskon = label.eq_ignore_ascii_case("risk_on");

        if is_riskoff {
            if dd < -20.0 {
                dd20_tp += 1;
            } else {
                dd20_fp += 1;
            }
        } else if dd < -20.0 {
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

    let (_, _, dd20_f1) = binary_metrics(dd20_tp, dd20_fp, dd20_fn);
    let (_, _, uptrend_f1) = binary_metrics(uptrend_tp, uptrend_fp, uptrend_fn);
    let alignment = (dd20_f1 + uptrend_f1) / 2.0;

    // Information score
    let mut entropy = 0.0;
    for (_, count) in &regime_counts {
        let p = *count as f64 / total;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    let max_entropy = (3.0f64).log2();
    let information = (entropy / max_entropy).clamp(0.0, 1.0);

    CounterfactualVariant {
        name: variant.to_string(),
        regime_distribution,
        alignment,
        information,
    }
}

pub fn compute_counterfactual_replay(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> Option<CounterfactualReport> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let market_states = compute_market_states(bars);
    let regimes_filtered: Vec<_> = regimes
        .iter()
        .filter(|r| market_states.contains_key(&r.date))
        .cloned()
        .collect();

    if regimes_filtered.is_empty() {
        return None;
    }

    let total_days = regimes_filtered.len();
    let window_from = regimes_filtered.first().map(|r| r.date).unwrap_or(bars[0].date);
    let window_to = regimes_filtered.last().map(|r| r.date).unwrap_or(bars[bars.len() - 1].date);
    let scope = regimes_filtered.first().map(|r| r.market.clone()).unwrap_or_default();

    let variants = vec![
        evaluate_counterfactual(&regimes_filtered, &market_states, "baseline"),
        evaluate_counterfactual(&regimes_filtered, &market_states, "trend_only"),
        evaluate_counterfactual(&regimes_filtered, &market_states, "risk_only"),
        evaluate_counterfactual(&regimes_filtered, &market_states, "trend_and_risk"),
        evaluate_counterfactual(&regimes_filtered, &market_states, "symmetric_and"),
        evaluate_counterfactual(&regimes_filtered, &market_states, "trend_dominant"),
        evaluate_counterfactual(&regimes_filtered, &market_states, "macro_dominant"),
    ];

    Some(CounterfactualReport {
        scope,
        window_from,
        window_to,
        total_days,
        variants,
    })
}

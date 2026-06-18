use crate::common::apply_persistence;
use chrono::NaiveDate;
use core_domain::{
    DailyBar, MarketRegimeSnapshot, Wave8ComparisonPoint, Wave8RevalidationReport,
};
use std::collections::HashMap;

// ============================================================
// TASK-035A Phase 2: Wave 8 Revalidation
// Side-by-side comparison of 1d vs 10d persistence for:
// - State Alignment (TASK-025A)
// - Economic Separation (TASK-027)
// - Allocation Prototype (TASK-032)
// ============================================================

fn classify_raw_regime(trend_score: f64, risk_score: f64, liquidity_score: f64) -> String {
    if trend_score >= 60.0 && liquidity_score >= 50.0 && risk_score >= 55.0 {
        "risk_on".to_string()
    } else if trend_score < 40.0 || risk_score < 40.0 {
        "risk_off".to_string()
    } else {
        "neutral".to_string()
    }
}

fn rolling_mean(values: &[f64], index: usize, period: usize) -> Option<f64> {
    if index + 1 < period {
        return None;
    }
    let window = &values[index + 1 - period..=index];
    Some(window.iter().sum::<f64>() / period as f64)
}

fn compute_trend_score_from_bars(bars: &[DailyBar]) -> HashMap<NaiveDate, f64> {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let mut result = HashMap::new();
    for (index, bar) in bars.iter().enumerate() {
        let ma20 = rolling_mean(&closes, index, 20);
        let ma60 = rolling_mean(&closes, index, 60);
        let score = match (ma20, ma60) {
            (Some(ma20), Some(ma60)) if bar.close > ma20 && ma20 > ma60 => 85.0,
            (Some(ma20), Some(_)) if bar.close > ma20 => 65.0,
            (Some(_), Some(ma60)) if bar.close > ma60 => 50.0,
            (Some(_), Some(_)) => 25.0,
            _ => 50.0,
        };
        result.insert(bar.date, score);
    }
    result
}

fn compute_information(labels: &[String]) -> f64 {
    let total = labels.len() as f64;
    if total == 0.0 {
        return 0.0;
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for label in labels {
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

fn compute_alignment(
    labels: &[(NaiveDate, String)],
    close_by_date: &HashMap<NaiveDate, f64>,
) -> f64 {
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

fn compute_economic_separation(
    labels: &[(NaiveDate, String)],
    close_by_date: &HashMap<NaiveDate, f64>,
    _scope: &str,
) -> f64 {
    let mut risk_on_returns = Vec::new();
    let mut neutral_returns = Vec::new();
    let mut risk_off_returns = Vec::new();

    let dates: Vec<NaiveDate> = labels.iter().map(|(d, _)| *d).collect();

    for i in 0..labels.len().saturating_sub(20) {
        let (_date, label) = &labels[i];
        let current_close = close_by_date.get(&dates[i]).copied().unwrap_or(0.0);
        let future_close = close_by_date.get(&dates[i + 20]).copied().unwrap_or(0.0);
        let ret = if current_close > 0.0 {
            (future_close - current_close) / current_close * 100.0
        } else {
            0.0
        };

        match label.as_str() {
            "risk_on" => risk_on_returns.push(ret),
            "neutral" => neutral_returns.push(ret),
            "risk_off" => risk_off_returns.push(ret),
            _ => {}
        }
    }

    let avg_risk_on = if !risk_on_returns.is_empty() {
        risk_on_returns.iter().sum::<f64>() / risk_on_returns.len() as f64
    } else {
        0.0
    };
    let avg_risk_off = if !risk_off_returns.is_empty() {
        risk_off_returns.iter().sum::<f64>() / risk_off_returns.len() as f64
    } else {
        0.0
    };

    (avg_risk_on - avg_risk_off).abs()
}

fn run_backtest(
    dates: &[NaiveDate],
    returns: &[f64],
    allocations: &[f64],
) -> (f64, f64) {
    let n = dates.len().min(returns.len()).min(allocations.len());
    if n < 2 {
        return (0.0, 0.0);
    }

    let mut portfolio_value = 1.0;
    let mut peak = 1.0;
    let mut max_dd = 0.0;
    let mut daily_returns = Vec::new();

    for i in 0..n {
        let alloc = allocations[i].clamp(0.0, 1.0);
        let daily_ret = returns[i] * alloc;
        portfolio_value *= 1.0 + daily_ret;
        daily_returns.push(daily_ret);

        if portfolio_value > peak {
            peak = portfolio_value;
        }
        let dd = (peak - portfolio_value) / peak;
        if dd > max_dd {
            max_dd = dd;
        }
    }

    let years = n as f64 / 252.0;
    let cagr = if years > 0.0 && portfolio_value > 0.0 {
        portfolio_value.powf(1.0 / years) - 1.0
    } else {
        0.0
    };

    let mean_ret = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
    let variance = daily_returns
        .iter()
        .map(|r| (r - mean_ret).powi(2))
        .sum::<f64>()
        / daily_returns.len() as f64;
    let std_dev = variance.sqrt();
    let sharpe = if std_dev > 0.0 {
        mean_ret / std_dev * 252.0_f64.sqrt()
    } else {
        0.0
    };

    (cagr, sharpe)
}

fn compute_allocation_backtest(
    labels: &[String],
    dates: &[NaiveDate],
    returns: &[f64],
    strategy: &str,
) -> (f64, f64) {
    let allocations: Vec<f64> = labels
        .iter()
        .map(|l| match strategy {
            "baseline" => match l.as_str() {
                "risk_on" => 1.0,
                "neutral" => 0.5,
                "risk_off" => 0.0,
                _ => 0.5,
            },
            "state_only" => match l.as_str() {
                "risk_on" => 1.0,
                "neutral" => 0.5,
                "risk_off" => 0.0,
                _ => 0.5,
            },
            "dual_layer" => match l.as_str() {
                "risk_on" => 0.9,
                "neutral" => 0.5,
                "risk_off" => 0.1,
                _ => 0.5,
            },
            _ => 0.5,
        })
        .collect();

    run_backtest(dates, returns, &allocations)
}

fn compute_comparison_point(
    raw_labels: &[String],
    dates: &[NaiveDate],
    close_by_date: &HashMap<NaiveDate, f64>,
    daily_returns: &[f64],
    persistence_days: usize,
    scope: &str,
) -> Wave8ComparisonPoint {
    let persisted_labels = apply_persistence(raw_labels, persistence_days);

    let labels_with_dates: Vec<(NaiveDate, String)> = dates
        .iter()
        .zip(persisted_labels.iter())
        .map(|(d, l)| (*d, l.clone()))
        .collect();

    let alignment = compute_alignment(&labels_with_dates, close_by_date);
    let information = compute_information(&persisted_labels);
    let economic_separation = compute_economic_separation(&labels_with_dates, close_by_date, scope);

    let (state_cagr, state_sharpe) =
        compute_allocation_backtest(&persisted_labels, dates, daily_returns, "state_only");
    let (dual_cagr, dual_sharpe) =
        compute_allocation_backtest(&persisted_labels, dates, daily_returns, "dual_layer");
    let (baseline_cagr, baseline_sharpe) =
        compute_allocation_backtest(&persisted_labels, dates, daily_returns, "baseline");

    Wave8ComparisonPoint {
        persistence_days,
        alignment_score: alignment,
        information_score: information,
        economic_separation,
        state_only_cagr: state_cagr,
        state_only_sharpe: state_sharpe,
        dual_layer_cagr: dual_cagr,
        dual_layer_sharpe: dual_sharpe,
        baseline_cagr: baseline_cagr,
        baseline_sharpe: baseline_sharpe,
    }
}

pub fn compute_wave8_revalidation(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<Wave8RevalidationReport> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let regimes_filtered: Vec<_> = regimes
        .iter()
        .filter(|r| close_by_date.contains_key(&r.date))
        .collect();

    if regimes_filtered.len() < 30 {
        return None;
    }

    let total_days = regimes_filtered.len();
    let window_from = regimes_filtered.first().map(|r| r.date).unwrap_or(bars[0].date);
    let window_to = regimes_filtered.last().map(|r| r.date).unwrap_or(bars[bars.len() - 1].date);

    // For HK, recompute trend_score from actual bars to fix the HSI→HSCEI bug
    let fresh_trend_scores: Option<HashMap<NaiveDate, f64>> = if scope_str == "HK" {
        Some(compute_trend_score_from_bars(bars))
    } else {
        None
    };

    let raw_labels: Vec<String> = regimes_filtered
        .iter()
        .map(|r| {
            let trend = fresh_trend_scores
                .as_ref()
                .and_then(|m| m.get(&r.date))
                .copied()
                .unwrap_or(r.trend_score);
            classify_raw_regime(trend, r.risk_score, r.liquidity_score)
        })
        .collect();

    let dates: Vec<NaiveDate> = regimes_filtered.iter().map(|r| r.date).collect();

    let mut daily_returns = Vec::new();
    for i in 0..regimes_filtered.len().saturating_sub(1) {
        let current_close = close_by_date.get(&dates[i]).copied().unwrap_or(0.0);
        let next_close = close_by_date.get(&dates[i + 1]).copied().unwrap_or(0.0);
        let ret = if current_close > 0.0 {
            (next_close - current_close) / current_close
        } else {
            0.0
        };
        daily_returns.push(ret);
    }

    let p1 = compute_comparison_point(&raw_labels, &dates, &close_by_date, &daily_returns, 1, scope_str);
    let p10 = compute_comparison_point(&raw_labels, &dates, &close_by_date, &daily_returns, 10, scope_str);

    let conclusion = format!(
        "WAVE8_REVALIDATION: {} 1d vs 10d. Alignment: {:.3}→{:.3} ({}). Information: {:.3}→{:.3}. Economic Separation: {:.1}→{:.1}. State Only Sharpe: {:.2}→{:.2}.",
        scope_str,
        p1.alignment_score,
        p10.alignment_score,
        if p1.alignment_score > p10.alignment_score { "IMPROVED" } else { "DEGRADED" },
        p1.information_score,
        p10.information_score,
        p1.economic_separation,
        p10.economic_separation,
        p1.state_only_sharpe,
        p10.state_only_sharpe,
    );

    Some(Wave8RevalidationReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        comparisons: vec![p1, p10],
        conclusion,
    })
}

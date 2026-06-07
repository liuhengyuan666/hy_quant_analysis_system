use chrono::NaiveDate;
use core_domain::{
    DailyBar, MarketRegimeSnapshot, PersistenceFrontierPoint, PersistenceFrontierReport,
};
use std::collections::HashMap;

// ============================================================
// TASK-034: Persistence Frontier Audit
// Maps persistence days vs Alignment + Economic metrics.
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

fn apply_persistence(raw_labels: &[String], days: usize) -> Vec<String> {
    if days == 0 {
        return raw_labels.to_vec();
    }
    let mut persisted = Vec::with_capacity(raw_labels.len());
    let mut current_regime = "neutral".to_string();
    let mut streak = 0;

    for label in raw_labels {
        if label == &current_regime {
            streak += 1;
        } else {
            streak = 1;
            current_regime = label.clone();
        }

        if streak >= days {
            persisted.push(current_regime.clone());
        } else {
            if persisted.is_empty() {
                persisted.push("neutral".to_string());
            } else {
                persisted.push(persisted.last().unwrap().clone());
            }
        }
    }

    persisted
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

fn run_backtest(
    dates: &[NaiveDate],
    returns: &[f64],
    allocations: &[f64],
) -> (f64, f64, f64, f64, f64, f64) {
    // Returns: (cagr, sharpe, sortino, max_drawdown, turnover, final_value)
    let n = dates.len().min(returns.len()).min(allocations.len());
    if n < 2 {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
    }

    let mut portfolio_value = 1.0;
    let mut peak = 1.0;
    let mut max_dd = 0.0;
    let mut daily_returns = Vec::new();
    let mut negative_returns = Vec::new();
    let mut turnover_sum = 0.0;

    for i in 0..n {
        let alloc = allocations[i].clamp(0.0, 1.0);
        let daily_ret = returns[i] * alloc;
        portfolio_value *= 1.0 + daily_ret;
        daily_returns.push(daily_ret);
        if daily_ret < 0.0 {
            negative_returns.push(daily_ret);
        }

        if i > 0 {
            let prev_alloc = allocations[i - 1].clamp(0.0, 1.0);
            turnover_sum += (alloc - prev_alloc).abs();
        }

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

    let neg_variance = if negative_returns.is_empty() {
        0.0
    } else {
        negative_returns
            .iter()
            .map(|r| r.powi(2))
            .sum::<f64>()
            / negative_returns.len() as f64
    };
    let neg_std = neg_variance.sqrt();
    let sortino = if neg_std > 0.0 {
        mean_ret / neg_std * 252.0_f64.sqrt()
    } else {
        0.0
    };

    let turnover = if n > 1 {
        turnover_sum / (n - 1) as f64
    } else {
        0.0
    };

    (cagr, sharpe, sortino, max_dd, turnover, portfolio_value)
}

pub fn compute_persistence_frontier(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<PersistenceFrontierReport> {
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

    let raw_labels: Vec<String> = regimes_filtered
        .iter()
        .map(|r| classify_raw_regime(r.trend_score, r.risk_score, r.liquidity_score))
        .collect();

    let persistence_configs = vec![0, 1, 2, 3, 5, 7, 10];
    let mut points = Vec::new();

    // Pre-compute daily returns
    let mut daily_returns = Vec::new();
    for i in 0..regimes_filtered.len().saturating_sub(1) {
        let regime = regimes_filtered[i];
        let current_close = close_by_date.get(&regime.date).copied().unwrap_or(0.0);
        let next_close = regimes_filtered
            .get(i + 1)
            .and_then(|r| close_by_date.get(&r.date))
            .copied()
            .unwrap_or(0.0);
        let ret = if current_close > 0.0 {
            (next_close - current_close) / current_close
        } else {
            0.0
        };
        daily_returns.push(ret);
    }

    for days in persistence_configs {
        let persisted_labels = apply_persistence(&raw_labels, days);

        // Compute alignment and information
        let labels_with_dates: Vec<(NaiveDate, String)> = regimes_filtered
            .iter()
            .zip(persisted_labels.iter())
            .map(|(r, l)| (r.date, l.clone()))
            .collect();
        let alignment = compute_alignment(&labels_with_dates, &close_by_date);
        let information = compute_information(&labels_with_dates);

        // Compute backtest
        let allocations: Vec<f64> = persisted_labels
            .iter()
            .map(|l| match l.as_str() {
                "risk_on" => 1.0,
                "neutral" => 0.5,
                "risk_off" => 0.0,
                _ => 0.5,
            })
            .collect();

        let dates: Vec<NaiveDate> = regimes_filtered.iter().map(|r| r.date).collect();
        let (cagr, sharpe, sortino, max_dd, turnover, final_value) =
            run_backtest(&dates, &daily_returns, &allocations);

        points.push(PersistenceFrontierPoint {
            confirmation_days: days,
            alignment,
            information,
            cagr,
            sharpe,
            sortino,
            max_drawdown: max_dd,
            turnover,
            final_value,
        });
    }

    // Find optimal days based on Sharpe ratio
    let optimal = points
        .iter()
        .max_by(|a, b| a.sharpe.partial_cmp(&b.sharpe).unwrap())
        .map(|p| p.confirmation_days)
        .unwrap_or(0);

    let conclusion = if optimal == 0 {
        "no_persistence_optimal: immediate switching maximizes Sharpe ratio".to_string()
    } else {
        format!(
            "persistence_{}_optimal: {}-day confirmation maximizes Sharpe ratio",
            optimal, optimal
        )
    };

    Some(PersistenceFrontierReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        points,
        optimal_days: optimal,
        conclusion,
    })
}

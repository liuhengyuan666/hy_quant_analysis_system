use chrono::NaiveDate;
use core_domain::{
    DailyBar, MarketRegimeSnapshot, PersistenceStrategyResult, StateReturnAttribution,
    StateSignalDecompositionReport, TransitionAlpha,
};
use std::collections::HashMap;

// ============================================================
// TASK-033: State Signal Decomposition Audit
// Explains why State Layer produces strong investment returns.
// ============================================================

fn normalize_regime_label(label: &str) -> String {
    label.to_lowercase().replace("risk_on", "riskon").replace("risk_off", "riskoff")
}

fn classify_raw_regime(trend_score: f64, risk_score: f64, liquidity_score: f64) -> String {
    if trend_score >= 60.0 && liquidity_score >= 50.0 && risk_score >= 55.0 {
        "riskon".to_string()
    } else if trend_score < 40.0 || risk_score < 40.0 {
        "riskoff".to_string()
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

// TASK-033A: State Return Attribution
fn compute_regime_attributions(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> Vec<StateReturnAttribution> {
    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let mut regime_data: HashMap<String, Vec<(f64, f64, f64)>> = HashMap::new();
    // (daily_ret, ret_20, ret_60)

    for (index, regime) in regimes.iter().enumerate() {
        let Some(current_close) = close_by_date.get(&regime.date) else {
            continue;
        };
        if *current_close <= 0.0 {
            continue;
        }

        let label = normalize_regime_label(&regime.regime_label);

        let next_close = regimes
            .get(index + 1)
            .and_then(|r| close_by_date.get(&r.date));
        let daily_ret = next_close.map(|c| (c - current_close) / current_close).unwrap_or(0.0);

        let ret_20 = regimes
            .get(index + 20)
            .and_then(|r| close_by_date.get(&r.date))
            .map(|c| (c - current_close) / current_close);
        let ret_60 = regimes
            .get(index + 60)
            .and_then(|r| close_by_date.get(&r.date))
            .map(|c| (c - current_close) / current_close);

        if let (Some(r20), Some(r60)) = (ret_20, ret_60) {
            regime_data
                .entry(label)
                .or_default()
                .push((daily_ret, r20, r60));
        }
    }

    let mut result = Vec::new();
    for (regime, data) in regime_data {
        if data.is_empty() {
            continue;
        }
        let count = data.len();
        let daily_rets: Vec<f64> = data.iter().map(|(r, _, _)| *r).collect();
        let rets_20: Vec<f64> = data.iter().map(|(_, r, _)| *r).collect();
        let rets_60: Vec<f64> = data.iter().map(|(_, _, r)| *r).collect();

        let total_ret = daily_rets.iter().sum::<f64>();
        let avg_daily = total_ret / count as f64;
        let avg_20 = rets_20.iter().sum::<f64>() / count as f64;
        let avg_60 = rets_60.iter().sum::<f64>() / count as f64;
        let win_rate = rets_20.iter().filter(|r| **r > 0.0).count() as f64 / count as f64;

        let variance = daily_rets.iter().map(|r| (r - avg_daily).powi(2)).sum::<f64>() / count as f64;
        let sharpe = if variance > 0.0 {
            (avg_daily * 252.0_f64.sqrt()) / (variance.sqrt() * 252.0_f64.sqrt())
        } else {
            0.0
        };

        result.push(StateReturnAttribution {
            state: regime,
            count,
            pct: count as f64 / regimes.len() as f64,
            total_return_contribution: total_ret,
            avg_daily_return: avg_daily,
            avg_20d_return: avg_20,
            avg_60d_return: avg_60,
            win_rate,
            sharpe,
        });
    }

    result
}

// TASK-033B: Persistence Contribution Audit
fn compute_persistence_audit(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> Vec<PersistenceStrategyResult> {
    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let raw_labels: Vec<String> = regimes
        .iter()
        .map(|r| classify_raw_regime(r.trend_score, r.risk_score, r.liquidity_score))
        .collect();

    let persistence_configs = vec![0, 5, 10, 15];
    let mut results = Vec::new();

    for days in persistence_configs {
        let persisted_labels = apply_persistence(&raw_labels, days);

        let mut portfolio_value = 1.0;
        let mut peak = 1.0;
        let mut max_dd = 0.0;
        let mut daily_returns = Vec::new();
        let mut negative_returns = Vec::new();
        let mut turnover_sum = 0.0;

        for i in 0..regimes.len().saturating_sub(1) {
            let regime = &regimes[i];
            let Some(current_close) = close_by_date.get(&regime.date) else {
                continue;
            };

            let next_regime = &regimes[i + 1];
            let Some(next_close) = close_by_date.get(&next_regime.date) else {
                continue;
            };

            let alloc = match persisted_labels[i].as_str() {
                "riskon" => 1.0,
                "neutral" => 0.5,
                "riskoff" => 0.0,
                _ => 0.5,
            };

            let daily_ret = if *current_close > 0.0 {
                (next_close - current_close) / current_close * alloc
            } else {
                0.0
            };

            portfolio_value *= 1.0 + daily_ret;
            daily_returns.push(daily_ret);
            if daily_ret < 0.0 {
                negative_returns.push(daily_ret);
            }

            if i > 0 {
                let prev_alloc = match persisted_labels[i - 1].as_str() {
                    "riskon" => 1.0,
                    "neutral" => 0.5,
                    "riskoff" => 0.0,
                    _ => 0.5,
                };
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

        let n = daily_returns.len();
        let years = n as f64 / 252.0;
        let cagr = if years > 0.0 && portfolio_value > 0.0 {
            portfolio_value.powf(1.0 / years) - 1.0
        } else {
            0.0
        };

        let mean_ret = if n > 0 {
            daily_returns.iter().sum::<f64>() / n as f64
        } else {
            0.0
        };
        let variance = if n > 0 {
            daily_returns.iter().map(|r| (r - mean_ret).powi(2)).sum::<f64>() / n as f64
        } else {
            0.0
        };
        let std_dev = variance.sqrt();
        let sharpe = if std_dev > 0.0 {
            mean_ret / std_dev * 252.0_f64.sqrt()
        } else {
            0.0
        };

        let turnover = if n > 1 {
            turnover_sum / (n - 1) as f64
        } else {
            0.0
        };

        results.push(PersistenceStrategyResult {
            confirmation_days: days,
            cagr,
            sharpe,
            max_drawdown: max_dd,
            turnover,
            final_value: portfolio_value,
        });
    }

    results
}

// TASK-033C: Regime Transition Alpha
fn compute_transition_alpha(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> Vec<TransitionAlpha> {
    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let labels: Vec<String> = regimes
        .iter()
        .map(|r| normalize_regime_label(&r.regime_label))
        .collect();

    let mut transitions: HashMap<(String, String), Vec<(f64, f64, f64, f64)>> = HashMap::new();
    // (ret_20, ret_60, ret_120, max_dd)

    for i in 1..regimes.len() {
        let prev_label = &labels[i - 1];
        let curr_label = &labels[i];

        if prev_label == curr_label {
            continue;
        }

        let regime = &regimes[i];

        let Some(current_close) = close_by_date.get(&regime.date) else {
            continue;
        };
        if *current_close <= 0.0 {
            continue;
        }

        let ret_20 = regimes
            .get(i + 20)
            .and_then(|r| close_by_date.get(&r.date))
            .map(|c| (c - current_close) / current_close);
        let ret_60 = regimes
            .get(i + 60)
            .and_then(|r| close_by_date.get(&r.date))
            .map(|c| (c - current_close) / current_close);

        let forward_closes: Vec<f64> = (1..=20)
            .filter_map(|offset| {
                regimes
                    .get(i + offset)
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
            transitions
                .entry((prev_label.clone(), curr_label.clone()))
                .or_default()
                .push((r20, r60, 0.0, max_dd));
        }
    }

    let mut result = Vec::new();
    for ((from, to), data) in transitions {
        if data.is_empty() {
            continue;
        }
        let count = data.len();
        let mut rets_20: Vec<f64> = data.iter().map(|(r, _, _, _)| *r).collect();
        let mut rets_60: Vec<f64> = data.iter().map(|(_, r, _, _)| *r).collect();
        let mut dds: Vec<f64> = data.iter().map(|(_, _, _, dd)| *dd).collect();

        rets_20.sort_by(|a, b| a.partial_cmp(b).unwrap());
        rets_60.sort_by(|a, b| a.partial_cmp(b).unwrap());
        dds.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mean_20 = rets_20.iter().sum::<f64>() / count as f64;
        let mean_60 = rets_60.iter().sum::<f64>() / count as f64;
        let win_rate_20 = rets_20.iter().filter(|r| **r > 0.0).count() as f64 / count as f64;
        let win_rate_60 = rets_60.iter().filter(|r| **r > 0.0).count() as f64 / count as f64;

        result.push(TransitionAlpha {
            from_state: from,
            to_state: to,
            count,
            avg_20d_return: mean_20,
            avg_60d_return: mean_60,
            win_rate_20d: win_rate_20,
            win_rate_60d: win_rate_60,
            max_dd_median: percentile(&dds, 0.50),
        });
    }

    result
}

pub fn compute_state_signal_decomposition(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<StateSignalDecompositionReport> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let total_days = regimes.len();
    let window_from = regimes.first().map(|r| r.date).unwrap_or(bars[0].date);
    let window_to = regimes.last().map(|r| r.date).unwrap_or(bars[bars.len() - 1].date);

    let regime_attributions = compute_regime_attributions(regimes, bars);
    let persistence_results = compute_persistence_audit(regimes, bars);
    let transition_results = compute_transition_alpha(regimes, bars);

    // Determine dominant alpha source
    let best_transition = transition_results.iter().max_by(|a, b| {
        a.avg_60d_return.partial_cmp(&b.avg_60d_return).unwrap()
    });
    
    let best_static = regime_attributions.iter().max_by(|a, b| {
        a.avg_60d_return.partial_cmp(&b.avg_60d_return).unwrap()
    });

    let conclusion = if let (Some(trans), Some(stat)) = (best_transition, best_static) {
        if trans.avg_60d_return > stat.avg_60d_return * 1.5 && trans.avg_60d_return > 0.05 {
            format!("transition_alpha: {} dominates with {:.1}% 60d return", 
                trans.from_state, trans.avg_60d_return * 100.0)
        } else {
            format!("static_regime: {} dominates with {:.1}% 60d return",
                stat.state, stat.avg_60d_return * 100.0)
        }
    } else {
        "insufficient_data".to_string()
    };

    Some(StateSignalDecompositionReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        state_attributions: regime_attributions,
        persistence_comparison: persistence_results,
        transition_alphas: transition_results,
        conclusion,
    })
}

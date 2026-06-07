use chrono::NaiveDate;
use core_domain::{DailyBar, MarketRegimeSnapshot};
use std::collections::HashMap;

// ============================================================
// TASK-070B: State Persistence Economics
// Computes economic statistics for each regime state:
// - Forward returns (20d, 60d, 120d)
// - Volatility
// - Max drawdown
// - Win rate (positive return frequency)
// ============================================================

#[derive(Debug, Clone)]
pub struct StateEconomics {
    pub state: String,
    pub sample_count: usize,
    pub fwd_return_20d_mean: f64,
    pub fwd_return_20d_std: f64,
    pub fwd_return_20d_min: f64,
    pub fwd_return_20d_max: f64,
    pub fwd_return_20d_win_rate: f64,
    pub fwd_return_60d_mean: f64,
    pub fwd_return_60d_std: f64,
    pub fwd_return_60d_min: f64,
    pub fwd_return_60d_max: f64,
    pub fwd_return_60d_win_rate: f64,
    pub fwd_return_120d_mean: f64,
    pub fwd_return_120d_std: f64,
    pub fwd_return_120d_min: f64,
    pub fwd_return_120d_max: f64,
    pub fwd_return_120d_win_rate: f64,
    pub max_drawdown_mean: f64,
    pub max_drawdown_std: f64,
    pub max_drawdown_max: f64,
    pub max_drawdown_min: f64,
    pub volatility_mean: f64,
    pub volatility_std: f64,
}

#[derive(Debug, Clone)]
pub struct StateEconomicsReport {
    pub market: String,
    pub states: Vec<StateEconomics>,
}

fn compute_forward_returns(bars: &[DailyBar], horizon: usize) -> HashMap<NaiveDate, f64> {
    let n = bars.len();
    let mut result = HashMap::new();
    if n <= horizon {
        return result;
    }
    for i in 0..n - horizon {
        let current = bars[i].close;
        let future = bars[i + horizon].close;
        let ret = (future - current) / current;
        result.insert(bars[i].date, ret);
    }
    result
}

fn compute_max_drawdown(bars: &[DailyBar], start_idx: usize, horizon: usize) -> f64 {
    let end_idx = (start_idx + horizon).min(bars.len());
    if start_idx >= end_idx {
        return 0.0;
    }
    let mut peak = bars[start_idx].close;
    let mut max_dd = 0.0;
    for i in start_idx..end_idx {
        if bars[i].close > peak {
            peak = bars[i].close;
        }
        let dd = (bars[i].close - peak) / peak;
        if dd < max_dd {
            max_dd = dd;
        }
    }
    max_dd
}

fn compute_volatility(bars: &[DailyBar], start_idx: usize, horizon: usize) -> f64 {
    let end_idx = (start_idx + horizon).min(bars.len());
    if start_idx + 1 >= end_idx {
        return 0.0;
    }
    let mut returns = Vec::new();
    for i in start_idx + 1..end_idx {
        let ret = (bars[i].close - bars[i - 1].close) / bars[i - 1].close;
        returns.push(ret);
    }
    if returns.is_empty() {
        return 0.0;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (returns.len() - 1).max(1) as f64;
    variance.sqrt()
}

fn compute_state_economics(
    market: &str,
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> Vec<StateEconomics> {
    let bar_by_date: HashMap<NaiveDate, usize> = bars.iter().enumerate().map(|(i, b)| (b.date, i)).collect();

    let fwd_20d = compute_forward_returns(bars, 20);
    let fwd_60d = compute_forward_returns(bars, 60);
    let fwd_120d = compute_forward_returns(bars, 120);

    let mut state_data: HashMap<String, Vec<(f64, f64, f64, f64, f64)>> = HashMap::new();
    // (fwd20, fwd60, fwd120, max_dd, vol)

    for regime in regimes {
        if let Some(&idx) = bar_by_date.get(&regime.date) {
            let r20 = fwd_20d.get(&regime.date).copied();
            let r60 = fwd_60d.get(&regime.date).copied();
            let r120 = fwd_120d.get(&regime.date).copied();

            if r20.is_some() || r60.is_some() || r120.is_some() {
                let dd = compute_max_drawdown(bars, idx, 60);
                let vol = compute_volatility(bars, idx, 60);

                state_data
                    .entry(regime.regime_label.clone())
                    .or_insert_with(Vec::new)
                    .push((r20.unwrap_or(0.0), r60.unwrap_or(0.0), r120.unwrap_or(0.0), dd, vol));
            }
        }
    }

    let mut results = Vec::new();
    for (state, data) in state_data {
        let n = data.len() as f64;
        if n == 0.0 {
            continue;
        }

        let r20_vec: Vec<f64> = data.iter().map(|d| d.0).collect();
        let r60_vec: Vec<f64> = data.iter().map(|d| d.1).collect();
        let r120_vec: Vec<f64> = data.iter().map(|d| d.2).collect();
        let dd_vec: Vec<f64> = data.iter().map(|d| d.3).collect();
        let vol_vec: Vec<f64> = data.iter().map(|d| d.4).collect();

        let win_rate = |v: &[f64]| {
            let positive = v.iter().filter(|&&x| x > 0.0).count();
            positive as f64 / v.len() as f64
        };

        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let std = |v: &[f64]| {
            let m = mean(v);
            let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1).max(1) as f64;
            var.sqrt()
        };
        let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
        let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        results.push(StateEconomics {
            state: state.clone(),
            sample_count: data.len(),
            fwd_return_20d_mean: mean(&r20_vec),
            fwd_return_20d_std: std(&r20_vec),
            fwd_return_20d_min: min(&r20_vec),
            fwd_return_20d_max: max(&r20_vec),
            fwd_return_20d_win_rate: win_rate(&r20_vec),
            fwd_return_60d_mean: mean(&r60_vec),
            fwd_return_60d_std: std(&r60_vec),
            fwd_return_60d_min: min(&r60_vec),
            fwd_return_60d_max: max(&r60_vec),
            fwd_return_60d_win_rate: win_rate(&r60_vec),
            fwd_return_120d_mean: mean(&r120_vec),
            fwd_return_120d_std: std(&r120_vec),
            fwd_return_120d_min: min(&r120_vec),
            fwd_return_120d_max: max(&r120_vec),
            fwd_return_120d_win_rate: win_rate(&r120_vec),
            max_drawdown_mean: mean(&dd_vec),
            max_drawdown_std: std(&dd_vec),
            max_drawdown_max: max(&dd_vec),
            max_drawdown_min: min(&dd_vec),
            volatility_mean: mean(&vol_vec),
            volatility_std: std(&vol_vec),
        });
    }

    results.sort_by(|a, b| a.state.cmp(&b.state));
    results
}

pub fn audit_state_persistence_economics(
    cn_regimes: &[MarketRegimeSnapshot],
    cn_bars: &[DailyBar],
    hk_regimes: &[MarketRegimeSnapshot],
    hk_bars: &[DailyBar],
) -> (StateEconomicsReport, StateEconomicsReport) {
    let cn_states = compute_state_economics("CN", cn_regimes, cn_bars);
    let hk_states = compute_state_economics("HK", hk_regimes, hk_bars);

    (
        StateEconomicsReport {
            market: "CN".to_string(),
            states: cn_states,
        },
        StateEconomicsReport {
            market: "HK".to_string(),
            states: hk_states,
        },
    )
}

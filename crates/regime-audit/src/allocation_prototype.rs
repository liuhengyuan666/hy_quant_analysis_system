use chrono::NaiveDate;
use core_domain::{
    AllocationPrototypeReport, AllocationStrategyResult, DailyBar, MarketRegimeSnapshot,
};
use std::collections::HashMap;

// ============================================================
// TASK-032: Allocation Prototype
// Compares 4 allocation strategies using regime signals.
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

fn normalize_state_label(label: &str) -> String {
    label.to_lowercase().replace("risk_on", "riskon").replace("risk_off", "riskoff")
}

fn get_dual_allocation(state: &str, economic: &str) -> f64 {
    match (state, economic) {
        ("riskon", "favorable") => 1.0,
        ("riskon", "neutral") => 0.8,
        ("riskon", "unfavorable") => 0.5,
        ("neutral", "favorable") => 0.8,
        ("neutral", "neutral") => 0.5,
        ("neutral", "unfavorable") => 0.3,
        ("riskoff", "favorable") => 0.6,
        ("riskoff", "neutral") => 0.2,
        ("riskoff", "unfavorable") => 0.0,
        _ => 0.5,
    }
}

fn get_state_only_allocation(state: &str) -> f64 {
    match state {
        "riskon" => 1.0,
        "neutral" => 0.5,
        "riskoff" => 0.0,
        _ => 0.5,
    }
}

fn get_economic_only_allocation(economic: &str) -> f64 {
    match economic {
        "favorable" => 1.0,
        "neutral" => 0.5,
        "unfavorable" => 0.0,
        _ => 0.5,
    }
}

fn run_backtest(
    dates: &[NaiveDate],
    returns: &[f64],
    allocations: &[f64],
    strategy_name: &str,
) -> AllocationStrategyResult {
    let n = dates.len().min(returns.len()).min(allocations.len());
    if n < 2 {
        return AllocationStrategyResult {
            strategy: strategy_name.to_string(),
            cagr: 0.0,
            sharpe: 0.0,
            sortino: 0.0,
            max_drawdown: 0.0,
            turnover: 0.0,
            final_value: 1.0,
            total_return: 0.0,
            avg_position: 0.0,
        };
    }

    let mut portfolio_value = 1.0;
    let mut peak = 1.0;
    let mut max_dd = 0.0;
    let mut daily_returns = Vec::new();
    let mut negative_returns = Vec::new();
    let mut turnover_sum = 0.0;
    let mut position_sum = 0.0;

    for i in 0..n {
        let alloc = allocations[i].clamp(0.0, 1.0);
        let daily_ret = returns[i] * alloc;
        portfolio_value *= 1.0 + daily_ret;
        daily_returns.push(daily_ret);
        if daily_ret < 0.0 {
            negative_returns.push(daily_ret);
        }
        position_sum += alloc;

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

    let total_return = portfolio_value - 1.0;
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
        (mean_ret * 252.0_f64.sqrt()) / (std_dev * 252.0_f64.sqrt())
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
        (mean_ret * 252.0_f64.sqrt()) / (neg_std * 252.0_f64.sqrt())
    } else {
        0.0
    };

    let turnover = if n > 1 {
        turnover_sum / (n - 1) as f64
    } else {
        0.0
    };

    let avg_position = position_sum / n as f64;

    AllocationStrategyResult {
        strategy: strategy_name.to_string(),
        cagr,
        sharpe,
        sortino,
        max_drawdown: max_dd,
        turnover,
        final_value: portfolio_value,
        total_return,
        avg_position,
    }
}

pub fn compute_allocation_prototype(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<AllocationPrototypeReport> {
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

    // Build daily returns and signals
    let mut dates = Vec::new();
    let mut daily_returns = Vec::new();
    let mut baseline_allocs = Vec::new();
    let mut state_allocs = Vec::new();
    let mut economic_allocs = Vec::new();
    let mut dual_allocs = Vec::new();

    for i in 0..regimes_filtered.len() - 1 {
        let regime = regimes_filtered[i];
        let Some(current_close) = close_by_date.get(&regime.date).copied() else {
            continue;
        };

        let next_regime = regimes_filtered[i + 1];
        let Some(next_close) = close_by_date.get(&next_regime.date).copied() else {
            continue;
        };

        let daily_ret = if current_close > 0.0 {
            (next_close - current_close) / current_close
        } else {
            0.0
        };

        let state = normalize_state_label(&regime.regime_label);
        let economic = classify_economic_state(regime.liquidity_score, regime.risk_score, scope_str);

        dates.push(regime.date);
        daily_returns.push(daily_ret);
        baseline_allocs.push(1.0);
        state_allocs.push(get_state_only_allocation(&state));
        economic_allocs.push(get_economic_only_allocation(&economic));
        dual_allocs.push(get_dual_allocation(&state, &economic));
    }

    if dates.len() < 10 {
        return None;
    }

    let baseline = run_backtest(&dates, &daily_returns, &baseline_allocs, "baseline");
    let state_only = run_backtest(&dates, &daily_returns, &state_allocs, "state_only");
    let economic_only = run_backtest(&dates, &daily_returns, &economic_allocs, "economic_only");
    let dual_layer = run_backtest(&dates, &daily_returns, &dual_allocs, "dual_layer");

    let dual_better_than_baseline = dual_layer.cagr > baseline.cagr && dual_layer.sharpe > baseline.sharpe;
    let dual_better_than_state = dual_layer.cagr > state_only.cagr && dual_layer.sharpe > state_only.sharpe;
    let dual_better_than_economic = dual_layer.cagr > economic_only.cagr && dual_layer.sharpe > economic_only.sharpe;

    Some(AllocationPrototypeReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        strategies: vec![baseline, state_only, economic_only, dual_layer],
        dual_better_than_baseline,
        dual_better_than_state,
        dual_better_than_economic,
    })
}

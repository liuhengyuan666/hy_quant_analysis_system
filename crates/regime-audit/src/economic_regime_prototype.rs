use chrono::NaiveDate;
use core_domain::{
    DailyBar, EconomicRegimePrototypeReport, EconomicRegimeSnapshot, EconomicState,
    MarketRegimeSnapshot,
};
use std::collections::HashMap;

// ============================================================
// TASK-029: Economic Regime Prototype
// Independent economic-prediction layer.
// NOT connected to production macro-engine.
// ============================================================

fn classify_economic_state(liquidity_score: f64, risk_score: f64, scope: &str) -> (EconomicState, String, f64) {
    match scope {
        "HK" => {
            // HK: Liquidity-dominant (TASK-028A showed Liquidity is economic-best)
            if liquidity_score >= 55.0 {
                (EconomicState::Favorable, "liquidity".to_string(), liquidity_score)
            } else if liquidity_score < 40.0 {
                (EconomicState::Unfavorable, "liquidity".to_string(), liquidity_score)
            } else {
                (EconomicState::Neutral, "liquidity".to_string(), liquidity_score)
            }
        }
        "CN" => {
            // CN: Liquidity-in-RiskOff context (TASK-028A showed Liquidity in RiskOff has Pearson=0.426)
            // For prototype, use Liquidity as primary with slightly different thresholds
            if liquidity_score >= 50.0 {
                (EconomicState::Favorable, "liquidity".to_string(), liquidity_score)
            } else if liquidity_score < 35.0 {
                (EconomicState::Unfavorable, "liquidity".to_string(), liquidity_score)
            } else {
                (EconomicState::Neutral, "liquidity".to_string(), liquidity_score)
            }
        }
        _ => {
            // Global default: balanced approach
            let composite = (liquidity_score + risk_score) / 2.0;
            if composite >= 55.0 {
                (EconomicState::Favorable, "composite".to_string(), composite)
            } else if composite < 40.0 {
                (EconomicState::Unfavorable, "composite".to_string(), composite)
            } else {
                (EconomicState::Neutral, "composite".to_string(), composite)
            }
        }
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

fn compute_economic_separation(
    labels: &[(NaiveDate, String)],
    close_by_date: &HashMap<NaiveDate, f64>,
) -> f64 {
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

pub fn compute_economic_regime_prototype(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<EconomicRegimePrototypeReport> {
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

    let mut snapshots = Vec::new();
    let mut labels: Vec<(NaiveDate, String)> = Vec::new();

    for regime in &regimes_filtered {
        let (state, dominant_factor, factor_score) = classify_economic_state(
            regime.liquidity_score,
            regime.risk_score,
            scope_str,
        );
        let state_str = match state {
            EconomicState::Favorable => "favorable".to_string(),
            EconomicState::Neutral => "neutral".to_string(),
            EconomicState::Unfavorable => "unfavorable".to_string(),
        };
        snapshots.push(EconomicRegimeSnapshot {
            date: regime.date,
            scope: scope_str.to_string(),
            state,
            dominant_factor,
            factor_score,
        });
        labels.push((regime.date, state_str));
    }

    let mut state_distribution: HashMap<String, f64> = HashMap::new();
    for snapshot in &snapshots {
        let key = match snapshot.state {
            EconomicState::Favorable => "favorable".to_string(),
            EconomicState::Neutral => "neutral".to_string(),
            EconomicState::Unfavorable => "unfavorable".to_string(),
        };
        *state_distribution.entry(key).or_insert(0.0) += 1.0 / total_days as f64;
    }

    let economic_separation = compute_economic_separation(&labels, &close_by_date);

    let validation_status = if economic_separation >= 33.4 {
        "pass".to_string()
    } else {
        "fail".to_string()
    };

    Some(EconomicRegimePrototypeReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        state_distribution,
        economic_separation,
        validation_status,
    })
}

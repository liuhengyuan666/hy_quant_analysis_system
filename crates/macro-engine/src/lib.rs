use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::NaiveDate;
use core_domain::{DailyBar, MacroSnapshot, MarketRegimeSnapshot};

#[derive(Debug, Clone)]
pub struct MacroFactorSeries {
    pub factor_name: &'static str,
    pub source: &'static str,
    pub invert_score: bool,
    pub observations: Vec<(NaiveDate, f64)>,
}

fn rolling_min_max(values: &[f64], index: usize, lookback: usize) -> (f64, f64) {
    let start = index.saturating_sub(lookback.saturating_sub(1));
    let window = &values[start..=index];
    let min = window
        .iter()
        .fold(f64::INFINITY, |acc, value| acc.min(*value));
    let max = window
        .iter()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(*value));
    (min, max)
}

fn bounded_score(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

pub fn build_macro_snapshots(series: &[MacroFactorSeries], lookback: usize) -> Vec<MacroSnapshot> {
    let mut snapshots = Vec::new();
    for factor in series {
        let values: Vec<f64> = factor
            .observations
            .iter()
            .map(|(_, value)| *value)
            .collect();
        for (index, (date, value)) in factor.observations.iter().enumerate() {
            let (min, max) = rolling_min_max(&values, index, lookback);
            let raw_score = if (max - min).abs() < f64::EPSILON {
                50.0
            } else if factor.invert_score {
                ((max - value) / (max - min)) * 100.0
            } else {
                ((value - min) / (max - min)) * 100.0
            };
            snapshots.push(MacroSnapshot {
                date: *date,
                factor_name: factor.factor_name.to_string(),
                factor_value: *value,
                factor_score: bounded_score(raw_score),
                factor_source: factor.source.to_string(),
            });
        }
    }
    snapshots
}

fn rolling_mean(values: &[f64], index: usize, period: usize) -> Option<f64> {
    if index + 1 < period {
        return None;
    }
    let window = &values[index + 1 - period..=index];
    Some(window.iter().sum::<f64>() / period as f64)
}

fn trend_score_for_anchor(bars: &[DailyBar]) -> BTreeMap<NaiveDate, f64> {
    let closes: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
    let mut result = BTreeMap::new();
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

pub fn build_market_regimes(
    macro_snapshots: &[MacroSnapshot],
    cn_anchor: &[DailyBar],
    hk_anchor: &[DailyBar],
) -> Vec<MarketRegimeSnapshot> {
    let mut grouped: HashMap<NaiveDate, HashMap<String, f64>> = HashMap::new();
    let mut factor_history: HashMap<String, BTreeMap<NaiveDate, f64>> = HashMap::new();
    for row in macro_snapshots {
        grouped
            .entry(row.date)
            .or_default()
            .insert(row.factor_name.clone(), row.factor_score);
        factor_history
            .entry(row.factor_name.clone())
            .or_default()
            .insert(row.date, row.factor_score);
    }

    let cn_trend = trend_score_for_anchor(cn_anchor);
    let hk_trend = trend_score_for_anchor(hk_anchor);
    let all_dates = cn_trend
        .keys()
        .chain(hk_trend.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    let required_factors = ["vix", "dollar_index", "us10y", "fed_funds"];

    all_dates
        .into_iter()
        .filter_map(|date| {
            let mut factor_values = HashMap::new();
            let mut factor_dates = Vec::new();

            for factor_name in required_factors {
                let history = factor_history.get(factor_name)?;
                let (as_of_date, score) = history.range(..=date).next_back()?;
                factor_values.insert(factor_name, *score);
                factor_dates.push(*as_of_date);
            }

            let macro_as_of_date = factor_dates.into_iter().min()?;
            let vix = *factor_values.get("vix")?;
            let dollar = *factor_values.get("dollar_index")?;
            let us10y = *factor_values.get("us10y")?;
            let fed_funds = *factor_values.get("fed_funds")?;
            let trend_components = [cn_trend.get(&date).copied(), hk_trend.get(&date).copied()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            let trend_score = if trend_components.is_empty() {
                50.0
            } else {
                trend_components.iter().sum::<f64>() / trend_components.len() as f64
            };
            let liquidity_score = (us10y + fed_funds) / 2.0;
            let risk_score = (vix + dollar) / 2.0;
            let regime_label =
                if trend_score >= 60.0 && liquidity_score >= 50.0 && risk_score >= 55.0 {
                    "risk_on"
                } else if trend_score < 40.0 || risk_score < 40.0 {
                    "risk_off"
                } else {
                    "neutral"
                };
            Some(MarketRegimeSnapshot {
                date,
                macro_as_of_date,
                market: "GLOBAL".to_string(),
                trend_score,
                liquidity_score,
                risk_score,
                regime_label: regime_label.to_string(),
            })
        })
        .collect()
}

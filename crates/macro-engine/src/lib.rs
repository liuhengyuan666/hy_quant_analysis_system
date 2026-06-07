use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::NaiveDate;
use core_domain::{
    AnalysisScope, DailyBar, EnvironmentSnapshot, MacroSnapshot, MarketRegimeSnapshot,
    StrategyState, StrategyStateSnapshot,
};

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

fn rolling_mean_std(values: &[f64], index: usize, lookback: usize) -> (f64, f64) {
    let start = index.saturating_sub(lookback.saturating_sub(1));
    let window = &values[start..=index];
    let mean = window.iter().sum::<f64>() / window.len() as f64;
    let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / window.len() as f64;
    let std = variance.sqrt();
    (mean, std)
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
            // TASK-080F: Fed Funds uses Z-score instead of min/max normalization
            // to avoid regime clustering (zero-rate vs hiking period)
            let raw_score = if factor.factor_name == "fed_funds" {
                let (mean, std) = rolling_mean_std(&values, index, lookback);
                if std < f64::EPSILON {
                    50.0
                } else {
                    let z_score = (value - mean) / std;
                    // TASK-080F: Cap Z-score to avoid regime clustering at extremes
                    // Raw Z-scores can reach ±15 during regime transitions (e.g. 2020→2022)
                    // Capping at ±3 maps to score range [5, 95] without 0/100 clustering
                    let capped_z = z_score.clamp(-3.0, 3.0);
                    // Invert: high Fed Funds (positive Z) = tight = bad → low score
                    50.0 - capped_z * 15.0
                }
            } else {
                let (min, max) = rolling_min_max(&values, index, lookback);
                if (max - min).abs() < f64::EPSILON {
                    50.0
                } else if factor.invert_score {
                    ((max - value) / (max - min)) * 100.0
                } else {
                    ((value - min) / (max - min)) * 100.0
                }
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

    let mut rows = Vec::new();
    for date in all_dates {
        let Some((macro_as_of_date, macro_risk_score, macro_liquidity_score)) = (|| {
            let mut factor_dates = Vec::new();

            let resolve_group = |names: &[&str], factor_dates: &mut Vec<NaiveDate>| {
                let values = names
                    .iter()
                    .filter_map(|factor_name| {
                        let history = factor_history.get(*factor_name)?;
                        let (as_of_date, score) = history.range(..=date).next_back()?;
                        factor_dates.push(*as_of_date);
                        Some(*score)
                    })
                    .collect::<Vec<_>>();
                (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
            };

            let macro_risk_score =
                resolve_group(&["vix", "dollar_index"], &mut factor_dates).unwrap_or(50.0);
            let macro_liquidity_score =
                resolve_group(&["us10y", "fed_funds"], &mut factor_dates).unwrap_or(50.0);

            if factor_dates.is_empty() {
                return None;
            }

            Some((
                factor_dates.into_iter().min()?,
                macro_risk_score,
                macro_liquidity_score,
            ))
        })() else {
            continue;
        };

        for scope in [AnalysisScope::Global, AnalysisScope::Cn, AnalysisScope::Hk] {
            let trend_score = match scope {
                AnalysisScope::Global => {
                    let trend_components =
                        [cn_trend.get(&date).copied(), hk_trend.get(&date).copied()]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();
                    if trend_components.is_empty() {
                        50.0
                    } else {
                        trend_components.iter().sum::<f64>() / trend_components.len() as f64
                    }
                }
                AnalysisScope::Cn => cn_trend.get(&date).copied().unwrap_or(50.0),
                AnalysisScope::Hk => hk_trend.get(&date).copied().unwrap_or(50.0),
            };
            let regime_label =
                if trend_score >= 60.0 && macro_liquidity_score >= 50.0 && macro_risk_score >= 55.0
                {
                    "risk_on"
                } else if trend_score < 40.0 || macro_risk_score < 40.0 {
                    "risk_off"
                } else {
                    "neutral"
                };
            rows.push(MarketRegimeSnapshot {
                date,
                macro_as_of_date,
                market: scope.as_str().to_string(),
                trend_score,
                liquidity_score: macro_liquidity_score,
                risk_score: macro_risk_score,
                regime_label: regime_label.to_string(),
            });
        }
    }
    rows
}

/// Build a strategy state snapshot from regime + environment.
///
/// State transitions are based on trend strength, risk level, market breadth,
/// liquidity and stress proxies.  The rules are intentionally conservative:
/// - liquidity crisis => NO_TRADE regardless of other signals
/// - high risk or weakening trend => DE_RISK
/// - strong trend + healthy breadth + low risk => FULL_TREND
/// - improving but not yet confirmed => CONFIRM_ADD
/// - deeply depressed but not crashing => LEFT_PROBE
pub fn build_strategy_state(
    regime: &MarketRegimeSnapshot,
    environment: &EnvironmentSnapshot,
) -> StrategyStateSnapshot {
    let trend = regime.trend_score;
    let risk = regime.risk_score;
    let liquidity = regime.liquidity_score;
    let breadth = environment.breadth_pct;
    let env_score = environment.environment_score;
    let stress = environment.stress_proxy_score;

    let (state, reason, position_pct) = if liquidity < 20.0 || (trend < 25.0 && risk > 75.0) {
        (
            StrategyState::NoTrade,
            "流动性极度紧张或市场极度低迷，全面观望",
            0.0,
        )
    } else if trend < 45.0 && breadth < 35.0 && env_score < 45.0 && risk < 65.0 {
        (
            StrategyState::LeftProbe,
            "市场低迷但未到崩溃边缘，可能触底，小仓位试探",
            20.0,
        )
    } else if trend > 70.0 && risk < 40.0 && breadth > 50.0 && env_score > 60.0 && stress < 60.0 {
        (
            StrategyState::FullTrend,
            "趋势明确，风险可控，广度健康，满仓操作",
            100.0,
        )
    } else if trend > 55.0 && env_score > 50.0 && breadth > 35.0 && risk < 55.0 {
        (StrategyState::ConfirmAdd, "趋势初步确认，逐步加仓", 60.0)
    } else if trend < 55.0 || risk > 60.0 || stress > 70.0 {
        (
            StrategyState::DeRisk,
            "趋势减弱或风险/压力上升，降低仓位",
            30.0,
        )
    } else {
        (StrategyState::NoTrade, "市场状态不明确，保持观望", 0.0)
    };

    let state_score = (trend * 0.35 + (100.0 - risk) * 0.25 + breadth * 0.20 + env_score * 0.20)
        .clamp(0.0, 100.0);

    StrategyStateSnapshot {
        date: regime.date,
        scope: regime.market.clone(),
        state,
        state_score,
        transition_reason: reason.to_string(),
        recommended_position_pct: position_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_anchor(symbol: &str, closes: &[f64]) -> Vec<DailyBar> {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        closes
            .iter()
            .enumerate()
            .map(|(index, close)| DailyBar {
                date: start + chrono::Duration::days(index as i64),
                symbol: symbol.to_string(),
                open: *close,
                high: *close,
                low: *close,
                close: *close,
                volume: 1_000.0,
                turnover: Some(10_000.0),
            })
            .collect()
    }

    #[test]
    fn build_market_regimes_outputs_global_cn_hk_rows() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let dates = (0..65)
            .map(|offset| start + chrono::Duration::days(offset))
            .collect::<Vec<_>>();
        let macro_snapshots = dates
            .iter()
            .flat_map(|date| {
                [
                    ("vix", 80.0),
                    ("dollar_index", 80.0),
                    ("us10y", 80.0),
                    ("fed_funds", 80.0),
                ]
                .into_iter()
                .map(move |(factor_name, factor_score)| MacroSnapshot {
                    date: *date,
                    factor_name: factor_name.to_string(),
                    factor_value: factor_score,
                    factor_score,
                    factor_source: "test".to_string(),
                })
            })
            .collect::<Vec<_>>();
        let cn_anchor = build_anchor(
            "000300",
            &(1..=65).map(|value| value as f64).collect::<Vec<_>>(),
        );
        let hk_anchor = build_anchor(
            "HSI",
            &(1..=65).rev().map(|value| value as f64).collect::<Vec<_>>(),
        );

        let rows = build_market_regimes(&macro_snapshots, &cn_anchor, &hk_anchor);
        let latest_date = *dates.last().unwrap();
        let latest_rows = rows
            .iter()
            .filter(|row| row.date == latest_date)
            .collect::<Vec<_>>();

        assert_eq!(latest_rows.len(), 3);
        let by_scope = latest_rows
            .into_iter()
            .map(|row| (row.market.as_str(), row))
            .collect::<std::collections::BTreeMap<_, _>>();

        assert_eq!(by_scope.get("CN").unwrap().regime_label, "risk_on");
        assert_eq!(by_scope.get("HK").unwrap().regime_label, "risk_off");
        assert_eq!(by_scope.get("GLOBAL").unwrap().regime_label, "neutral");
        assert!(
            by_scope.get("CN").unwrap().trend_score > by_scope.get("GLOBAL").unwrap().trend_score
        );
        assert!(
            by_scope.get("HK").unwrap().trend_score < by_scope.get("GLOBAL").unwrap().trend_score
        );
    }
}

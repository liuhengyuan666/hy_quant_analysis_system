use chrono::NaiveDate;
use core_domain::DailyBar;
use gt_regime_generator::{CandidateGenerator, RegimeCandidate};
use market_state_extractor::{
    MarketStateObservation, TrendDirection, VolatilityRegime,
};
use std::collections::HashMap;

// ============================================================
// TASK-018D: Regime Attribution Audit
// Diagnose why future best-performing periods fall into Neutral
// instead of RiskOn.
// ============================================================

// ------------------------------------------------------------------
// Audit 1: Candidate Coverage — forward returns by candidate type
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CandidateCoverageStat {
    pub count: usize,
    pub pct: f64,
    pub forward_return_20d_mean: f64,
    pub forward_return_60d_mean: f64,
    pub max_drawdown_median: f64,
    pub sharpe_median: f64,
}

#[derive(Debug, Clone)]
pub struct CandidateCoverageReport {
    pub scope: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub stats: HashMap<String, CandidateCoverageStat>,
}

pub fn audit_candidate_coverage(
    observations: &[MarketStateObservation],
    bars: &[DailyBar],
    scope: &str,
) -> CandidateCoverageReport {
    let mut candidate_returns: HashMap<String, Vec<(f64, f64, f64, f64)>> = HashMap::new();

    for (idx, obs) in observations.iter().enumerate() {
        let (candidate, _confidence) = CandidateGenerator::generate(obs);
        let candidate_label = format!("{:?}", candidate);

        let forward_20 = if idx + 20 < bars.len() {
            let future_close = bars[idx + 20].close;
            (future_close - bars[idx].close) / bars[idx].close
        } else {
            f64::NAN
        };

        let forward_60 = if idx + 60 < bars.len() {
            let future_close = bars[idx + 60].close;
            (future_close - bars[idx].close) / bars[idx].close
        } else {
            f64::NAN
        };

        let max_dd = if idx + 60 < bars.len() {
            let mut max_dd = 0.0;
            let entry = bars[idx].close;
            let mut peak = entry;
            for j in idx..=idx + 60 {
                if bars[j].close > peak {
                    peak = bars[j].close;
                }
                let dd = (bars[j].close - peak) / peak;
                if dd < max_dd {
                    max_dd = dd;
                }
            }
            max_dd
        } else {
            f64::NAN
        };

        let vol = if idx + 20 < bars.len() {
            let returns: Vec<f64> = (idx + 1..=idx + 20)
                .map(|j| {
                    let prev = bars[j - 1].close;
                    let curr = bars[j].close;
                    if prev.abs() < f64::EPSILON {
                        0.0
                    } else {
                        (curr - prev) / prev
                    }
                })
                .collect();
            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance =
                returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
            variance.sqrt() * (252.0_f64).sqrt() // annualized
        } else {
            f64::NAN
        };

        if !forward_60.is_nan() {
            candidate_returns
                .entry(candidate_label)
                .or_default()
                .push((forward_20, forward_60, max_dd, vol));
        }
    }

    let total = candidate_returns.values().map(|v| v.len()).sum::<usize>() as f64;
    let mut stats = HashMap::new();

    for (candidate_label, data) in candidate_returns {
        let n = data.len();
        let forward_20_mean = data.iter().map(|d| d.0).sum::<f64>() / n as f64;
        let forward_60_mean = data.iter().map(|d| d.1).sum::<f64>() / n as f64;

        let mut max_dds: Vec<f64> = data.iter().map(|d| d.2).collect();
        max_dds.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let max_dd_median = max_dds.get(n / 2).copied().unwrap_or(0.0);

        let mut sharpes: Vec<f64> = data
            .iter()
            .map(|d| {
                let ret = d.1; // 60d return
                let vol = d.3;
                if vol.abs() < f64::EPSILON {
                    0.0
                } else {
                    (ret / (vol / (252.0_f64).sqrt())) * (252.0_f64 / 60.0).sqrt()
                }
            })
            .collect();
        sharpes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let sharpe_median = sharpes.get(n / 2).copied().unwrap_or(0.0);

        stats.insert(
            candidate_label,
            CandidateCoverageStat {
                count: n,
                pct: n as f64 / total,
                forward_return_20d_mean: forward_20_mean,
                forward_return_60d_mean: forward_60_mean,
                max_drawdown_median: max_dd_median,
                sharpe_median,
            },
        );
    }

    CandidateCoverageReport {
        scope: scope.to_string(),
        window_from: observations.first().map(|o| o.date).unwrap_or(NaiveDate::MIN),
        window_to: observations.last().map(|o| o.date).unwrap_or(NaiveDate::MAX),
        stats,
    }
}

// ------------------------------------------------------------------
// Audit 2: Trigger Attribution — which dimensions drive each candidate
// ------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct TriggerStat {
    pub count: usize,
    pub pct_of_regime: f64,
    pub avg_60d_return: f64,
}

#[derive(Debug, Clone)]
pub struct TriggerAttributionReport {
    pub scope: String,
    pub trigger_breakdown: HashMap<String, HashMap<String, TriggerStat>>,
    // Outer key: candidate (RiskOn/Neutral/RiskOff)
    // Inner key: trigger description
}

pub fn audit_trigger_attribution(
    observations: &[MarketStateObservation],
    bars: &[DailyBar],
    scope: &str,
) -> TriggerAttributionReport {
    let mut trigger_breakdown: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();

    for (idx, obs) in observations.iter().enumerate() {
        let (candidate, _) = CandidateGenerator::generate(obs);
        let candidate_label = format!("{:?}", candidate);

        let forward_60 = if idx + 60 < bars.len() {
            (bars[idx + 60].close - bars[idx].close) / bars[idx].close
        } else {
            f64::NAN
        };

        if forward_60.is_nan() {
            continue;
        }

        // Identify the primary trigger(s)
        let mut triggers = Vec::new();

        // Trend trigger
        let trend_trigger = match (obs.trend.short_term, obs.trend.medium_term) {
            (TrendDirection::StrongUptrend, TrendDirection::StrongUptrend) => {
                "Trend:StrongUp+StrongUp"
            }
            (TrendDirection::StrongDowntrend, _) | (_, TrendDirection::StrongDowntrend) => {
                "Trend:StrongDown"
            }
            (TrendDirection::Uptrend, _) | (_, TrendDirection::Uptrend) => "Trend:Up",
            (TrendDirection::Downtrend, _) | (_, TrendDirection::Downtrend) => "Trend:Down",
            _ => "Trend:Sideways",
        };
        triggers.push(trend_trigger.to_string());

        // Volatility trigger
        let vol_trigger = match obs.volatility.volatility_regime {
            VolatilityRegime::Low => "Vol:Low",
            VolatilityRegime::Normal => "Vol:Normal",
            VolatilityRegime::Elevated => "Vol:Elevated",
            VolatilityRegime::Spike => "Vol:Spike",
        };
        triggers.push(vol_trigger.to_string());

        // Drawdown trigger
        let dd_trigger = if obs.drawdown_pct > -5.0 {
            "DD:Shallow(>-5%)"
        } else if obs.drawdown_pct > -10.0 {
            "DD:Moderate(-5~-10%)"
        } else if obs.drawdown_pct > -15.0 {
            "DD:Deep(-10~-15%)"
        } else if obs.drawdown_pct > -20.0 {
            "DD:Severe(-15~-20%)"
        } else {
            "DD:Crash(<-20%)"
        };
        triggers.push(dd_trigger.to_string());

        // Also record the dominant trigger (the one with highest score)
        let risk_on_score = score_risk_on(obs);
        let risk_off_score = score_risk_off(obs);

        let dominant = if risk_on_score > 60.0 && risk_on_score > risk_off_score {
            "Dominant:Trend+Vol+DD"
        } else if risk_off_score > 60.0 && risk_off_score > risk_on_score {
            "Dominant:RiskOffSignal"
        } else {
            "Dominant:Neutral(ThresholdNotMet)"
        };
        triggers.push(dominant.to_string());

        for trigger in triggers {
            trigger_breakdown
                .entry(candidate_label.clone())
                .or_default()
                .entry(trigger)
                .or_default()
                .push(forward_60);
        }
    }

    // Convert Vec<f64> to TriggerStat
    let mut result: HashMap<String, HashMap<String, TriggerStat>> = HashMap::new();
    for (candidate, triggers) in trigger_breakdown {
        let total_for_candidate = triggers.values().map(|v| v.len()).sum::<usize>() as f64;
        let mut trigger_stats = HashMap::new();
        for (trigger_name, returns) in triggers {
            let n = returns.len();
            let avg_return = returns.iter().sum::<f64>() / n as f64;
            trigger_stats.insert(
                trigger_name,
                TriggerStat {
                    count: n,
                    pct_of_regime: n as f64 / total_for_candidate,
                    avg_60d_return: avg_return,
                },
            );
        }
        result.insert(candidate, trigger_stats);
    }

    TriggerAttributionReport {
        scope: scope.to_string(),
        trigger_breakdown: result,
    }
}

// ------------------------------------------------------------------
// Audit 3: Confusion Against Returns — where do top/bottom returns fall?
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ConfusionReport {
    pub scope: String,
    pub top_quartile_distribution: HashMap<String, f64>,
    pub bottom_quartile_distribution: HashMap<String, f64>,
    pub top_quartile_return: f64,
    pub bottom_quartile_return: f64,
}

pub fn audit_confusion_against_returns(
    observations: &[MarketStateObservation],
    bars: &[DailyBar],
    scope: &str,
) -> ConfusionReport {
    let mut labeled_returns: Vec<(RegimeCandidate, f64)> = Vec::new();

    for (idx, obs) in observations.iter().enumerate() {
        let (candidate, _) = CandidateGenerator::generate(obs);
        let forward_60 = if idx + 60 < bars.len() {
            (bars[idx + 60].close - bars[idx].close) / bars[idx].close
        } else {
            f64::NAN
        };
        if !forward_60.is_nan() {
            labeled_returns.push((candidate, forward_60));
        }
    }

    // Sort by return
    labeled_returns.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let n = labeled_returns.len();
    let quartile_size = n / 4;

    // Top quartile (best returns)
    let top_quartile = &labeled_returns[n - quartile_size..];
    let mut top_dist: HashMap<String, usize> = HashMap::new();
    for (candidate, _) in top_quartile {
        *top_dist.entry(format!("{:?}", candidate)).or_insert(0) += 1;
    }

    // Bottom quartile (worst returns)
    let bottom_quartile = &labeled_returns[..quartile_size];
    let mut bottom_dist: HashMap<String, usize> = HashMap::new();
    for (candidate, _) in bottom_quartile {
        *bottom_dist.entry(format!("{:?}", candidate)).or_insert(0) += 1;
    }

    let top_total = top_quartile.len() as f64;
    let bottom_total = bottom_quartile.len() as f64;

    ConfusionReport {
        scope: scope.to_string(),
        top_quartile_distribution: top_dist
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / top_total))
            .collect(),
        bottom_quartile_distribution: bottom_dist
            .iter()
            .map(|(k, v)| (k.clone(), *v as f64 / bottom_total))
            .collect(),
        top_quartile_return: top_quartile.last().map(|(_, r)| *r).unwrap_or(0.0),
        bottom_quartile_return: bottom_quartile.first().map(|(_, r)| *r).unwrap_or(0.0),
    }
}

// Helpers (copied from CandidateGenerator for self-containment)
fn score_risk_on(obs: &MarketStateObservation) -> f64 {
    let trend_ok = match (obs.trend.short_term, obs.trend.medium_term) {
        (TrendDirection::StrongUptrend, TrendDirection::StrongUptrend) => 100.0,
        (TrendDirection::StrongUptrend, TrendDirection::Uptrend)
        | (TrendDirection::Uptrend, TrendDirection::StrongUptrend) => 85.0,
        (TrendDirection::Uptrend, TrendDirection::Uptrend) => 70.0,
        (TrendDirection::Uptrend, _) | (_, TrendDirection::Uptrend) => 50.0,
        _ => 0.0,
    };

    let vol_ok = match obs.volatility.volatility_regime {
        VolatilityRegime::Low => 100.0,
        VolatilityRegime::Normal => 80.0,
        VolatilityRegime::Elevated => 40.0,
        VolatilityRegime::Spike => 0.0,
    };

    let drawdown_ok = if obs.drawdown_pct > -5.0 {
        100.0
    } else if obs.drawdown_pct > -10.0 {
        80.0
    } else if obs.drawdown_pct > -15.0 {
        50.0
    } else {
        0.0
    };

    trend_ok * 0.40 + vol_ok * 0.30 + drawdown_ok * 0.30
}

fn score_risk_off(obs: &MarketStateObservation) -> f64 {
    let trend_bad = match (obs.trend.short_term, obs.trend.medium_term) {
        (TrendDirection::StrongDowntrend, _) | (_, TrendDirection::StrongDowntrend) => 100.0,
        (TrendDirection::Downtrend, _) | (_, TrendDirection::Downtrend) => 70.0,
        _ => 0.0,
    };

    let vol_bad = match obs.volatility.volatility_regime {
        VolatilityRegime::Spike => 100.0,
        VolatilityRegime::Elevated => 70.0,
        VolatilityRegime::Normal => 20.0,
        VolatilityRegime::Low => 0.0,
    };

    let drawdown_bad = if obs.drawdown_pct < -25.0 {
        100.0
    } else if obs.drawdown_pct < -20.0 {
        85.0
    } else if obs.drawdown_pct < -15.0 {
        60.0
    } else if obs.drawdown_pct < -10.0 {
        30.0
    } else {
        0.0
    };

    let max_factor = f64::max(f64::max(trend_bad, vol_bad), drawdown_bad);
    let factor_count = (if trend_bad > 50.0 { 1 } else { 0 })
        + (if vol_bad > 50.0 { 1 } else { 0 })
        + (if drawdown_bad > 50.0 { 1 } else { 0 });

    let boost = match factor_count {
        2 => 10.0,
        3 => 20.0,
        _ => 0.0,
    };

    (max_factor + boost).min(100.0)
}

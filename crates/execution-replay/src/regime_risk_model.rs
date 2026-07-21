//! EXPERIMENT ARCHIVE — TASK-166 FAILED.
//!
//! Retained for research reference only. Do not wire into DecisionEngine or
//! extend; kept so the failed Regime-Aware State Risk approach stays
//! inspectable alongside its ADR trail.
#![allow(unused)]

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ExecutionResearchRecord;

/// TASK-166: Regime-Aware State Risk Model.
///
/// Unlike HoldingRiskScore (Transition Detector), this model identifies when the
/// market is ALREADY in a dangerous state (State Detector). It is designed to
/// complement the Transition Detector in RiskOff / Bearish regimes where the
/// Transition Detector produces no signals.
///
/// Components:
/// - TrendBreakdown: price below MA20 and MA60, and MA60 slope < 0
/// - VolatilityExpansion: amplitude_pct percentile > 70% over trailing 60 days
/// - MarketBreadthCollapse: breadth_pct < 30% (state, not transition)
/// - LiquidityStress: volume_ratio < 0.6 (state, not transition)
///
/// Research-only; does not modify the Execution Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeRiskAnalysis {
    pub total_records: usize,
    pub regime_distribution: Vec<RegimeDistribution>,
    pub state_risk_buckets: Vec<StateRiskBucket>,
    pub regime_classification: Vec<RegimeClassification>,
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeDistribution {
    pub regime: String,
    pub count: usize,
    pub negative_t60_rate: f64,
    pub avg_t60: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRiskBucket {
    pub score_label: String,
    pub min_score: f64,
    pub max_score: f64,
    pub count: usize,
    pub negative_t60_rate: f64,
    pub baseline_negative_rate: f64,
    pub lift: f64,
    pub precision: f64,
    pub avg_t60: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeClassification {
    pub regime: String,
    pub count: usize,
    pub negative_t60_rate: f64,
    pub avg_t60: f64,
    pub recall: f64,
}

/// Computes the Regime Risk Analysis.
pub fn compute_regime_risk_analysis(records: &[ExecutionResearchRecord]) -> RegimeRiskAnalysis {
    let total_records = records.len();
    let baseline = compute_baseline(records);

    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut scored_records: Vec<(f64, &ExecutionResearchRecord)> = Vec::new();
    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let score = compute_regime_risk_score(record, *date, by_date);
            scored_records.push((score, record));
        }
    }

    let regime_distribution = compute_regime_distribution(records);
    let state_risk_buckets = compute_state_risk_buckets(&scored_records, baseline.0);
    let regime_classification = compute_regime_classification(&scored_records, baseline.0);

    let verdict = build_verdict(&state_risk_buckets, &regime_classification);

    RegimeRiskAnalysis {
        total_records,
        regime_distribution,
        state_risk_buckets,
        regime_classification,
        verdict,
    }
}

/// Computes the Regime Risk Score for a single record.
///
/// Score formula (0-4):
/// - TrendBreakdown: 1.0 if price < MA20 and MA60, and MA60 slope < 0
/// - VolatilityExpansion: 1.0 if amplitude_pct > 70th percentile over trailing 60 days
/// - MarketBreadthCollapse: 1.0 if breadth_pct < 30
/// - LiquidityStress: 1.0 if volume_ratio < 0.6
pub fn compute_regime_risk_score(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> f64 {
    let mut score = 0.0;

    if is_trend_breakdown(record, date, by_date) {
        score += 1.0;
    }
    if is_volatility_expansion(record, date, by_date) {
        score += 1.0;
    }
    if is_market_breadth_collapse(record) {
        score += 1.0;
    }
    if is_liquidity_stress(record) {
        score += 1.0;
    }
    score
}

fn is_trend_breakdown(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> bool {
    let close = record.event.request.quote.close;
    let ma20 = compute_moving_average(by_date, date, 20);
    let ma60 = compute_moving_average(by_date, date, 60);
    let ma60_slope = compute_moving_average_slope(by_date, date, 60, 20);

    close < ma20 && close < ma60 && ma60_slope < 0.0
}

fn compute_moving_average(
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    date: NaiveDate,
    window: usize,
) -> f64 {
    let mut sum = 0.0;
    let mut count = 0usize;
    for (_d, r) in by_date.range(..date).rev() {
        sum += r.event.request.quote.close;
        count += 1;
        if count >= window {
            break;
        }
    }
    if count == 0 {
        return 0.0;
    }
    sum / count as f64
}

fn compute_moving_average_slope(
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    date: NaiveDate,
    window: usize,
    lag: usize,
) -> f64 {
    let current = compute_moving_average(by_date, date, window);
    let past_date = date.checked_sub_signed(chrono::Duration::days(lag as i64));
    let past = past_date.map(|d| compute_moving_average(by_date, d, window)).unwrap_or(0.0);
    current - past
}

fn is_volatility_expansion(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> bool {
    let current_amp = record.event.features.amplitude_pct;
    let mut amps = Vec::new();
    let mut count = 0usize;
    for (_d, r) in by_date.range(..date).rev() {
        amps.push(r.event.features.amplitude_pct);
        count += 1;
        if count >= 60 {
            break;
        }
    }
    if amps.is_empty() {
        return false;
    }
    amps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let percentile_70 = amps[(amps.len() as f64 * 0.70) as usize];
    current_amp > percentile_70
}

fn is_market_breadth_collapse(record: &ExecutionResearchRecord) -> bool {
    record.event.request.market_view.breadth.breadth_pct < 30.0
}

fn is_liquidity_stress(record: &ExecutionResearchRecord) -> bool {
    let volume = record.event.request.quote.volume;
    let volume_ma20 = record.event.request.volume_ma20;
    let ratio = if volume_ma20 > 1e-9 {
        volume / volume_ma20
    } else {
        1.0
    };
    ratio < 0.6
}

fn compute_baseline(records: &[ExecutionResearchRecord]) -> (f64, f64) {
    let mut negatives = 0usize;
    let mut count = 0usize;
    let mut sum = 0.0;
    for r in records {
        if let Some(t60) = r.outcome.t60_return {
            count += 1;
            sum += t60;
            if t60 < 0.0 {
                negatives += 1;
            }
        }
    }
    let rate = safe_rate(negatives, count);
    let avg = safe_avg(sum, count);
    (rate, avg)
}

fn compute_regime_distribution(records: &[ExecutionResearchRecord]) -> Vec<RegimeDistribution> {
    let mut regime_map: BTreeMap<String, Vec<&ExecutionResearchRecord>> = BTreeMap::new();
    for r in records {
        let regime = classify_regime(&r.event.request.market_view.market_regime_label);
        regime_map.entry(regime).or_default().push(r);
    }

    let mut results = Vec::new();
    for (regime, items) in regime_map {
        let mut negatives = 0usize;
        let mut count = 0usize;
        let mut sum = 0.0;
        for r in &items {
            if let Some(t60) = r.outcome.t60_return {
                count += 1;
                sum += t60;
                if t60 < 0.0 {
                    negatives += 1;
                }
            }
        }
        results.push(RegimeDistribution {
            regime,
            count: items.len(),
            negative_t60_rate: safe_rate(negatives, count),
            avg_t60: safe_avg(sum, count),
        });
    }
    results
}

fn classify_regime(label: &str) -> String {
    let lower = label.to_lowercase();
    if lower.contains("risk_on") || lower.contains("bull") {
        "RiskOn".into()
    } else if lower.contains("risk_off") || lower.contains("bear") {
        "RiskOff".into()
    } else {
        "Neutral".into()
    }
}

fn compute_state_risk_buckets(
    scored_records: &[(f64, &ExecutionResearchRecord)],
    baseline_rate: f64,
) -> Vec<StateRiskBucket> {
    let mut buckets = Vec::new();
    for (label, min, max) in [
        ("score 0.0", 0.0, 0.01),
        ("score (0, 1.0)", 0.01, 1.01),
        ("score [1.0, 2.0)", 1.00, 2.01),
        ("score [2.0, 3.0)", 2.00, 3.01),
        ("score [3.0, 4.0)", 3.00, 4.01),
        ("score >= 4.0", 4.00, 10.0),
    ] {
        let bucket_records: Vec<&ExecutionResearchRecord> = scored_records
            .iter()
            .filter(|(score, _)| *score >= min && *score < max)
            .map(|(_, r)| *r)
            .collect();
        buckets.push(build_state_risk_bucket(label, min, max, &bucket_records, baseline_rate));
    }
    buckets
}

fn build_state_risk_bucket(
    label: &str,
    min: f64,
    max: f64,
    records: &[&ExecutionResearchRecord],
    baseline_rate: f64,
) -> StateRiskBucket {
    let mut negatives = 0usize;
    let mut count = 0usize;
    let mut sum = 0.0;
    for r in records {
        if let Some(t60) = r.outcome.t60_return {
            count += 1;
            sum += t60;
            if t60 < 0.0 {
                negatives += 1;
            }
        }
    }
    let negative_rate = safe_rate(negatives, count);
    let lift = if baseline_rate > 0.0 {
        negative_rate / baseline_rate
    } else {
        0.0
    };

    StateRiskBucket {
        score_label: label.to_string(),
        min_score: min,
        max_score: max,
        count: records.len(),
        negative_t60_rate: negative_rate,
        baseline_negative_rate: baseline_rate,
        lift,
        precision: negative_rate,
        avg_t60: safe_avg(sum, count),
    }
}

fn compute_regime_classification(
    scored_records: &[(f64, &ExecutionResearchRecord)],
    baseline_rate: f64,
) -> Vec<RegimeClassification> {
    let mut regime_map: BTreeMap<String, Vec<(f64, &ExecutionResearchRecord)>> = BTreeMap::new();
    for (score, record) in scored_records {
        let regime = classify_regime(&record.event.request.market_view.market_regime_label);
        regime_map.entry(regime).or_default().push((*score, *record));
    }

    let mut results = Vec::new();
    for (regime, items) in regime_map {
        let count = items.len();
        let mut negatives = 0usize;
        let mut total = 0usize;
        let mut sum = 0.0;
        let mut high_risk_detected = 0usize;

        for (score, record) in &items {
            if let Some(t60) = record.outcome.t60_return {
                total += 1;
                sum += t60;
                if t60 < 0.0 {
                    negatives += 1;
                }
                if *score >= 2.0 {
                    high_risk_detected += 1;
                }
            }
        }

        let negative_rate = safe_rate(negatives, total);
        let recall = if total > 0 {
            high_risk_detected as f64 / total as f64
        } else {
            0.0
        };

        results.push(RegimeClassification {
            regime,
            count,
            negative_t60_rate: negative_rate,
            avg_t60: safe_avg(sum, total),
            recall,
        });
    }
    results
}

fn build_verdict(
    buckets: &[StateRiskBucket],
    classification: &[RegimeClassification],
) -> String {
    let mut lines = Vec::new();
    lines.push("Regime-Aware State Risk Model analysis:".to_string());

    let best = buckets
        .iter()
        .filter(|b| b.count >= 30 && b.precision >= 0.60 && b.lift >= 1.2)
        .max_by(|a, b| {
            a.lift
                .partial_cmp(&b.lift)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    if let Some(b) = best {
        lines.push(format!(
            "Best bucket: {} — n={}, precision={:.1}%, lift={:.2}",
            b.score_label,
            b.count,
            b.precision * 100.0,
            b.lift
        ));
    } else {
        lines.push("No state risk bucket meets calibration gate (sample >= 30, precision >= 60%, lift >= 1.2).".into());
    }

    let risk_off = classification
        .iter()
        .find(|r| r.regime == "RiskOff")
        .map(|r| r.recall);
    if let Some(recall) = risk_off {
        lines.push(format!("RiskOff recall (score >= 2.0): {:.1}%", recall * 100.0));
        if recall >= 0.70 {
            lines.push("Regime classification meets acceptance gate (recall >= 70%).".into());
        } else {
            lines.push("Regime classification does not meet acceptance gate (recall < 70%).".into());
        }
    } else {
        lines.push("No RiskOff regime found in dataset.".into());
    }

    lines.join("\n")
}

fn safe_rate(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn safe_avg(sum: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_on_empty_records() {
        let (rate, avg) = compute_baseline(&[]);
        assert_eq!(rate, 0.0);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn regime_classification() {
        assert_eq!(classify_regime("risk_on"), "RiskOn");
        assert_eq!(classify_regime("risk_off"), "RiskOff");
        assert_eq!(classify_regime("neutral"), "Neutral");
    }
}

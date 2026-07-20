use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{
    confirmation_decay::detect_confirmation_decay_v4,
    holding_risk_bundle::detect_liquidity_pressure_v3,
    transition_analysis::detect_leadership_decay,
    ExecutionResearchRecord,
};

/// TASK-161: Holding Risk Calibration v2.
///
/// Defines `HoldingRiskScore` as a weighted combination of validated Evidence
/// dimensions and validates it as a stable Research Asset at T+60. Includes
/// score bucket analysis, regime split, and walk-forward validation.
/// Research-only; does not modify the Execution Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingRiskCalibrationAnalysis {
    pub total_records: usize,
    pub baseline_negative_t60_rate: f64,
    pub score_buckets: Vec<ScoreBucketStats>,
    pub regime_buckets: Vec<RegimeBucketStats>,
    pub walk_forward: WalkForwardStats,
    pub verdict: String,
}

/// T+60 performance per HoldingRiskScore bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBucketStats {
    pub score_label: String,
    pub min_score: f64,
    pub max_score: f64,
    pub count: usize,
    pub negative_t60_rate: f64,
    pub baseline_negative_rate: f64,
    pub lift: f64,
    pub precision: f64,
    pub avg_t60: f64,
    pub median_t60: f64,
    pub false_reduce_rate: f64,
}

/// T+60 performance per market regime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegimeBucketStats {
    pub regime: String,
    pub count: usize,
    pub high_risk_count: usize,
    pub high_risk_precision: f64,
    pub high_risk_lift: f64,
    pub baseline_negative_rate: f64,
}

/// Walk-forward validation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardStats {
    pub train_period: String,
    pub validate_period: String,
    pub train_high_risk_count: usize,
    pub train_precision: f64,
    pub train_lift: f64,
    pub validate_high_risk_count: usize,
    pub validate_precision: f64,
    pub validate_lift: f64,
    pub precision_decay: f64,
}

/// Computes the HoldingRiskScore for a single record.
///
/// Score formula:
/// - LeadershipDecayPersistence (>=5 days): 0.5
/// - LiquidityPressure (any volume decline, >=3 days): 0.25
/// - ConfirmationDecay (delta_5d < -5 or consecutive >= 2): 0.25
pub fn compute_holding_risk_score(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> f64 {
    let leadership = detect_leadership_decay(record, date, by_date);
    let leadership_flag = leadership.is_leadership_decay()
        && leadership.consecutive_decline_days >= 5;
    let liquidity = detect_liquidity_pressure_v3(record, date, by_date);
    let confirmation = detect_confirmation_decay_v4(record, date, by_date);

    let mut score = 0.0;
    if leadership_flag {
        score += 0.5;
    }
    if liquidity {
        score += 0.25;
    }
    if confirmation {
        score += 0.25;
    }
    score
}

/// Computes the Holding Risk Calibration v2 analysis.
pub fn compute_holding_risk_calibration(
    records: &[ExecutionResearchRecord],
) -> HoldingRiskCalibrationAnalysis {
    let total_records = records.len();
    let baseline_rate = compute_baseline(records);

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
            let score = compute_holding_risk_score(record, *date, by_date);
            scored_records.push((score, record));
        }
    }

    let score_buckets = compute_score_buckets(&scored_records, baseline_rate);
    let regime_buckets = compute_regime_buckets(&scored_records, baseline_rate);
    let walk_forward = compute_walk_forward(&scored_records, baseline_rate);

    let verdict = build_verdict(&score_buckets, &regime_buckets, &walk_forward);

    HoldingRiskCalibrationAnalysis {
        total_records,
        baseline_negative_t60_rate: baseline_rate,
        score_buckets,
        regime_buckets,
        walk_forward,
        verdict,
    }
}

fn compute_baseline(records: &[ExecutionResearchRecord]) -> f64 {
    let mut negatives = 0usize;
    let mut count = 0usize;
    for r in records {
        if let Some(t60) = r.outcome.t60_return {
            count += 1;
            if t60 < 0.0 {
                negatives += 1;
            }
        }
    }
    safe_rate(negatives, count)
}

fn compute_score_buckets(
    scored_records: &[(f64, &ExecutionResearchRecord)],
    baseline_rate: f64,
) -> Vec<ScoreBucketStats> {
    let mut buckets = Vec::new();
    for (label, min, max) in [
        ("score 0.0", 0.0, 0.01),
        ("score (0, 0.25)", 0.01, 0.251),
        ("score [0.25, 0.5)", 0.25, 0.501),
        ("score [0.5, 0.75)", 0.50, 0.751),
        ("score [0.75, 1.0)", 0.75, 1.001),
        ("score >= 1.0", 1.00, 10.0),
    ] {
        let bucket_records: Vec<&ExecutionResearchRecord> = scored_records
            .iter()
            .filter(|(score, _)| *score >= min && *score < max)
            .map(|(_, r)| *r)
            .collect();
        buckets.push(build_score_bucket(label, min, max, &bucket_records, baseline_rate));
    }
    buckets
}

fn build_score_bucket(
    label: &str,
    min: f64,
    max: f64,
    records: &[&ExecutionResearchRecord],
    baseline_rate: f64,
) -> ScoreBucketStats {
    let mut negatives = 0usize;
    let mut count = 0usize;
    let mut sum = 0.0;
    let mut values = Vec::new();
    for r in records {
        if let Some(t60) = r.outcome.t60_return {
            count += 1;
            sum += t60;
            values.push(t60);
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
    let precision = negative_rate;
    let avg = safe_avg(sum, count);
    let median = median(&values);
    let false_reduce = if count > 0 {
        values.iter().filter(|&&v| v >= 0.0).count() as f64 / count as f64
    } else {
        0.0
    };

    ScoreBucketStats {
        score_label: label.to_string(),
        min_score: min,
        max_score: max,
        count: records.len(),
        negative_t60_rate: negative_rate,
        baseline_negative_rate: baseline_rate,
        lift,
        precision,
        avg_t60: avg,
        median_t60: median,
        false_reduce_rate: false_reduce,
    }
}

fn compute_regime_buckets(
    scored_records: &[(f64, &ExecutionResearchRecord)],
    baseline_rate: f64,
) -> Vec<RegimeBucketStats> {
    let mut regime_map: BTreeMap<String, Vec<(f64, &ExecutionResearchRecord)>> = BTreeMap::new();
    for (score, record) in scored_records {
        let regime = classify_regime(&record.event.request.market_view.market_regime_label);
        regime_map.entry(regime).or_default().push((*score, *record));
    }

    let mut results = Vec::new();
    for (regime, items) in regime_map {
        let count = items.len();
        let high_risk: Vec<&ExecutionResearchRecord> = items
            .iter()
            .filter(|(score, _)| *score >= 0.75)
            .map(|(_, r)| *r)
            .collect();
        let high_risk_count = high_risk.len();
        let (neg, total) = high_risk
            .iter()
            .filter_map(|r| r.outcome.t60_return)
            .fold((0, 0), |(neg, total), t60| {
                (neg + (t60 < 0.0) as usize, total + 1)
            });
        let precision = safe_rate(neg, total);
        let lift = if baseline_rate > 0.0 {
            precision / baseline_rate
        } else {
            0.0
        };

        results.push(RegimeBucketStats {
            regime,
            count,
            high_risk_count,
            high_risk_precision: precision,
            high_risk_lift: lift,
            baseline_negative_rate: baseline_rate,
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

fn compute_walk_forward(
    scored_records: &[(f64, &ExecutionResearchRecord)],
    baseline_rate: f64,
) -> WalkForwardStats {
    let train_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let train_end = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let validate_start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let validate_end = NaiveDate::from_ymd_opt(2025, 6, 30).unwrap();

    let train: Vec<&ExecutionResearchRecord> = scored_records
        .iter()
        .filter(|(score, r)| {
            let d = r.event.date();
            *score >= 0.75 && d >= train_start && d <= train_end
        })
        .map(|(_, r)| *r)
        .collect();
    let validate: Vec<&ExecutionResearchRecord> = scored_records
        .iter()
        .filter(|(score, r)| {
            let d = r.event.date();
            *score >= 0.75 && d >= validate_start && d <= validate_end
        })
        .map(|(_, r)| *r)
        .collect();

    let (train_neg, train_total) = train
        .iter()
        .filter_map(|r| r.outcome.t60_return)
        .fold((0, 0), |(neg, total), t60| (neg + (t60 < 0.0) as usize, total + 1));
    let train_precision = safe_rate(train_neg, train_total);
    let train_lift = if baseline_rate > 0.0 {
        train_precision / baseline_rate
    } else {
        0.0
    };

    let (validate_neg, validate_total) = validate
        .iter()
        .filter_map(|r| r.outcome.t60_return)
        .fold((0, 0), |(neg, total), t60| (neg + (t60 < 0.0) as usize, total + 1));
    let validate_precision = safe_rate(validate_neg, validate_total);
    let validate_lift = if baseline_rate > 0.0 {
        validate_precision / baseline_rate
    } else {
        0.0
    };

    let precision_decay = if train_precision > 0.0 {
        (train_precision - validate_precision) / train_precision
    } else {
        0.0
    };

    WalkForwardStats {
        train_period: format!("{} to {}", train_start, train_end),
        validate_period: format!("{} to {}", validate_start, validate_end),
        train_high_risk_count: train.len(),
        train_precision,
        train_lift,
        validate_high_risk_count: validate.len(),
        validate_precision,
        validate_lift,
        precision_decay,
    }
}

fn build_verdict(
    score_buckets: &[ScoreBucketStats],
    regime_buckets: &[RegimeBucketStats],
    walk_forward: &WalkForwardStats,
) -> String {
    let best = score_buckets
        .iter()
        .filter(|b| b.count >= 30 && b.precision >= 0.60 && b.lift >= 1.3)
        .max_by(|a, b| {
            a.lift
                .partial_cmp(&b.lift)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    let mut lines = Vec::new();
    lines.push("Holding Risk Calibration v2 analysis:".to_string());

    if let Some(b) = best {
        lines.push(format!(
            "Best bucket: {} — n={}, precision={:.1}%, lift={:.2}",
            b.score_label,
            b.count,
            b.precision * 100.0,
            b.lift
        ));
    } else {
        lines.push("No score bucket meets calibration gate (sample >= 30, precision >= 60%, lift >= 1.3).".into());
    }

    lines.push(format!(
        "Walk-forward: train precision={:.1}% -> validate precision={:.1}% (decay={:.1}%)",
        walk_forward.train_precision * 100.0,
        walk_forward.validate_precision * 100.0,
        walk_forward.precision_decay * 100.0
    ));

    let regime_stable = regime_buckets
        .iter()
        .filter(|r| r.high_risk_count >= 30)
        .all(|r| r.high_risk_precision >= 0.55);
    if regime_stable {
        lines.push("Regime stability: PASS (high-risk precision >= 55% in all major regimes)".into());
    } else {
        lines.push("Regime stability: FAIL (high-risk precision < 55% in at least one regime)".into());
    }

    if best.is_some() && regime_stable && walk_forward.precision_decay < 0.20 {
        lines.push("HoldingRiskScore meets Calibration v2 acceptance gate.".into());
    } else {
        lines.push("HoldingRiskScore does not yet meet Calibration v2 acceptance gate.".into());
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

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_on_empty_records() {
        let rate = compute_baseline(&[]);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn regime_classification() {
        assert_eq!(classify_regime("risk_on"), "RiskOn");
        assert_eq!(classify_regime("bullish"), "RiskOn");
        assert_eq!(classify_regime("risk_off"), "RiskOff");
        assert_eq!(classify_regime("bearish"), "RiskOff");
        assert_eq!(classify_regime("neutral"), "Neutral");
        assert_eq!(classify_regime("DeRisk"), "Neutral");
    }
}

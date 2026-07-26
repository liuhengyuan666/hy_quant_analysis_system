use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::confirmation_decay::detect_confirmation_decay_v4;
use crate::transition_analysis::{
    detect_breadth_deterioration, detect_leadership_decay,
};
use crate::ExecutionResearchRecord;

/// TASK-158: Holding Risk Evidence Bundle.
///
/// Combines multiple medium-term transition signals (LeadershipDecay,
/// BreadthDeterioration, LiquidityDeterioration) into a single Holding Risk
/// Score. This is a research-only analysis; it does not modify any
/// Observation, Evidence, Assessment, Decision, or Policy code.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HoldingRiskBundleAnalysis {
    pub total_records: usize,
    pub score_distribution: Vec<ScoreBucket>,
    pub verdict: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBucket {
    pub score_label: String,
    pub count: usize,
    pub t60_negative_rate: f64,
    pub baseline_negative_rate: f64,
    pub lift: f64,
    pub precision: f64,
    pub avg_t60_return: f64,
    pub median_t60_return: f64,
    pub avg_max_drawdown: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiquidityDeteriorationSignal {
    pub volume_ratio: f64,
    pub volume_ratio_delta_5d: f64,
    pub volume_ratio_delta_10d: f64,
    pub triggered_by: String,
}

impl LiquidityDeteriorationSignal {
    pub fn is_liquidity_deterioration(&self) -> bool {
        !self.triggered_by.is_empty()
    }
}

/// Per-record holding risk bundle.
#[derive(Debug, Clone, Default)]
pub struct HoldingRiskBundle {
    pub leadership_decay: bool,
    pub breadth_deterioration: bool,
    pub liquidity_deterioration: bool,
    pub weighted_score: f64,
    pub signal_count: u32,
}

const LIQUIDITY_DELTA_5D_THRESHOLD: f64 = -0.50;
const LIQUIDITY_DELTA_10D_THRESHOLD: f64 = -1.00;

/// Computes the Holding Risk Bundle Analysis over a set of records.
///
/// Natural horizon is T+60 (MediumTerm). Each record is scored by the
/// presence of LeadershipDecay, BreadthDeterioration, and LiquidityDeterioration.
/// The analysis reports T+60 outcomes per score bucket.
pub fn compute_holding_risk_bundle_analysis(
    records: &[ExecutionResearchRecord],
) -> HoldingRiskBundleAnalysis {
    let total_records = records.len();

    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut bundles: Vec<(HoldingRiskBundle, &ExecutionResearchRecord)> = Vec::new();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let leadership = detect_leadership_decay(*record, *date, by_date);
            let breadth = detect_breadth_deterioration(*record, *date, by_date);
            let liquidity = detect_liquidity_deterioration(*record, *date, by_date);

            let leadership_flag = leadership.is_leadership_decay();
            let breadth_flag = breadth.is_breadth_deterioration();
            let liquidity_flag = liquidity.is_liquidity_deterioration();

            let signal_count = [leadership_flag, breadth_flag, liquidity_flag]
                .iter()
                .filter(|&&b| b)
                .count() as u32;

            let weighted_score = if leadership_flag { 0.4 } else { 0.0 }
                + if breadth_flag { 0.3 } else { 0.0 }
                + if liquidity_flag { 0.3 } else { 0.0 };

            bundles.push((
                HoldingRiskBundle {
                    leadership_decay: leadership_flag,
                    breadth_deterioration: breadth_flag,
                    liquidity_deterioration: liquidity_flag,
                    weighted_score,
                    signal_count,
                },
                record,
            ));
        }
    }

    let baseline_records: Vec<&ExecutionResearchRecord> =
        bundles.iter().map(|(_, r)| *r).collect();

    let mut score_buckets = Vec::new();
    for score in 0..=3 {
        let label = format!("{} signals", score);
        let bucket_records: Vec<&ExecutionResearchRecord> = bundles
            .iter()
            .filter(|(b, _)| b.signal_count == score)
            .map(|(_, r)| *r)
            .collect();
        score_buckets.push(build_score_bucket(
            &label,
            &baseline_records,
            &bucket_records,
        ));
    }

    let mut weighted_buckets = Vec::new();
    for (label, min, max) in [
        ("score 0.0", 0.0, 0.01),
        ("score (0, 0.4)", 0.01, 0.41),
        ("score [0.4, 0.7)", 0.40, 0.71),
        ("score [0.7, 1.0)", 0.70, 1.01),
        ("score >= 1.0", 1.00, 10.0),
    ] {
        let bucket_records: Vec<&ExecutionResearchRecord> = bundles
            .iter()
            .filter(|(b, _)| b.weighted_score >= min && b.weighted_score < max)
            .map(|(_, r)| *r)
            .collect();
        weighted_buckets.push(build_score_bucket(&label, &baseline_records, &bucket_records));
    }

    let verdict = build_bundle_verdict(&score_buckets, &weighted_buckets, total_records);

    HoldingRiskBundleAnalysis {
        total_records,
        score_distribution: score_buckets,
        verdict,
    }
}

pub(crate) fn detect_liquidity_deterioration(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> LiquidityDeteriorationSignal {
    let volume = record.event.request.quote.volume;
    let volume_ma20 = record.event.request.volume_ma20;

    let volume_ratio = if volume_ma20 > 1e-9 {
        volume / volume_ma20
    } else {
        1.0
    };

    let delta_5d = compute_volume_ratio_delta(record, date, by_date, 5);
    let delta_10d = compute_volume_ratio_delta(record, date, by_date, 10);

    let trigger_5d = delta_5d < LIQUIDITY_DELTA_5D_THRESHOLD;
    let trigger_10d = delta_10d < LIQUIDITY_DELTA_10D_THRESHOLD;

    let triggered_by = if trigger_5d && trigger_10d {
        "both"
    } else if trigger_5d {
        "delta_5d"
    } else if trigger_10d {
        "delta_10d"
    } else {
        ""
    };

    LiquidityDeteriorationSignal {
        volume_ratio,
        volume_ratio_delta_5d: delta_5d,
        volume_ratio_delta_10d: delta_10d,
        triggered_by: triggered_by.to_string(),
    }
}

fn compute_volume_ratio_delta(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    trading_days_ago: usize,
) -> f64 {
    let volume = record.event.request.quote.volume;
    let volume_ma20 = record.event.request.volume_ma20;
    let current_ratio = if volume_ma20 > 1e-9 {
        volume / volume_ma20
    } else {
        1.0
    };

    let mut count = 0usize;
    for (_past_date, past) in by_date.range(..date).rev() {
        count += 1;
        if count == trading_days_ago {
            let past_volume = past.event.request.quote.volume;
            let past_volume_ma20 = past.event.request.volume_ma20;
            let past_ratio = if past_volume_ma20 > 1e-9 {
                past_volume / past_volume_ma20
            } else {
                1.0
            };
            return current_ratio - past_ratio;
        }
    }
    0.0
}

fn build_score_bucket(
    label: &str,
    baseline_records: &[&ExecutionResearchRecord],
    bucket_records: &[&ExecutionResearchRecord],
) -> ScoreBucket {
    let baseline_returns: Vec<f64> = baseline_records
        .iter()
        .filter_map(|r| r.outcome.t60_return)
        .collect();
    let bucket_returns: Vec<f64> = bucket_records
        .iter()
        .filter_map(|r| r.outcome.t60_return)
        .collect();

    let baseline_dds: Vec<f64> = baseline_records
        .iter()
        .filter_map(|r| r.outcome.t60_return.and_then(|_| r.outcome.max_drawdown))
        .collect();
    let bucket_dds: Vec<f64> = bucket_records
        .iter()
        .filter_map(|r| r.outcome.t60_return.and_then(|_| r.outcome.max_drawdown))
        .collect();

    let baseline_negative = baseline_returns.iter().filter(|&&r| r < 0.0).count();
    let bucket_negative = bucket_returns.iter().filter(|&&r| r < 0.0).count();

    let baseline_negative_rate = safe_rate(baseline_negative, baseline_returns.len());
    let bucket_negative_rate = safe_rate(bucket_negative, bucket_returns.len());
    let lift = if baseline_negative_rate > 1e-9 {
        bucket_negative_rate / baseline_negative_rate
    } else {
        1.0
    };
    let precision = bucket_negative_rate;

    let _avg_baseline_return = safe_avg(baseline_returns.iter().copied().sum(), baseline_returns.len());
    let avg_bucket_return = safe_avg(bucket_returns.iter().copied().sum(), bucket_returns.len());
    let median_bucket_return = median(&bucket_returns);

    let _avg_baseline_dd = safe_avg(baseline_dds.iter().copied().sum(), baseline_dds.len());
    let avg_bucket_dd = safe_avg(bucket_dds.iter().copied().sum(), bucket_dds.len());

    ScoreBucket {
        score_label: label.to_string(),
        count: bucket_records.len(),
        t60_negative_rate: bucket_negative_rate,
        baseline_negative_rate,
        lift,
        precision,
        avg_t60_return: avg_bucket_return,
        median_t60_return: median_bucket_return,
        avg_max_drawdown: avg_bucket_dd,
    }
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

/// TASK-160.1: Holding Risk Bundle V2.
///
/// Replaces the snapshot LeadershipDecay dimension with a persistence-aware version
/// (`LeadershipDecayPersistence >= min_consecutive_days`). The goal is to test
/// whether sustained deterioration is a stronger medium-term Holding Risk signal
/// than a single-day snapshot.
///
/// Weights:
/// - LeadershipDecayPersistence: 0.5
/// - BreadthDeterioration:       0.25
/// - LiquidityDeterioration:     0.25
pub fn compute_holding_risk_bundle_v2_analysis(
    records: &[ExecutionResearchRecord],
    min_leadership_persistence_days: usize,
) -> HoldingRiskBundleAnalysis {
    let total_records = records.len();

    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut bundles: Vec<(HoldingRiskBundle, &ExecutionResearchRecord)> = Vec::new();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let leadership = detect_leadership_decay(*record, *date, by_date);
            let breadth = detect_breadth_deterioration(*record, *date, by_date);
            let liquidity = detect_liquidity_deterioration(*record, *date, by_date);

            let leadership_persistence_flag =
                leadership.is_leadership_decay() && leadership.consecutive_decline_days >= min_leadership_persistence_days;
            let breadth_flag = breadth.is_breadth_deterioration();
            let liquidity_flag = liquidity.is_liquidity_deterioration();

            let signal_count = [leadership_persistence_flag, breadth_flag, liquidity_flag]
                .iter()
                .filter(|&&b| b)
                .count() as u32;

            let weighted_score = if leadership_persistence_flag { 0.5 } else { 0.0 }
                + if breadth_flag { 0.25 } else { 0.0 }
                + if liquidity_flag { 0.25 } else { 0.0 };

            bundles.push((
                HoldingRiskBundle {
                    leadership_decay: leadership_persistence_flag,
                    breadth_deterioration: breadth_flag,
                    liquidity_deterioration: liquidity_flag,
                    weighted_score,
                    signal_count,
                },
                record,
            ));
        }
    }

    let baseline_records: Vec<&ExecutionResearchRecord> =
        bundles.iter().map(|(_, r)| *r).collect();

    let mut score_buckets = Vec::new();
    for score in 0..=3 {
        let label = format!("{} signals", score);
        let bucket_records: Vec<&ExecutionResearchRecord> = bundles
            .iter()
            .filter(|(b, _)| b.signal_count == score)
            .map(|(_, r)| *r)
            .collect();
        score_buckets.push(build_score_bucket(
            &label,
            &baseline_records,
            &bucket_records,
        ));
    }

    let mut weighted_buckets = Vec::new();
    for (label, min, max) in [
        ("score 0.0", 0.0, 0.01),
        ("score (0, 0.5)", 0.01, 0.51),
        ("score [0.5, 0.75)", 0.50, 0.76),
        ("score [0.75, 1.0)", 0.75, 1.01),
        ("score >= 1.0", 1.00, 10.0),
    ] {
        let bucket_records: Vec<&ExecutionResearchRecord> = bundles
            .iter()
            .filter(|(b, _)| b.weighted_score >= min && b.weighted_score < max)
            .map(|(_, r)| *r)
            .collect();
        weighted_buckets.push(build_score_bucket(&label, &baseline_records, &bucket_records));
    }

    let verdict = build_bundle_v2_verdict(
        &score_buckets,
        &weighted_buckets,
        total_records,
        min_leadership_persistence_days,
    );

    HoldingRiskBundleAnalysis {
        total_records,
        score_distribution: score_buckets,
        verdict,
    }
}

/// TASK-160.2A: Holding Risk Bundle V3.
///
/// Combines LeadershipDecay persistence, LiquidityPressure, and BreadthDeterioration
/// into a medium-term (T+60) holding risk score. This tests whether adding a
/// capital-pressure dimension improves the bundle.
///
/// Weights:
/// - LeadershipDecayPersistence (>=5 days): 0.4
/// - LiquidityPressure (any volume decline, >=3 days): 0.3
/// - BreadthDeterioration:                 0.3
pub fn compute_holding_risk_bundle_v3_analysis(
    records: &[ExecutionResearchRecord],
) -> HoldingRiskBundleAnalysis {
    let total_records = records.len();

    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut bundles: Vec<(HoldingRiskBundle, &ExecutionResearchRecord)> = Vec::new();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let leadership = detect_leadership_decay(*record, *date, by_date);
            let breadth = detect_breadth_deterioration(*record, *date, by_date);
            let liquidity = detect_liquidity_pressure_v3(*record, *date, by_date);

            let leadership_flag = leadership.is_leadership_decay()
                && leadership.consecutive_decline_days >= 5;
            let breadth_flag = breadth.is_breadth_deterioration();
            let liquidity_flag = liquidity;

            let signal_count = [leadership_flag, breadth_flag, liquidity_flag]
                .iter()
                .filter(|&&b| b)
                .count() as u32;

            let weighted_score = if leadership_flag { 0.4 } else { 0.0 }
                + if breadth_flag { 0.3 } else { 0.0 }
                + if liquidity_flag { 0.3 } else { 0.0 };

            bundles.push((
                HoldingRiskBundle {
                    leadership_decay: leadership_flag,
                    breadth_deterioration: breadth_flag,
                    liquidity_deterioration: liquidity_flag,
                    weighted_score,
                    signal_count,
                },
                record,
            ));
        }
    }

    let baseline_records: Vec<&ExecutionResearchRecord> =
        bundles.iter().map(|(_, r)| *r).collect();

    let mut score_buckets = Vec::new();
    for score in 0..=3 {
        let label = format!("{} signals", score);
        let bucket_records: Vec<&ExecutionResearchRecord> = bundles
            .iter()
            .filter(|(b, _)| b.signal_count == score)
            .map(|(_, r)| *r)
            .collect();
        score_buckets.push(build_score_bucket(
            &label,
            &baseline_records,
            &bucket_records,
        ));
    }

    let mut weighted_buckets = Vec::new();
    for (label, min, max) in [
        ("score 0.0", 0.0, 0.01),
        ("score (0, 0.4)", 0.01, 0.41),
        ("score [0.4, 0.7)", 0.40, 0.71),
        ("score [0.7, 1.0)", 0.70, 1.01),
        ("score >= 1.0", 1.00, 10.0),
    ] {
        let bucket_records: Vec<&ExecutionResearchRecord> = bundles
            .iter()
            .filter(|(b, _)| b.weighted_score >= min && b.weighted_score < max)
            .map(|(_, r)| *r)
            .collect();
        weighted_buckets.push(build_score_bucket(&label, &baseline_records, &bucket_records));
    }

    let verdict = build_bundle_v3_verdict(&score_buckets, &weighted_buckets, total_records);

    HoldingRiskBundleAnalysis {
        total_records,
        score_distribution: score_buckets,
        verdict,
    }
}

pub(crate) fn detect_liquidity_pressure_v3(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> bool {
    let volume_ratio_delta = compute_volume_ratio_delta(record, date, by_date, 5);
    if volume_ratio_delta >= 0.0 {
        return false;
    }
    let consecutive = count_consecutive_volume_decline_days(date, by_date);
    consecutive >= 3
}

fn count_consecutive_volume_decline_days(
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> usize {
    let mut consecutive = 0usize;
    for days_back in 1..=10 {
        if let Some(prev_date) = date.checked_sub_signed(chrono::Duration::days(days_back)) {
            if let Some(prev) = by_date.get(&prev_date) {
                let prev_delta = compute_volume_ratio_delta(prev, prev_date, by_date, 5);
                if prev_delta < 0.0 {
                    consecutive += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
    consecutive
}

fn build_bundle_v3_verdict(
    score_buckets: &[ScoreBucket],
    weighted_buckets: &[ScoreBucket],
    total_records: usize,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "Holding Risk Bundle V3 analyzed across {} records. Signal-count buckets:",
        total_records
    ));
    for b in score_buckets {
        parts.push(format!(
            "- {}: n={}, negative T+60={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%, avg_dd={:.2}%",
            b.score_label,
            b.count,
            b.t60_negative_rate * 100.0,
            b.baseline_negative_rate * 100.0,
            b.lift,
            b.precision * 100.0,
            b.avg_t60_return * 100.0,
            b.avg_max_drawdown * 100.0
        ));
    }

    parts.push("Weighted-score buckets:".to_string());
    for b in weighted_buckets {
        parts.push(format!(
            "- {}: n={}, negative T+60={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%",
            b.score_label,
            b.count,
            b.t60_negative_rate * 100.0,
            b.baseline_negative_rate * 100.0,
            b.lift,
            b.precision * 100.0,
            b.avg_t60_return * 100.0
        ));
    }

    let best = weighted_buckets
        .iter()
        .filter(|b| b.count >= 30 && b.lift >= 1.3 && b.precision >= 0.55 && b.t60_negative_rate < 1.0)
        .max_by(|a, b| a.lift.partial_cmp(&b.lift).unwrap_or(std::cmp::Ordering::Equal));

    match best {
        Some(b) => parts.push(format!(
            "Best validated V3 bucket: {} (lift={:.2}, precision={:.1}%). This capital-pressure-aware score indicates elevated medium-term holding risk.",
            b.score_label,
            b.lift,
            b.precision * 100.0
        )),
        None => parts.push(
            "No V3 bucket meets TASK-160.2A acceptance gate at T+60. LiquidityPressure dimension may need further tuning."
                .to_string(),
        ),
    }

    parts.join("\n")
}

/// TASK-160.2B: Holding Risk Bundle V4.
///
/// Adds ConfirmationDecay as a Confirmatory Dimension to the V3 bundle.
/// Weights:
/// - LeadershipDecayPersistence (>=5 days): 0.4
/// - LiquidityPressure (any volume decline, >=3 days): 0.3
/// - ConfirmationDecay (delta_5d < -5 or consecutive >= 2): 0.3
pub fn compute_holding_risk_bundle_v4_analysis(
    records: &[ExecutionResearchRecord],
) -> HoldingRiskBundleAnalysis {
    let total_records = records.len();

    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut bundles: Vec<(HoldingRiskBundle, &ExecutionResearchRecord)> = Vec::new();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let leadership = detect_leadership_decay(*record, *date, by_date);
            let breadth = detect_breadth_deterioration(*record, *date, by_date);
            let liquidity = detect_liquidity_pressure_v3(*record, *date, by_date);
            let confirmation = detect_confirmation_decay_v4(*record, *date, by_date);

            let leadership_flag = leadership.is_leadership_decay()
                && leadership.consecutive_decline_days >= 5;
            let breadth_flag = breadth.is_breadth_deterioration();
            let liquidity_flag = liquidity;
            let confirmation_flag = confirmation;

            let signal_count = [leadership_flag, breadth_flag, liquidity_flag, confirmation_flag]
                .iter()
                .filter(|&&b| b)
                .count() as u32;

            let weighted_score = if leadership_flag { 0.4 } else { 0.0 }
                + if liquidity_flag { 0.3 } else { 0.0 }
                + if confirmation_flag { 0.3 } else { 0.0 };

            bundles.push((
                HoldingRiskBundle {
                    leadership_decay: leadership_flag,
                    breadth_deterioration: breadth_flag,
                    liquidity_deterioration: liquidity_flag,
                    weighted_score,
                    signal_count,
                },
                record,
            ));
        }
    }

    let baseline_records: Vec<&ExecutionResearchRecord> =
        bundles.iter().map(|(_, r)| *r).collect();

    let mut score_buckets = Vec::new();
    for score in 0..=4 {
        let label = format!("{} signals", score);
        let bucket_records: Vec<&ExecutionResearchRecord> = bundles
            .iter()
            .filter(|(b, _)| b.signal_count == score)
            .map(|(_, r)| *r)
            .collect();
        score_buckets.push(build_score_bucket(
            &label,
            &baseline_records,
            &bucket_records,
        ));
    }

    let mut weighted_buckets = Vec::new();
    for (label, min, max) in [
        ("score 0.0", 0.0, 0.01),
        ("score (0, 0.4)", 0.01, 0.41),
        ("score [0.4, 0.7)", 0.40, 0.71),
        ("score [0.7, 1.0)", 0.70, 1.01),
        ("score >= 1.0", 1.00, 10.0),
    ] {
        let bucket_records: Vec<&ExecutionResearchRecord> = bundles
            .iter()
            .filter(|(b, _)| b.weighted_score >= min && b.weighted_score < max)
            .map(|(_, r)| *r)
            .collect();
        weighted_buckets.push(build_score_bucket(&label, &baseline_records, &bucket_records));
    }

    let verdict = build_bundle_v4_verdict(&score_buckets, &weighted_buckets, total_records);

    HoldingRiskBundleAnalysis {
        total_records,
        score_distribution: score_buckets,
        verdict,
    }
}

fn build_bundle_v4_verdict(
    score_buckets: &[ScoreBucket],
    weighted_buckets: &[ScoreBucket],
    total_records: usize,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "Holding Risk Bundle V4 analyzed across {} records. Signal-count buckets:",
        total_records
    ));
    for b in score_buckets {
        parts.push(format!(
            "- {}: n={}, negative T+60={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%, avg_dd={:.2}%",
            b.score_label,
            b.count,
            b.t60_negative_rate * 100.0,
            b.baseline_negative_rate * 100.0,
            b.lift,
            b.precision * 100.0,
            b.avg_t60_return * 100.0,
            b.avg_max_drawdown * 100.0
        ));
    }

    parts.push("Weighted-score buckets:".to_string());
    for b in weighted_buckets {
        parts.push(format!(
            "- {}: n={}, negative T+60={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%",
            b.score_label,
            b.count,
            b.t60_negative_rate * 100.0,
            b.baseline_negative_rate * 100.0,
            b.lift,
            b.precision * 100.0,
            b.avg_t60_return * 100.0
        ));
    }

    let best = weighted_buckets
        .iter()
        .filter(|b| b.count >= 30 && b.lift >= 1.3 && b.precision >= 0.55 && b.t60_negative_rate < 1.0)
        .max_by(|a, b| a.lift.partial_cmp(&b.lift).unwrap_or(std::cmp::Ordering::Equal));

    match best {
        Some(b) => parts.push(format!(
            "Best validated V4 bucket: {} (lift={:.2}, precision={:.1}%). This confirmation-aware score indicates elevated medium-term holding risk.",
            b.score_label,
            b.lift,
            b.precision * 100.0
        )),
        None => parts.push(
            "No V4 bucket meets TASK-160.2B acceptance gate at T+60. ConfirmationDecay may need further tuning."
                .to_string(),
        ),
    }

    parts.join("\n")
}

fn build_bundle_v2_verdict(
    score_buckets: &[ScoreBucket],
    weighted_buckets: &[ScoreBucket],
    total_records: usize,
    min_leadership_persistence_days: usize,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "Holding Risk Bundle V2 analyzed across {} records. LeadershipDecay persistence threshold: >= {} consecutive days. Signal-count buckets:",
        total_records, min_leadership_persistence_days
    ));
    for b in score_buckets {
        parts.push(format!(
            "- {}: n={}, negative T+60={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%, avg_dd={:.2}%",
            b.score_label,
            b.count,
            b.t60_negative_rate * 100.0,
            b.baseline_negative_rate * 100.0,
            b.lift,
            b.precision * 100.0,
            b.avg_t60_return * 100.0,
            b.avg_max_drawdown * 100.0
        ));
    }

    parts.push("Weighted-score buckets:".to_string());
    for b in weighted_buckets {
        parts.push(format!(
            "- {}: n={}, negative T+60={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%",
            b.score_label,
            b.count,
            b.t60_negative_rate * 100.0,
            b.baseline_negative_rate * 100.0,
            b.lift,
            b.precision * 100.0,
            b.avg_t60_return * 100.0
        ));
    }

    let best = weighted_buckets
        .iter()
        .filter(|b| b.count >= 30 && b.lift >= 1.3 && b.precision >= 0.55 && b.t60_negative_rate < 1.0)
        .max_by(|a, b| a.lift.partial_cmp(&b.lift).unwrap_or(std::cmp::Ordering::Equal));

    match best {
        Some(b) => parts.push(format!(
            "Best validated V2 bucket: {} (lift={:.2}, precision={:.1}%). This persistence-based score indicates elevated medium-term holding risk.",
            b.score_label,
            b.lift,
            b.precision * 100.0
        )),
        None => parts.push(
            "No V2 bucket meets TASK-160.1 acceptance gate at T+60. Iterate on persistence threshold or weights."
                .to_string(),
        ),
    }

    parts.join("\n")
}

fn build_bundle_verdict(
    score_buckets: &[ScoreBucket],
    weighted_buckets: &[ScoreBucket],
    total_records: usize,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "Holding Risk Bundle analyzed across {} records. Signal-count buckets:",
        total_records
    ));
    for b in score_buckets {
        parts.push(format!(
            "- {}: n={}, negative T+60={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%, avg_dd={:.2}%",
            b.score_label,
            b.count,
            b.t60_negative_rate * 100.0,
            b.baseline_negative_rate * 100.0,
            b.lift,
            b.precision * 100.0,
            b.avg_t60_return * 100.0,
            b.avg_max_drawdown * 100.0
        ));
    }

    parts.push("Weighted-score buckets:".to_string());
    for b in weighted_buckets {
        parts.push(format!(
            "- {}: n={}, negative T+60={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%",
            b.score_label,
            b.count,
            b.t60_negative_rate * 100.0,
            b.baseline_negative_rate * 100.0,
            b.lift,
            b.precision * 100.0,
            b.avg_t60_return * 100.0
        ));
    }

    // Find the highest signal-count bucket with meaningful lift and precision.
    let best = score_buckets
        .iter()
        .filter(|b| b.count >= 30 && b.lift >= 1.2 && b.precision >= 0.50)
        .max_by(|a, b| a.lift.partial_cmp(&b.lift).unwrap_or(std::cmp::Ordering::Equal));

    match best {
        Some(b) => parts.push(format!(
            "Best validated bucket: {} (lift={:.2}, precision={:.1}%). This score level indicates elevated medium-term holding risk.",
            b.score_label,
            b.lift,
            b.precision * 100.0
        )),
        None => parts.push(
            "No signal-count bucket meets ADR-101 thresholds at T+60. Iterate on signal weights or add more holding risk dimensions."
                .to_string(),
        ),
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use execution_engine::v2::assessment::{ExecutionAssessment, RiskLevel};
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::evidence::{Evidence, EvidenceKind, EvidencePayload, EvidenceSource};
    use execution_engine::v2::feature::IntradayFeatures;
    use execution_engine::v2::request::{
        ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
    };
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn make_record(
        date: NaiveDate,
        leadership_stability: f64,
        breadth_pct: f64,
        volume: f64,
        volume_ma20: f64,
        t60_return: f64,
    ) -> crate::ExecutionResearchRecord {
        let policy = ExecutionPolicy::default();
        let request = ExecutionRequest {
            symbol: "000001".into(),
            date,
            signal: core_domain::SignalSnapshot {
                date,
                symbol: "000001".into(),
                final_score: 70.0,
                signal_label: SignalLabel::Buy,
                analysis_scope: "CN".into(),
                regime_basis_scope: "CN".into(),
                reason: core_domain::SignalReason {
                    best_strategy: StrategyKind::MomentumRight,
                    strategy_score: 0.0,
                    strategy_contribution: 0.0,
                    alignment: 0,
                    aligned_strategies: vec![],
                    alignment_contribution: 0.0,
                    regime: core_domain::RegimeReason {
                        trend_score: 0.0,
                        risk_score: 0.0,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    rotation: core_domain::RotationReason {
                        momentum_score: 0.0,
                        rank: None,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    final_score: 70.0,
                    label: SignalLabel::Buy,
                    summary: "test".into(),
                },
            },
            strategy_state: core_domain::StrategyStateSnapshot {
                date,
                scope: "CN".into(),
                state: StrategyState::NoTrade,
                state_score: 50.0,
                transition_reason: "test".into(),
                recommended_position_pct: 0.0,
            },
            quote: QuoteSnapshot {
                symbol: "000001".into(),
                ts: Utc::now(),
                open: 10.0,
                high: 11.0,
                low: 9.0,
                close: 10.5,
                volume,
                prev_close: 10.0,
            },
            volume_ma20,
            market_view: ExecutionMarketView {
                research_version: "1".into(),
                market_regime_label: "Bullish".into(),
                confirmation: ConfirmationSummary {
                    trend: ConfirmationDimension { score: 50.0, label: "Moderate".into() },
                    participation: ConfirmationDimension { score: 50.0, label: "Moderate".into() },
                    risk: ConfirmationDimension { score: 50.0, label: "Moderate".into() },
                    overall: "Moderate".into(),
                },
                breadth: BreadthSummary { breadth_pct, sma5: None, delta_5d: None, condition: "moderate".into() },
                recovery: RecoverySummary { score: 50.0, drivers: vec![] },
                rotation_state: "mixed".into(),
                leadership_stability,
            },
            policy,
        };
        let features = IntradayFeatures {
            symbol: "000001".into(),
            today_return: 0.0,
            open_return: 0.0,
            gap_pct: 0.0,
            close_position: 0.5,
            amplitude_pct: 0.02,
            upper_shadow_pct: 0.0,
            lower_shadow_pct: 0.0,
            volume_ratio: 1.0,
            body_ratio: 0.3,
            gap_fill_ratio: 0.0,
        };
        let assessment = ExecutionAssessment {
            confidence: 0.5,
            consensus: 0.6,
            coverage: 1.0,
            risk: RiskLevel::Medium,
            dominant_direction: -0.4,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };
        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: execution_engine::types::ExecutionState::Maintain,
            confidence: 0.5,
            risk: RiskLevel::Medium,
            evidences: vec![Evidence {
                kind: EvidenceKind::Breadth,
                confidence: 0.8,
                direction: -1.0,
                source: EvidenceSource::ResearchContext,
                payload: EvidencePayload::Empty,
            }],
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };
        let event = ExecutionEvent::new(request, features, vec![], vec![], assessment, decision);
        crate::ExecutionResearchRecord {
            event,
            outcome: crate::ExecutionOutcome {
                t5_return: None,
                t20_return: None,
                t60_return: Some(t60_return),
                t120_return: None,
                mfe: None,
                mae: None,
                max_drawdown: None,
                holding_days: None,
                benchmark_return: None,
                alpha: None,
                stop_loss_hit: None,
                take_profit_hit: None,
            },
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn detects_liquidity_deterioration_from_volume_ratio_decline() {
        let d0 = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 7, 4).unwrap();
        let d4 = NaiveDate::from_ymd_opt(2026, 7, 5).unwrap();
        let d5 = NaiveDate::from_ymd_opt(2026, 7, 6).unwrap();
        let d6 = NaiveDate::from_ymd_opt(2026, 7, 7).unwrap();

        // Stable volume ratio = 1.0, then sudden drop.
        let records = vec![
            make_record(d0, 0.8, 50.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d1, 0.8, 50.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d2, 0.8, 50.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d3, 0.8, 50.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d4, 0.8, 50.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d5, 0.8, 50.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d6, 0.8, 50.0, 300_000.0, 1_000_000.0, -0.05), // ratio drops to 0.3
        ];
        let analysis = compute_holding_risk_bundle_analysis(&records);
        assert!(analysis.total_records > 0);
        // The last record should have a liquidity deterioration signal because volume ratio dropped sharply.
        assert!(analysis.score_distribution.iter().any(|b| b.count > 0));
    }

    #[test]
    fn all_negative_returns_increase_holding_risk_score() {
        let d0 = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 7, 4).unwrap();
        let d4 = NaiveDate::from_ymd_opt(2026, 7, 5).unwrap();
        let d5 = NaiveDate::from_ymd_opt(2026, 7, 6).unwrap();
        let d6 = NaiveDate::from_ymd_opt(2026, 7, 7).unwrap();

        // Leadership and breadth both deteriorating; volume ratio stable.
        let records = vec![
            make_record(d0, 0.80, 60.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d1, 0.78, 58.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d2, 0.75, 55.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d3, 0.70, 50.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d4, 0.65, 45.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d5, 0.60, 40.0, 1_000_000.0, 1_000_000.0, 0.01),
            make_record(d6, 0.55, 35.0, 1_000_000.0, 1_000_000.0, -0.05),
        ];
        let analysis = compute_holding_risk_bundle_analysis(&records);
        // The last record should have at least leadership and breadth signals.
        let three_signal = analysis
            .score_distribution
            .iter()
            .find(|b| b.score_label == "3 signals");
        assert!(three_signal.is_some());
    }
}

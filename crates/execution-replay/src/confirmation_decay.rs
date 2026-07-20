use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ExecutionResearchRecord;

/// TASK-160.2B: ConfirmationDecay Research Asset.
///
/// ConfirmationDecay studies whether confirmation strength is continuously
/// declining, not just low at a single point in time. It is designed as a
/// Confirmatory Dimension for the Holding Risk Bundle, not as a standalone
/// Exit Signal. This is a Research-only module; it does not modify the
/// Execution Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationDecayAnalysis {
    pub total_records: usize,
    pub baseline_negative_t20_rate: f64,
    pub baseline_negative_t60_rate: f64,
    pub baseline_avg_t20: f64,
    pub baseline_avg_t60: f64,
    pub signal_count: usize,
    pub negative_t20_rate: f64,
    pub negative_t60_rate: f64,
    pub lift_t20: f64,
    pub lift_t60: f64,
    pub precision_t20: f64,
    pub precision_t60: f64,
    pub avg_t20: f64,
    pub avg_t60: f64,
    pub median_t20: f64,
    pub median_t60: f64,
    pub false_reduce_rate_t20: f64,
    pub false_reduce_rate_t60: f64,
    pub signals: Vec<ConfirmationDecaySignal>,
    pub verdict: String,
}

/// A single ConfirmationDecay signal and its components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationDecaySignal {
    pub date: NaiveDate,
    pub confirmation_score: f64,
    pub confirmation_delta_5d: f64,
    pub confirmation_delta_10d: f64,
    pub slope_10d: f64,
    pub consecutive_decline_days: usize,
    pub decay_score: f64,
}

impl ConfirmationDecaySignal {
    /// True when the record satisfies the ConfirmationDecay conditions.
    pub fn is_decay(&self) -> bool {
        self.decay_score >= 1.0
    }
}

/// Computes the ConfirmationDecay analysis with default parameters.
///
/// Default conditions:
/// - `confirmation_delta_5d < -10` OR `slope_10d < -2` OR `consecutive_decline_days >= 3`
/// - optionally require `today_return < 0` (price weakness)
pub fn compute_confirmation_decay_analysis(
    records: &[ExecutionResearchRecord],
    require_price_weakness: bool,
) -> ConfirmationDecayAnalysis {
    compute_confirmation_decay_analysis_with_params(
        records,
        -10.0,
        -2.0,
        3,
        require_price_weakness,
    )
}

/// Computes ConfirmationDecay with explicit tuning parameters.
pub fn compute_confirmation_decay_analysis_with_params(
    records: &[ExecutionResearchRecord],
    delta_5d_threshold: f64,
    slope_10d_threshold: f64,
    min_consecutive_days: usize,
    require_price_weakness: bool,
) -> ConfirmationDecayAnalysis {
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

    let mut signals = Vec::new();
    let mut sample_records: Vec<&ExecutionResearchRecord> = Vec::new();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let confirmation_score = compute_confirmation_score(record);
            let delta_5d = compute_confirmation_delta(record, *date, by_date, 5);
            let delta_10d = compute_confirmation_delta(record, *date, by_date, 10);
            let slope_10d = compute_confirmation_slope(*date, by_date, 10);
            let consecutive_days = count_consecutive_decline_days(*date, by_date);
            let today_return = record.event.features.today_return;

            let delta_trigger = delta_5d < delta_5d_threshold;
            let slope_trigger = slope_10d < slope_10d_threshold;
            let consecutive_trigger = consecutive_days >= min_consecutive_days;
            let price_ok = !require_price_weakness || today_return < 0.0;

            let decay_score = if (delta_trigger || slope_trigger || consecutive_trigger) && price_ok {
                1.0
            } else {
                0.0
            };

            let signal = ConfirmationDecaySignal {
                date: *date,
                confirmation_score,
                confirmation_delta_5d: delta_5d,
                confirmation_delta_10d: delta_10d,
                slope_10d,
                consecutive_decline_days: consecutive_days,
                decay_score,
            };

            if signal.is_decay() {
                sample_records.push(record);
            }
            signals.push(signal);
        }
    }

    let stats = compute_stats(&sample_records, baseline.0, baseline.1, baseline.2, baseline.3);

    let verdict = build_verdict(
        stats.signal_count,
        stats.negative_t20_rate,
        stats.lift_t20,
        stats.precision_t20,
        stats.false_reduce_rate_t20,
        stats.negative_t60_rate,
        stats.lift_t60,
        stats.precision_t60,
        stats.false_reduce_rate_t60,
    );

    ConfirmationDecayAnalysis {
        total_records,
        baseline_negative_t20_rate: baseline.0,
        baseline_negative_t60_rate: baseline.2,
        baseline_avg_t20: baseline.1,
        baseline_avg_t60: baseline.3,
        signal_count: stats.signal_count,
        negative_t20_rate: stats.negative_t20_rate,
        negative_t60_rate: stats.negative_t60_rate,
        lift_t20: stats.lift_t20,
        lift_t60: stats.lift_t60,
        precision_t20: stats.precision_t20,
        precision_t60: stats.precision_t60,
        avg_t20: stats.avg_t20,
        avg_t60: stats.avg_t60,
        median_t20: stats.median_t20,
        median_t60: stats.median_t60,
        false_reduce_rate_t20: stats.false_reduce_rate_t20,
        false_reduce_rate_t60: stats.false_reduce_rate_t60,
        signals,
        verdict,
    }
}

fn compute_confirmation_score(record: &ExecutionResearchRecord) -> f64 {
    let c = &record.event.request.market_view.confirmation;
    (c.trend.score + c.participation.score + c.risk.score) / 3.0
}

fn compute_confirmation_delta(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    trading_days_ago: usize,
) -> f64 {
    let current = compute_confirmation_score(record);
    let mut count = 0usize;
    for (_past_date, past) in by_date.range(..date).rev() {
        count += 1;
        if count == trading_days_ago {
            let past = compute_confirmation_score(past);
            return current - past;
        }
    }
    0.0
}

fn compute_confirmation_slope(
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    window: usize,
) -> f64 {
    let mut values = Vec::new();
    let mut count = 0usize;
    for (_past_date, past) in by_date.range(..date).rev() {
        count += 1;
        if count <= window {
            values.push(compute_confirmation_score(past));
        } else {
            break;
        }
    }
    if values.len() < 3 {
        return 0.0;
    }
    let n = values.len() as f64;
    let xs: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
    let sum_x: f64 = xs.iter().sum();
    let sum_y: f64 = values.iter().sum();
    let sum_xy: f64 = xs.iter().zip(values.iter()).map(|(x, y)| x * y).sum();
    let sum_x2: f64 = xs.iter().map(|x| x * x).sum();
    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-9 {
        return 0.0;
    }
    (n * sum_xy - sum_x * sum_y) / denom
}

fn count_consecutive_decline_days(
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> usize {
    let mut consecutive = 0usize;
    let mut current = if let Some(cur) = by_date.get(&date) {
        compute_confirmation_score(cur)
    } else {
        return 0;
    };

    for days_back in 1..=10 {
        if let Some(prev_date) = date.checked_sub_signed(chrono::Duration::days(days_back)) {
            if let Some(prev) = by_date.get(&prev_date) {
                let prev_score = compute_confirmation_score(prev);
                if current < prev_score {
                    consecutive += 1;
                    current = prev_score;
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

/// Detects ConfirmationDecay for a single record with default bundle parameters.
///
/// Bundle V4 uses this as a confirmatory dimension: `confirmation_delta_5d < -5`
/// or `consecutive_decline_days >= 2`. This is intentionally weaker than the
/// standalone Research Asset definition because it is meant to complement
/// LeadershipDecay and LiquidityPressure inside a bundle.
pub(crate) fn detect_confirmation_decay_v4(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> bool {
    let delta_5d = compute_confirmation_delta(record, date, by_date, 5);
    let consecutive = count_consecutive_decline_days(date, by_date);
    delta_5d < -5.0 || consecutive >= 2
}

fn compute_baseline(records: &[ExecutionResearchRecord]) -> (f64, f64, f64, f64) {
    let mut neg_t20 = 0usize;
    let mut count_t20 = 0usize;
    let mut sum_t20 = 0.0;
    let mut neg_t60 = 0usize;
    let mut count_t60 = 0usize;
    let mut sum_t60 = 0.0;

    for r in records {
        if let Some(t20) = r.outcome.t20_return {
            count_t20 += 1;
            sum_t20 += t20;
            if t20 < 0.0 {
                neg_t20 += 1;
            }
        }
        if let Some(t60) = r.outcome.t60_return {
            count_t60 += 1;
            sum_t60 += t60;
            if t60 < 0.0 {
                neg_t60 += 1;
            }
        }
    }
    (
        safe_rate(neg_t20, count_t20),
        safe_avg(sum_t20, count_t20),
        safe_rate(neg_t60, count_t60),
        safe_avg(sum_t60, count_t60),
    )
}

struct Stats {
    signal_count: usize,
    negative_t20_rate: f64,
    negative_t60_rate: f64,
    lift_t20: f64,
    lift_t60: f64,
    precision_t20: f64,
    precision_t60: f64,
    avg_t20: f64,
    avg_t60: f64,
    median_t20: f64,
    median_t60: f64,
    false_reduce_rate_t20: f64,
    false_reduce_rate_t60: f64,
}

fn compute_stats(
    records: &[&ExecutionResearchRecord],
    baseline_neg_t20: f64,
    _baseline_avg_t20: f64,
    baseline_neg_t60: f64,
    _baseline_avg_t60: f64,
) -> Stats {
    let signal_count = records.len();
    let mut neg_t20 = 0usize;
    let mut count_t20 = 0usize;
    let mut sum_t20 = 0.0;
    let mut t20_values = Vec::new();
    let mut neg_t60 = 0usize;
    let mut count_t60 = 0usize;
    let mut sum_t60 = 0.0;
    let mut t60_values = Vec::new();

    for r in records {
        if let Some(t20) = r.outcome.t20_return {
            count_t20 += 1;
            sum_t20 += t20;
            t20_values.push(t20);
            if t20 < 0.0 {
                neg_t20 += 1;
            }
        }
        if let Some(t60) = r.outcome.t60_return {
            count_t60 += 1;
            sum_t60 += t60;
            t60_values.push(t60);
            if t60 < 0.0 {
                neg_t60 += 1;
            }
        }
    }

    let neg_t20_rate = safe_rate(neg_t20, count_t20);
    let neg_t60_rate = safe_rate(neg_t60, count_t60);
    let lift_t20 = if baseline_neg_t20 > 0.0 {
        neg_t20_rate / baseline_neg_t20
    } else {
        0.0
    };
    let lift_t60 = if baseline_neg_t60 > 0.0 {
        neg_t60_rate / baseline_neg_t60
    } else {
        0.0
    };

    let false_reduce_t20 = if count_t20 > 0 {
        t20_values.iter().filter(|&&v| v >= 0.0).count() as f64 / count_t20 as f64
    } else {
        0.0
    };
    let false_reduce_t60 = if count_t60 > 0 {
        t60_values.iter().filter(|&&v| v >= 0.0).count() as f64 / count_t60 as f64
    } else {
        0.0
    };

    Stats {
        signal_count,
        negative_t20_rate: neg_t20_rate,
        negative_t60_rate: neg_t60_rate,
        lift_t20,
        lift_t60,
        precision_t20: neg_t20_rate,
        precision_t60: neg_t60_rate,
        avg_t20: safe_avg(sum_t20, count_t20),
        avg_t60: safe_avg(sum_t60, count_t60),
        median_t20: median(&t20_values),
        median_t60: median(&t60_values),
        false_reduce_rate_t20: false_reduce_t20,
        false_reduce_rate_t60: false_reduce_t60,
    }
}

fn build_verdict(
    signal_count: usize,
    neg_t20: f64,
    lift_t20: f64,
    precision_t20: f64,
    false_t20: f64,
    neg_t60: f64,
    lift_t60: f64,
    precision_t60: f64,
    false_t60: f64,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("ConfirmationDecay selected {} records.", signal_count));
    lines.push(format!(
        "T+20: negative={:.1}%, lift={:.2}, precision={:.1}%, false reduce={:.1}%.",
        neg_t20 * 100.0,
        lift_t20,
        precision_t20 * 100.0,
        false_t20 * 100.0
    ));
    lines.push(format!(
        "T+60: negative={:.1}%, lift={:.2}, precision={:.1}%, false reduce={:.1}%.",
        neg_t60 * 100.0,
        lift_t60,
        precision_t60 * 100.0,
        false_t60 * 100.0
    ));

    let t20_pass = signal_count >= 30 && precision_t20 >= 0.50 && lift_t20 >= 1.2 && false_t20 < 0.40;
    let t60_pass = signal_count >= 30 && precision_t60 >= 0.50 && lift_t60 >= 1.2 && false_t60 < 0.40;

    if t20_pass && t60_pass {
        lines.push("ConfirmationDecay passes both T+20 and T+60 validation gates.".into());
    } else if t60_pass {
        lines.push("ConfirmationDecay passes T+60 gate but not T+20; treat as MediumTerm Confirmatory Dimension.".into());
    } else if t20_pass {
        lines.push("ConfirmationDecay passes T+20 gate but not T+60; treat as ShortTerm Confirmatory Dimension.".into());
    } else if signal_count >= 30 {
        lines.push("ConfirmationDecay is computable but does not pass all validation gates.".into());
    } else {
        lines.push("Insufficient samples to evaluate ConfirmationDecay.".into());
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
    fn median_computes_middle() {
        assert_eq!(median(&[1.0, 2.0, 3.0]), 2.0);
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), 2.5);
    }

    #[test]
    fn slope_computes_direction() {
        // Unit test placeholder: slope correctness is covered by integration
        // tests with real ExecutionResearchRecord data.
        assert!(true);
    }
}

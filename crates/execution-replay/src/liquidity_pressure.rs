use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::ExecutionResearchRecord;

/// TASK-160.2A: LiquidityPressure Research Asset.
///
/// LiquidityPressure is defined as sustained capital pressure, not a single-day
/// volume spike. It combines turnover decay, price weakness, and breadth failing
/// to recover, persisted for several consecutive days. This is a Research-only
/// module; it does not modify the Execution Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPressureAnalysis {
    pub total_records: usize,
    pub baseline_negative_t60_rate: f64,
    pub baseline_avg_t60: f64,
    pub signal_count: usize,
    pub negative_t60_rate: f64,
    pub lift: f64,
    pub precision: f64,
    pub avg_t60: f64,
    pub median_t60: f64,
    pub false_reduce_rate: f64,
    pub consecutive_pressure_days: usize,
    pub threshold_volume_ratio_delta: f64,
    pub volume_level_threshold: Option<f64>,
    pub signals: Vec<LiquidityPressureSignal>,
    pub verdict: String,
}

/// A single LiquidityPressure signal and its components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPressureSignal {
    pub date: NaiveDate,
    pub volume_ratio: f64,
    pub volume_ratio_delta_5d: f64,
    pub today_return: f64,
    pub breadth_delta_5d: f64,
    pub consecutive_pressure_days: usize,
    pub pressure_score: f64,
}

impl LiquidityPressureSignal {
    /// True when the record satisfies all LiquidityPressure components.
    pub fn is_pressure(&self) -> bool {
        self.pressure_score >= 1.0
    }
}

/// Computes the LiquidityPressure Research Asset analysis.
///
/// Default definition:
/// - `volume_ratio_delta_5d < volume_ratio_delta_threshold` (turnover decay)
/// - `today_return < 0.0` (price weakness) when `require_price_weakness` is true
/// - `breadth_delta_5d < 0.0` (breadth not recovering) when `require_breadth_weakness` is true
/// - persisted for at least `consecutive_pressure_days` consecutive days
pub fn compute_liquidity_pressure_analysis(
    records: &[ExecutionResearchRecord],
    consecutive_pressure_days: usize,
) -> LiquidityPressureAnalysis {
    compute_liquidity_pressure_analysis_with_params(
        records,
        consecutive_pressure_days,
        -0.20,
        true,
        true,
        None,
    )
}

/// Computes LiquidityPressure with explicit tuning parameters.
///
/// If `volume_level_threshold` is `Some(level)`, the signal uses `volume_ratio < level`
/// as the liquidity condition. Otherwise it uses `volume_ratio_delta_5d < volume_ratio_delta_threshold`.
pub fn compute_liquidity_pressure_analysis_with_params(
    records: &[ExecutionResearchRecord],
    consecutive_pressure_days: usize,
    volume_ratio_delta_threshold: f64,
    require_price_weakness: bool,
    require_breadth_weakness: bool,
    volume_level_threshold: Option<f64>,
) -> LiquidityPressureAnalysis {
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
            let volume_ratio = volume_ratio(record);
            let volume_ratio_delta = compute_volume_ratio_delta(record, *date, by_date, 5);
            let today_return = record.event.features.today_return;
            let breadth_delta = compute_breadth_delta(record, *date, by_date, 5);

            let volume_condition_met = match volume_level_threshold {
                Some(level) => volume_ratio < level,
                None => volume_ratio_delta < volume_ratio_delta_threshold,
            };

            let consecutive_days = match volume_level_threshold {
                Some(level) => count_consecutive_volume_level_days(*date, by_date, level),
                None => count_consecutive_pressure_days(
                    *date,
                    by_date,
                    volume_ratio_delta_threshold,
                    if require_price_weakness { 0.0 } else { f64::INFINITY },
                    if require_breadth_weakness { 0.0 } else { f64::INFINITY },
                ),
            };

            let price_ok = !require_price_weakness || today_return < 0.0;
            let breadth_ok = !require_breadth_weakness || breadth_delta < 0.0;
            let pressure_score = if volume_condition_met
                && price_ok
                && breadth_ok
                && consecutive_days >= consecutive_pressure_days
            {
                1.0
            } else {
                0.0
            };

            let signal = LiquidityPressureSignal {
                date: *date,
                volume_ratio,
                volume_ratio_delta_5d: volume_ratio_delta,
                today_return,
                breadth_delta_5d: breadth_delta,
                consecutive_pressure_days: consecutive_days,
                pressure_score,
            };

            if signal.is_pressure() {
                sample_records.push(record);
            }
            signals.push(signal);
        }
    }

    let stats = compute_stats(&sample_records, baseline.0);

    let verdict = build_verdict(
        consecutive_pressure_days,
        stats.signal_count,
        stats.negative_t60_rate,
        stats.lift,
        stats.precision,
        stats.false_reduce_rate,
    );

    LiquidityPressureAnalysis {
        total_records,
        baseline_negative_t60_rate: baseline.0,
        baseline_avg_t60: baseline.1,
        signal_count: stats.signal_count,
        negative_t60_rate: stats.negative_t60_rate,
        lift: stats.lift,
        precision: stats.precision,
        avg_t60: stats.avg_t60,
        median_t60: stats.median_t60,
        false_reduce_rate: stats.false_reduce_rate,
        consecutive_pressure_days,
        threshold_volume_ratio_delta: volume_ratio_delta_threshold,
        volume_level_threshold,
        signals,
        verdict,
    }
}

fn volume_ratio(record: &ExecutionResearchRecord) -> f64 {
    let volume = record.event.request.quote.volume;
    let volume_ma20 = record.event.request.volume_ma20;
    if volume_ma20 > 1e-9 {
        volume / volume_ma20
    } else {
        1.0
    }
}

fn compute_volume_ratio_delta(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    trading_days_ago: usize,
) -> f64 {
    let current_ratio = volume_ratio(record);
    let mut count = 0usize;
    for (_past_date, past) in by_date.range(..date).rev() {
        count += 1;
        if count == trading_days_ago {
            let past_ratio = volume_ratio(past);
            return current_ratio - past_ratio;
        }
    }
    0.0
}

fn compute_breadth_delta(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    trading_days_ago: usize,
) -> f64 {
    let current_breadth = record.event.request.market_view.breadth.breadth_pct;
    let mut count = 0usize;
    for (_past_date, past) in by_date.range(..date).rev() {
        count += 1;
        if count == trading_days_ago {
            let past_breadth = past.event.request.market_view.breadth.breadth_pct;
            return current_breadth - past_breadth;
        }
    }
    0.0
}

fn count_consecutive_pressure_days(
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    volume_threshold: f64,
    return_threshold: f64,
    breadth_threshold: f64,
) -> usize {
    let mut consecutive = 0usize;
    for days_back in 1..=10 {
        if let Some(prev_date) = date.checked_sub_signed(chrono::Duration::days(days_back)) {
            if let Some(prev) = by_date.get(&prev_date) {
                let prev_volume_delta = compute_volume_ratio_delta(prev, prev_date, by_date, 5);
                let prev_return = prev.event.features.today_return;
                let prev_breadth_delta = compute_breadth_delta(prev, prev_date, by_date, 5);
                if prev_volume_delta < volume_threshold
                    && prev_return < return_threshold
                    && prev_breadth_delta < breadth_threshold
                {
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

fn count_consecutive_volume_level_days(
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    level_threshold: f64,
) -> usize {
    let mut consecutive = 0usize;
    for days_back in 1..=10 {
        if let Some(prev_date) = date.checked_sub_signed(chrono::Duration::days(days_back)) {
            if let Some(prev) = by_date.get(&prev_date) {
                let prev_volume_ratio = volume_ratio(prev);
                if prev_volume_ratio < level_threshold {
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

struct Stats {
    signal_count: usize,
    negative_t60_rate: f64,
    lift: f64,
    precision: f64,
    avg_t60: f64,
    median_t60: f64,
    false_reduce_rate: f64,
}

fn compute_stats(records: &[&ExecutionResearchRecord], baseline_rate: f64) -> Stats {
    let signal_count = records.len();
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
    let false_reduce_rate = if count > 0 {
        values.iter().filter(|&&v| v >= 0.0).count() as f64 / count as f64
    } else {
        0.0
    };

    Stats {
        signal_count,
        negative_t60_rate: negative_rate,
        lift,
        precision,
        avg_t60: avg,
        median_t60: median,
        false_reduce_rate,
    }
}

fn build_verdict(
    consecutive_days: usize,
    signal_count: usize,
    negative_rate: f64,
    lift: f64,
    precision: f64,
    false_reduce_rate: f64,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "LiquidityPressure (consecutive >= {} days) selected {} records.",
        consecutive_days, signal_count
    ));
    lines.push(format!(
        "T+60: negative rate={:.1}%, lift={:.2}, precision={:.1}%, false reduce rate={:.1}%.",
        negative_rate * 100.0,
        lift,
        precision * 100.0,
        false_reduce_rate * 100.0
    ));
    if signal_count >= 30 && precision >= 0.50 && lift >= 1.2 && false_reduce_rate < 0.40 {
        lines.push("This LiquidityPressure profile meets the Research Asset acceptance gate.".into());
    } else if signal_count >= 30 {
        lines.push(
            "This LiquidityPressure profile is computable but does not meet all Research Asset thresholds."
                .into(),
        );
    } else {
        lines.push("Insufficient samples to evaluate LiquidityPressure.".into());
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
    fn baseline_on_empty_records() {
        let (rate, avg) = compute_baseline(&[]);
        assert_eq!(rate, 0.0);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn pressure_signal_matches_all_conditions() {
        let signal = LiquidityPressureSignal {
            date: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            volume_ratio: 0.8,
            volume_ratio_delta_5d: -0.25,
            today_return: -0.01,
            breadth_delta_5d: -2.0,
            consecutive_pressure_days: 3,
            pressure_score: 1.0,
        };
        assert!(signal.is_pressure());
    }
}

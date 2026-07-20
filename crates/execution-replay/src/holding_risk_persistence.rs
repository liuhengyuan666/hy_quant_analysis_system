use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{
    transition_analysis::{detect_leadership_decay, LeadershipDecaySignal},
    ExecutionResearchRecord,
};

/// TASK-160.1: Holding Risk Persistence analysis.
///
/// Tests whether sustained LeadershipDecay (consecutive decline days + velocity)
/// is a stronger medium-term Holding Risk signal than a single-day snapshot.
/// This is a Research-only module; it does not modify the Execution Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoldingRiskPersistenceAnalysis {
    pub total_records: usize,
    pub baseline_negative_t60_rate: f64,
    pub baseline_avg_t60: f64,
    pub experiments: Vec<PersistenceExperiment>,
    pub velocity_experiment: Option<PersistenceExperiment>,
    pub verdict: String,
}

/// A single persistence experiment (e.g. "LeadershipDecay for N consecutive days").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceExperiment {
    pub signal_name: String,
    pub horizon: String,
    pub min_consecutive_days: usize,
    pub velocity_window: Option<usize>,
    pub sample_count: usize,
    pub negative_rate: f64,
    pub baseline_negative_rate: f64,
    pub lift: f64,
    pub precision: f64,
    pub avg_t60: f64,
    pub median_t60: f64,
    pub false_reduce_rate: f64,
}

/// Per-record persistence signal for LeadershipDecay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadershipDecayPersistenceSignal {
    pub date: NaiveDate,
    pub leadership_now: f64,
    pub decay_score: f64,
    pub consecutive_decline_days: usize,
    pub velocity_vs_5d: f64,
    pub velocity_vs_10d: f64,
    pub persistence_score: f64,
}

impl LeadershipDecayPersistenceSignal {
    /// True when the record shows at least one day of LeadershipDecay.
    pub fn is_decay(&self) -> bool {
        self.decay_score >= 1.0
    }

    /// True when LeadershipDecay has persisted for at least `n` consecutive days.
    pub fn is_persistent(&self, n: usize) -> bool {
        self.is_decay() && self.consecutive_decline_days >= n
    }
}

/// Computes the LeadershipDecay persistence analysis.
///
/// Runs experiments for consecutive decline thresholds [1, 2, 3, 5, 10] and a
/// velocity experiment (current leadership vs trailing 5-day average).
pub fn compute_holding_risk_persistence_analysis(
    records: &[ExecutionResearchRecord],
) -> HoldingRiskPersistenceAnalysis {
    let total_records = records.len();

    let baseline = compute_baseline(records);

    let signals = build_leadership_persistence_signals(records);

    let thresholds = vec![1usize, 2, 3, 5, 10];
    let mut experiments = Vec::new();
    for n in thresholds {
        experiments.push(run_consecutive_experiment(
            &signals,
            records,
            n,
            baseline.0,
            baseline.1,
        ));
    }

    let velocity_experiment = Some(run_velocity_experiment(
        &signals,
        records,
        5,
        baseline.0,
        baseline.1,
    ));

    let verdict = build_verdict(&experiments, velocity_experiment.as_ref(), baseline.0);

    HoldingRiskPersistenceAnalysis {
        total_records,
        baseline_negative_t60_rate: baseline.0,
        baseline_avg_t60: baseline.1,
        experiments,
        velocity_experiment,
        verdict,
    }
}

fn build_leadership_persistence_signals(
    records: &[ExecutionResearchRecord],
) -> Vec<LeadershipDecayPersistenceSignal> {
    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut signals = Vec::new();
    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let decay = detect_leadership_decay(record, *date, by_date);
            let velocity_5d = compute_leadership_velocity(record, *date, by_date, 5);
            let velocity_10d = compute_leadership_velocity(record, *date, by_date, 10);
            let persistence_score = persistence_score(&decay, velocity_5d);

            signals.push(LeadershipDecayPersistenceSignal {
                date: *date,
                leadership_now: decay.leadership_now,
                decay_score: decay.decay_score,
                consecutive_decline_days: decay.consecutive_decline_days,
                velocity_vs_5d: velocity_5d,
                velocity_vs_10d: velocity_10d,
                persistence_score,
            });
        }
    }
    signals
}

fn compute_leadership_velocity(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    window: usize,
) -> f64 {
    let current = record.event.request.market_view.leadership_stability;
    let mut values = Vec::new();
    let mut count = 0usize;
    for (_past_date, past) in by_date.range(..date).rev() {
        count += 1;
        if count <= window {
            values.push(past.event.request.market_view.leadership_stability);
        } else {
            break;
        }
    }
    if values.is_empty() {
        return 0.0;
    }
    let avg = values.iter().sum::<f64>() / values.len() as f64;
    current - avg
}

fn persistence_score(decay: &LeadershipDecaySignal, velocity_vs_5d: f64) -> f64 {
    let mut score = decay.decay_score;
    if decay.consecutive_decline_days >= 2 {
        score += 0.5;
    }
    if decay.consecutive_decline_days >= 3 {
        score += 0.5;
    }
    if velocity_vs_5d < -0.10 {
        score += 0.5;
    }
    if velocity_vs_5d < -0.20 {
        score += 0.5;
    }
    score
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

fn run_consecutive_experiment(
    signals: &[LeadershipDecayPersistenceSignal],
    records: &[ExecutionResearchRecord],
    min_consecutive_days: usize,
    baseline_rate: f64,
    _baseline_avg: f64,
) -> PersistenceExperiment {
    let mut samples = Vec::new();
    for signal in signals {
        if signal.is_persistent(min_consecutive_days) {
            if let Some(record) = find_record(records, signal.date) {
                samples.push(record);
            }
        }
    }
    compute_experiment_stats(
        samples,
        baseline_rate,
        format!("LeadershipDecay >= {} consecutive days", min_consecutive_days),
        "T+60".into(),
        min_consecutive_days,
        None,
    )
}

fn run_velocity_experiment(
    signals: &[LeadershipDecayPersistenceSignal],
    records: &[ExecutionResearchRecord],
    window: usize,
    baseline_rate: f64,
    _baseline_avg: f64,
) -> PersistenceExperiment {
    let mut samples = Vec::new();
    for signal in signals {
        if signal.is_decay() && signal.velocity_vs_5d < -0.10 {
            if let Some(record) = find_record(records, signal.date) {
                samples.push(record);
            }
        }
    }
    compute_experiment_stats(
        samples,
        baseline_rate,
        format!("LeadershipDecay with velocity < -0.10 ({}-day window)", window),
        "T+60".into(),
        1,
        Some(window),
    )
}

fn compute_experiment_stats(
    samples: Vec<&ExecutionResearchRecord>,
    baseline_rate: f64,
    signal_name: String,
    horizon: String,
    min_consecutive_days: usize,
    velocity_window: Option<usize>,
) -> PersistenceExperiment {
    let sample_count = samples.len();
    let mut negatives = 0usize;
    let mut count = 0usize;
    let mut sum = 0.0;
    let mut t60_values = Vec::new();
    for r in &samples {
        if let Some(t60) = r.outcome.t60_return {
            count += 1;
            sum += t60;
            t60_values.push(t60);
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
    let avg_t60 = safe_avg(sum, count);
    let median_t60 = median(&t60_values);
    let false_reduce_rate = if count > 0 {
        let false_reduces = t60_values.iter().filter(|&&v| v >= 0.0).count();
        false_reduces as f64 / count as f64
    } else {
        0.0
    };

    PersistenceExperiment {
        signal_name,
        horizon,
        min_consecutive_days,
        velocity_window,
        sample_count,
        negative_rate,
        baseline_negative_rate: baseline_rate,
        lift,
        precision,
        avg_t60,
        median_t60,
        false_reduce_rate,
    }
}

fn find_record(
    records: &[ExecutionResearchRecord],
    date: NaiveDate,
) -> Option<&ExecutionResearchRecord> {
    records.iter().find(|r| r.event.date() == date)
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

fn build_verdict(
    experiments: &[PersistenceExperiment],
    velocity_experiment: Option<&PersistenceExperiment>,
    baseline_rate: f64,
) -> String {
    let best = experiments
        .iter()
        .chain(velocity_experiment.iter().copied())
        .filter(|e| e.sample_count >= 30)
        .max_by(|a, b| {
            a.lift
                .partial_cmp(&b.lift)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    let mut lines = Vec::new();
    lines.push(format!(
        "Baseline T+60 negative rate: {:.1}%. Tested {} consecutive-day thresholds and one velocity experiment.",
        baseline_rate * 100.0,
        experiments.len()
    ));

    if let Some(e) = best {
        lines.push(format!(
            "Best experiment: {} — n={}, negative rate={:.1}%, lift={:.2}, precision={:.1}%, false reduce rate={:.1}%.",
            e.signal_name,
            e.sample_count,
            e.negative_rate * 100.0,
            e.lift,
            e.precision * 100.0,
            e.false_reduce_rate * 100.0
        ));
        if e.lift >= 1.3 && e.precision >= 0.55 && e.false_reduce_rate < 0.40 {
            lines.push("This persistence profile meets the TASK-160.1 acceptance gate.".into());
        } else {
            lines.push("This persistence profile does not yet meet the TASK-160.1 acceptance gate.".into());
        }
    } else {
        lines.push("No experiment had sufficient samples to evaluate.".into());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_computes_rate() {
        let records = vec![];
        let (rate, avg) = compute_baseline(&records);
        assert_eq!(rate, 0.0);
        assert_eq!(avg, 0.0);
    }

    #[test]
    fn persistence_signal_detects_persistent_decay() {
        let signal = LeadershipDecayPersistenceSignal {
            date: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            leadership_now: 0.4,
            decay_score: 1.0,
            consecutive_decline_days: 3,
            velocity_vs_5d: -0.15,
            velocity_vs_10d: -0.20,
            persistence_score: 2.0,
        };
        assert!(signal.is_persistent(1));
        assert!(signal.is_persistent(3));
        assert!(!signal.is_persistent(5));
    }

    #[test]
    fn median_computes_middle_value() {
        assert_eq!(median(&[1.0, 2.0, 3.0]), 2.0);
        assert_eq!(median(&[1.0, 2.0, 3.0, 4.0]), 2.5);
    }
}

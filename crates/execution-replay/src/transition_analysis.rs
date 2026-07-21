use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{ExecutionOutcome, ExecutionResearchRecord};

/// 2B-2: Transition Evidence Modeling.
///
/// Research-only analysis of "change / deterioration" signals derived from the
/// existing `ExecutionResearchRecord` stream. It does not modify the Execution
/// Pipeline (Observation / Evidence / Assessment / Decision / Policy).  Validated
/// signals may later be promoted to `ObservationKind` or `EvidenceKind` via a
/// separate ADR process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionAnalysis {
    pub candidate: TransitionCandidate,
    pub total_records: usize,
    pub samples: usize,
    pub baseline_negative_t20_rate: f64,
    pub baseline_negative_t60_rate: f64,
    pub negative_t20_rate: f64,
    pub negative_t60_rate: f64,
    pub precision_t20: f64,
    pub precision_t60: f64,
    pub lift_t20: f64,
    pub lift_t60: f64,
    pub avg_t20: f64,
    pub avg_t60: f64,
    pub breakdown: TransitionBreakdown,
    pub verdict: String,
}

/// Transition evidence candidates under investigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransitionCandidate {
    RecoveryFailure,
    BreadthDeterioration,
    LeadershipDecay,
}

impl std::str::FromStr for TransitionCandidate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "recovery_failure" | "recovery-failure" => Ok(TransitionCandidate::RecoveryFailure),
            "breadth_deterioration" | "breadth-deterioration" => Ok(TransitionCandidate::BreadthDeterioration),
            "leadership_decay" | "leadership-decay" => Ok(TransitionCandidate::LeadershipDecay),
            _ => Err(format!("unknown transition candidate: {}", s)),
        }
    }
}

impl std::fmt::Display for TransitionCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionCandidate::RecoveryFailure => write!(f, "RecoveryFailure"),
            TransitionCandidate::BreadthDeterioration => write!(f, "BreadthDeterioration"),
            TransitionCandidate::LeadershipDecay => write!(f, "LeadershipDecay"),
        }
    }
}

/// Candidate-specific breakdown of samples.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TransitionBreakdown {
    RecoveryFailure(FailureBreakdown),
    BreadthDeterioration(BreadthDeteriorationBreakdown),
    LeadershipDecay(LeadershipDecayBreakdown),
}

impl Default for TransitionBreakdown {
    fn default() -> Self {
        TransitionBreakdown::RecoveryFailure(FailureBreakdown::default())
    }
}

/// A single RecoveryFailure signal and its components.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecoveryFailureSignal {
    pub price_recovery_failed: bool,
    pub breadth_recovery_failed: bool,
    pub leadership_recovery_failed: bool,
    pub failure_score: f64,
}

impl RecoveryFailureSignal {
    pub fn is_recovery_failure(&self) -> bool {
        self.failure_score >= 0.5
    }
}

/// Breakdown of RecoveryFailure samples by component combination.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FailureBreakdown {
    pub price_only: usize,
    pub breadth_only: usize,
    pub leadership_only: usize,
    pub price_breadth: usize,
    pub price_leadership: usize,
    pub breadth_leadership: usize,
    pub full_failure: usize,
}

/// A single BreadthDeterioration signal and its components.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BreadthDeteriorationSignal {
    pub breadth_now: f64,
    pub breadth_delta_5d: f64,
    pub breadth_delta_10d: f64,
    pub consecutive_decline_days: usize,
    pub deterioration_score: f64,
    pub triggered_by: String,
}

impl BreadthDeteriorationSignal {
    pub fn is_breadth_deterioration(&self) -> bool {
        self.deterioration_score >= 1.0
    }
}

/// Breakdown of BreadthDeterioration samples by the source of deterioration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BreadthDeteriorationBreakdown {
    pub delta_5d_only: usize,
    pub delta_10d_only: usize,
    pub both: usize,
}

/// Computes a Transition Analysis for the requested candidate.
///
/// This is a read-only research tool. It does not modify the Execution Pipeline.
pub fn compute_transition_analysis(
    records: &[ExecutionResearchRecord],
    candidate: TransitionCandidate,
) -> TransitionAnalysis {
    match candidate {
        TransitionCandidate::RecoveryFailure => compute_recovery_failure_analysis(records),
        TransitionCandidate::BreadthDeterioration => compute_breadth_deterioration_analysis(records),
        TransitionCandidate::LeadershipDecay => compute_leadership_decay_analysis(records),
    }
}

fn _empty_analysis(
    records: &[ExecutionResearchRecord],
    candidate: TransitionCandidate,
    verdict: &str,
) -> TransitionAnalysis {
    let total_records = records.len();
    let (base_neg_t20, base_neg_t60, base_count_t20, base_count_t60, _, _) = aggregate_outcomes(&records.iter().collect::<Vec<_>>());
    TransitionAnalysis {
        candidate,
        total_records,
        samples: 0,
        baseline_negative_t20_rate: safe_rate(base_neg_t20, base_count_t20),
        baseline_negative_t60_rate: safe_rate(base_neg_t60, base_count_t60),
        negative_t20_rate: 0.0,
        negative_t60_rate: 0.0,
        precision_t20: 0.0,
        precision_t60: 0.0,
        lift_t20: 0.0,
        lift_t60: 0.0,
        avg_t20: 0.0,
        avg_t60: 0.0,
        breakdown: TransitionBreakdown::RecoveryFailure(FailureBreakdown::default()),
        verdict: verdict.to_string(),
    }
}

fn compute_recovery_failure_analysis(records: &[ExecutionResearchRecord]) -> TransitionAnalysis {
    let total_records = records.len();

    let baseline: Vec<&ExecutionResearchRecord> = records.iter().collect();
    let (base_neg_t20, base_neg_t60, base_count_t20, base_count_t60, base_sum_t20, base_sum_t60) = aggregate_outcomes(&baseline);
    let baseline_negative_t20_rate = safe_rate(base_neg_t20, base_count_t20);
    let baseline_negative_t60_rate = safe_rate(base_neg_t60, base_count_t60);
    let baseline_avg_t20 = safe_avg(base_sum_t20, base_count_t20);
    let _baseline_avg_t60 = safe_avg(base_sum_t60, base_count_t60);

    // Group by symbol and sort by date so we can look back in time.
    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> = BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut sample_records: Vec<&ExecutionResearchRecord> = Vec::new();
    let mut breakdown = FailureBreakdown::default();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let signal = detect_recovery_failure(*record, *date, by_date);
            if signal.is_recovery_failure() {
                sample_records.push(record);
                update_breakdown(&mut breakdown, &signal);
            }
        }
    }

    let (sample_neg_t20, sample_neg_t60, sample_count_t20, sample_count_t60, sample_sum_t20, sample_sum_t60) =
        aggregate_outcomes(&sample_records);
    let samples = sample_records.len();

    let negative_t20_rate = safe_rate(sample_neg_t20, sample_count_t20);
    let negative_t60_rate = safe_rate(sample_neg_t60, sample_count_t60);
    let avg_t20 = safe_avg(sample_sum_t20, sample_count_t20);
    let avg_t60 = safe_avg(sample_sum_t60, sample_count_t60);

    let lift_t20 = if baseline_negative_t20_rate > 0.0 {
        negative_t20_rate / baseline_negative_t20_rate
    } else {
        0.0
    };
    let lift_t60 = if baseline_negative_t60_rate > 0.0 {
        negative_t60_rate / baseline_negative_t60_rate
    } else {
        0.0
    };

    let verdict = build_recovery_failure_verdict(
        samples,
        baseline_negative_t20_rate,
        negative_t20_rate,
        lift_t20,
        avg_t20,
        baseline_avg_t20,
    );

    TransitionAnalysis {
        candidate: TransitionCandidate::RecoveryFailure,
        total_records,
        samples,
        baseline_negative_t20_rate,
        baseline_negative_t60_rate,
        negative_t20_rate,
        negative_t60_rate,
        precision_t20: negative_t20_rate,
        precision_t60: negative_t60_rate,
        lift_t20,
        lift_t60,
        avg_t20,
        avg_t60,
        breakdown: TransitionBreakdown::RecoveryFailure(breakdown),
        verdict,
    }
}

/// Detects whether a record at `date` represents a failed recovery.
///
/// 3-phase model:
/// 1. Initial pressure: a recent prior day (within 5 calendar days) shows a
///    significant price drop or a close near the daily low.
/// 2. Recovery attempt: current day shows a real positive bounce (not just "not falling").
/// 3. Recovery failure: the bounce is present but price, breadth, and leadership
///    have not recovered meaningfully.
fn detect_recovery_failure(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> RecoveryFailureSignal {
    const MAX_LOOKBACK_DAYS: i64 = 5;
    const PRESSURE_RETURN_THRESHOLD: f64 = -0.015;
    const PRESSURE_CLOSE_POSITION_THRESHOLD: f64 = 0.25;
    const BOUNCE_RETURN_THRESHOLD: f64 = 0.005;

    // Phase 1: find a recent pressure day. We require a significant drop or a close
    // near the daily low, which sets up the possibility of a recovery attempt.
    let mut pressure_record: Option<&ExecutionResearchRecord> = None;
    for days_back in 1..=MAX_LOOKBACK_DAYS {
        if let Some(lookback_date) = date.checked_sub_signed(chrono::Duration::days(days_back)) {
            if let Some(prior) = by_date.get(&lookback_date) {
                if prior.event.features.today_return < PRESSURE_RETURN_THRESHOLD
                    || prior.event.features.close_position < PRESSURE_CLOSE_POSITION_THRESHOLD
                {
                    pressure_record = Some(prior);
                    break;
                }
            }
        }
    }

    let pressure = match pressure_record {
        Some(p) => p,
        None => return RecoveryFailureSignal::default(),
    };

    // Phase 2: recovery attempt. Today must show a real bounce, not just "not falling".
    let current_return = record.event.features.today_return;
    let current_close = record.event.request.quote.close;
    let pressure_close = pressure.event.request.quote.close;
    let recovery_attempt = current_return >= BOUNCE_RETURN_THRESHOLD && current_close >= pressure_close * 0.98;
    if !recovery_attempt {
        return RecoveryFailureSignal::default();
    }

    // Phase 3: recovery failure components. The bounce is present but underlying
    // conditions did not recover.
    let price_recovery_failed = current_close < pressure_close * 1.02;

    let current_breadth = record.event.request.market_view.breadth.breadth_pct;
    let pressure_breadth = pressure.event.request.market_view.breadth.breadth_pct;
    let current_breadth_delta = record
        .event
        .request
        .market_view
        .breadth
        .delta_5d
        .unwrap_or(0.0);
    let breadth_recovery_failed =
        current_breadth < pressure_breadth + 3.0 || current_breadth < 45.0 || current_breadth_delta < -2.0;

    let current_leadership = record.event.request.market_view.leadership_stability;
    let pressure_leadership = pressure.event.request.market_view.leadership_stability;
    let leadership_recovery_failed =
        current_leadership < pressure_leadership * 1.02 || current_leadership < 0.55;

    let failure_score = 0.4 * (price_recovery_failed as i32 as f64)
        + 0.4 * (breadth_recovery_failed as i32 as f64)
        + 0.2 * (leadership_recovery_failed as i32 as f64);

    RecoveryFailureSignal {
        price_recovery_failed,
        breadth_recovery_failed,
        leadership_recovery_failed,
        failure_score,
    }
}

fn update_breakdown(breakdown: &mut FailureBreakdown, signal: &RecoveryFailureSignal) {
    match (
        signal.price_recovery_failed,
        signal.breadth_recovery_failed,
        signal.leadership_recovery_failed,
    ) {
        (true, true, true) => breakdown.full_failure += 1,
        (true, true, false) => breakdown.price_breadth += 1,
        (true, false, true) => breakdown.price_leadership += 1,
        (false, true, true) => breakdown.breadth_leadership += 1,
        (true, false, false) => breakdown.price_only += 1,
        (false, true, false) => breakdown.breadth_only += 1,
        (false, false, true) => breakdown.leadership_only += 1,
        (false, false, false) => {} // Should not reach here if failure_score >= 0.5.
    }
}

fn aggregate_outcomes(records: &[&ExecutionResearchRecord]) -> (usize, usize, usize, usize, f64, f64) {
    let mut neg_t20 = 0usize;
    let mut neg_t60 = 0usize;
    let mut count_t20 = 0usize;
    let mut count_t60 = 0usize;
    let mut sum_t20 = 0.0;
    let mut sum_t60 = 0.0;

    for r in records {
        if let Some(t20) = r.outcome.t20_return {
            sum_t20 += t20;
            count_t20 += 1;
            if t20 < 0.0 {
                neg_t20 += 1;
            }
        }
        if let Some(t60) = r.outcome.t60_return {
            sum_t60 += t60;
            count_t60 += 1;
            if t60 < 0.0 {
                neg_t60 += 1;
            }
        }
    }

    (neg_t20, neg_t60, count_t20, count_t60, sum_t20, sum_t60)
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

fn build_recovery_failure_verdict(
    samples: usize,
    baseline_negative_t20_rate: f64,
    negative_t20_rate: f64,
    lift_t20: f64,
    avg_t20: f64,
    baseline_avg_t20: f64,
) -> String {
    if samples == 0 {
        return "No RecoveryFailure samples detected.".to_string();
    }

    let mut parts = Vec::new();
    parts.push(format!(
        "RecoveryFailure samples: {}. Baseline negative T+20: {:.1}%. Signal negative T+20: {:.1}%. Lift: {:.2}.",
        samples,
        baseline_negative_t20_rate * 100.0,
        negative_t20_rate * 100.0,
        lift_t20
    ));

    if samples < 30 {
        parts.push(format!(
            "Sample size below ADR-101 minimum ({} < 30). Result is exploratory, not validated.",
            samples
        ));
    } else if negative_t20_rate >= 0.50 && lift_t20 >= 1.2 {
        parts.push("Meets ADR-101 thresholds (n >= 30, precision >= 50%, lift >= 1.2). Candidate is validated for promotion discussion.".to_string());
    } else if lift_t20 >= 1.2 {
        parts.push("Lift >= 1.2 but precision < 50%. Signal has directional value but insufficient accuracy for exit decisions.".to_string());
    } else if negative_t20_rate >= 0.50 {
        parts.push("Precision >= 50% but lift < 1.2. Signal is not better than naive bearish baseline.".to_string());
    } else {
        parts.push("Does not meet ADR-101 thresholds. Reject or iterate on detection logic.".to_string());
    }

    parts.push(format!(
        "Average T+20: RecoveryFailure={:.2}%, baseline={:.2}%.",
        avg_t20 * 100.0,
        baseline_avg_t20 * 100.0
    ));

    parts.join(" ")
}

fn compute_breadth_deterioration_analysis(records: &[ExecutionResearchRecord]) -> TransitionAnalysis {
    let total_records = records.len();

    let baseline: Vec<&ExecutionResearchRecord> = records.iter().collect();
    let (base_neg_t20, base_neg_t60, base_count_t20, base_count_t60, base_sum_t20, base_sum_t60) = aggregate_outcomes(&baseline);
    let baseline_negative_t20_rate = safe_rate(base_neg_t20, base_count_t20);
    let baseline_negative_t60_rate = safe_rate(base_neg_t60, base_count_t60);
    let baseline_avg_t20 = safe_avg(base_sum_t20, base_count_t20);
    let _baseline_avg_t60 = safe_avg(base_sum_t60, base_count_t60);

    // Group by symbol and sort by date so we can look back for historical breadth.
    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> = BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    // Diagnostic: compute distribution of breadth deltas before any thresholding.
    let delta_distribution = compute_breadth_delta_distribution(&by_symbol);

    let mut sample_records: Vec<&ExecutionResearchRecord> = Vec::new();
    let mut breakdown = BreadthDeteriorationBreakdown::default();
    let mut delta5_records: Vec<&ExecutionResearchRecord> = Vec::new();
    let mut delta10_records: Vec<&ExecutionResearchRecord> = Vec::new();
    let mut both_records: Vec<&ExecutionResearchRecord> = Vec::new();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let signal = detect_breadth_deterioration(*record, *date, by_date);
            if signal.is_breadth_deterioration() {
                sample_records.push(record);
                match signal.triggered_by.as_str() {
                    "delta_5d" => {
                        breakdown.delta_5d_only += 1;
                        delta5_records.push(record);
                    }
                    "delta_10d" => {
                        breakdown.delta_10d_only += 1;
                        delta10_records.push(record);
                    }
                    "both" => {
                        breakdown.both += 1;
                        both_records.push(record);
                    }
                    _ => {}
                }
            }
        }
    }

    let (sample_neg_t20, sample_neg_t60, sample_count_t20, sample_count_t60, sample_sum_t20, sample_sum_t60) =
        aggregate_outcomes(&sample_records);
    let samples = sample_records.len();

    let negative_t20_rate = safe_rate(sample_neg_t20, sample_count_t20);
    let negative_t60_rate = safe_rate(sample_neg_t60, sample_count_t60);
    let avg_t20 = safe_avg(sample_sum_t20, sample_count_t20);
    let avg_t60 = safe_avg(sample_sum_t60, sample_count_t60);

    let lift_t20 = if baseline_negative_t20_rate > 0.0 {
        negative_t20_rate / baseline_negative_t20_rate
    } else {
        0.0
    };
    let lift_t60 = if baseline_negative_t60_rate > 0.0 {
        negative_t60_rate / baseline_negative_t60_rate
    } else {
        0.0
    };

    let sub_stats = compute_breadth_sub_stats(
        &delta5_records,
        &delta10_records,
        &both_records,
    );

    let verdict = build_breadth_deterioration_verdict(
        samples,
        baseline_negative_t20_rate,
        negative_t20_rate,
        lift_t20,
        avg_t20,
        baseline_avg_t20,
        &sub_stats,
        &delta_distribution,
    );

    TransitionAnalysis {
        candidate: TransitionCandidate::BreadthDeterioration,
        total_records,
        samples,
        baseline_negative_t20_rate,
        baseline_negative_t60_rate,
        negative_t20_rate,
        negative_t60_rate,
        precision_t20: negative_t20_rate,
        precision_t60: negative_t60_rate,
        lift_t20,
        lift_t60,
        avg_t20,
        avg_t60,
        breakdown: TransitionBreakdown::BreadthDeterioration(breakdown),
        verdict,
    }
}

#[derive(Debug, Clone, Default)]
struct BreadthSubStats {
    delta5_negative_t20: f64,
    delta10_negative_t20: f64,
    both_negative_t20: f64,
    delta5_count: usize,
    delta10_count: usize,
    both_count: usize,
}

fn compute_breadth_sub_stats(
    delta5: &[&ExecutionResearchRecord],
    delta10: &[&ExecutionResearchRecord],
    both: &[&ExecutionResearchRecord],
) -> BreadthSubStats {
    let (n5, _, c5, _, _, _) = aggregate_outcomes(delta5);
    let (n10, _, c10, _, _, _) = aggregate_outcomes(delta10);
    let (nb, _, cb, _, _, _) = aggregate_outcomes(both);

    BreadthSubStats {
        delta5_negative_t20: safe_rate(n5, c5),
        delta10_negative_t20: safe_rate(n10, c10),
        both_negative_t20: safe_rate(nb, cb),
        delta5_count: delta5.len(),
        delta10_count: delta10.len(),
        both_count: both.len(),
    }
}

/// Detects whether a record at `date` represents a breadth deterioration transition.
///
/// Uses both the pre-computed `delta_5d` from the record and a self-computed
/// `delta_10d` from the historical record stream. This keeps the analysis fully
/// research-only and independent of any future ObservationEngine change.
pub(crate) fn detect_breadth_deterioration(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> BreadthDeteriorationSignal {
    const DELTA_5D_THRESHOLD: f64 = -5.0; // percentage points
    const DELTA_10D_THRESHOLD: f64 = -10.0; // percentage points

    let breadth_now = record.event.request.market_view.breadth.breadth_pct;

    // Prefer the pre-computed delta_5d; fall back to computing it from the record stream.
    let delta_5d = record
        .event
        .request
        .market_view
        .breadth
        .delta_5d
        .unwrap_or_else(|| compute_breadth_delta(record, date, by_date, 5));

    let delta_10d = compute_breadth_delta(record, date, by_date, 10);

    let delta_5d_trigger = delta_5d < DELTA_5D_THRESHOLD;
    let delta_10d_trigger = delta_10d < DELTA_10D_THRESHOLD;

    let triggered_by = if delta_5d_trigger && delta_10d_trigger {
        "both"
    } else if delta_5d_trigger {
        "delta_5d"
    } else if delta_10d_trigger {
        "delta_10d"
    } else {
        ""
    };

    let deterioration_score = if delta_5d_trigger || delta_10d_trigger {
        1.0
    } else {
        0.0
    };

    let consecutive_decline_days = count_breadth_decline_days(date, by_date);

    BreadthDeteriorationSignal {
        breadth_now,
        breadth_delta_5d: delta_5d,
        breadth_delta_10d: delta_10d,
        consecutive_decline_days,
        deterioration_score,
        triggered_by: triggered_by.to_string(),
    }
}

fn compute_breadth_delta(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    trading_days_ago: usize,
) -> f64 {
    let current = record.event.request.market_view.breadth.breadth_pct;

    // Find the Nth prior trading day (N = trading_days_ago). Calendar days are
    // not reliable because trading is sparse around weekends/holidays.
    let mut count = 0usize;
    for (_past_date, past) in by_date.range(..date).rev() {
        count += 1;
        if count == trading_days_ago {
            let past_breadth = past.event.request.market_view.breadth.breadth_pct;
            return current - past_breadth;
        }
    }
    0.0
}

fn count_breadth_decline_days(
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> usize {
    let mut consecutive = 0usize;
    for days_back in 1..=10 {
        if let Some(prev_date) = date.checked_sub_signed(chrono::Duration::days(days_back)) {
            if let Some(prev) = by_date.get(&prev_date) {
                if let Some(current) = by_date.get(&date) {
                    let current_breadth = current.event.request.market_view.breadth.breadth_pct;
                    let prev_breadth = prev.event.request.market_view.breadth.breadth_pct;
                    if current_breadth < prev_breadth {
                        consecutive += 1;
                    } else {
                        break;
                    }
                }
            }
        }
    }
    consecutive
}

#[derive(Debug, Clone, Default)]
struct DeltaDistribution {
    delta_5d_count: usize,
    delta_10d_count: usize,
    delta_5d_p10: f64,
    delta_5d_p25: f64,
    delta_5d_p50: f64,
    delta_5d_p75: f64,
    delta_5d_p90: f64,
    delta_10d_p10: f64,
    delta_10d_p25: f64,
    delta_10d_p50: f64,
    delta_10d_p75: f64,
    delta_10d_p90: f64,
    breadth_pct_min: f64,
    breadth_pct_max: f64,
    breadth_pct_p50: f64,
    symbol_count: usize,
    min_records_per_symbol: usize,
    max_records_per_symbol: usize,
}

fn compute_breadth_delta_distribution(
    by_symbol: &BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>>,
) -> DeltaDistribution {
    let mut delta_5d_values: Vec<f64> = Vec::new();
    let mut delta_10d_values: Vec<f64> = Vec::new();
    let mut breadth_pct_values: Vec<f64> = Vec::new();

    let mut min_records: Option<usize> = None;
    let mut max_records: Option<usize> = None;

    for (_symbol, by_date) in by_symbol {
        let symbol_count = by_date.len();
        min_records = Some(min_records.map_or(symbol_count, |m| m.min(symbol_count)));
        max_records = Some(max_records.map_or(symbol_count, |m| m.max(symbol_count)));

        for (date, record) in by_date {
            let delta_5d = compute_breadth_delta(*record, *date, by_date, 5);
            let delta_10d = compute_breadth_delta(*record, *date, by_date, 10);
            let breadth_pct = record.event.request.market_view.breadth.breadth_pct;

            delta_5d_values.push(delta_5d);
            delta_10d_values.push(delta_10d);
            breadth_pct_values.push(breadth_pct);
        }
    }

    DeltaDistribution {
        delta_5d_count: delta_5d_values.len(),
        delta_10d_count: delta_10d_values.len(),
        delta_5d_p10: percentile(&delta_5d_values, 0.10),
        delta_5d_p25: percentile(&delta_5d_values, 0.25),
        delta_5d_p50: percentile(&delta_5d_values, 0.50),
        delta_5d_p75: percentile(&delta_5d_values, 0.75),
        delta_5d_p90: percentile(&delta_5d_values, 0.90),
        delta_10d_p10: percentile(&delta_10d_values, 0.10),
        delta_10d_p25: percentile(&delta_10d_values, 0.25),
        delta_10d_p50: percentile(&delta_10d_values, 0.50),
        delta_10d_p75: percentile(&delta_10d_values, 0.75),
        delta_10d_p90: percentile(&delta_10d_values, 0.90),
        breadth_pct_min: if let Some(&m) = breadth_pct_values.iter().min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) { m } else { 0.0 },
        breadth_pct_max: if let Some(&m) = breadth_pct_values.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) { m } else { 0.0 },
        breadth_pct_p50: percentile(&breadth_pct_values, 0.50),
        symbol_count: by_symbol.len(),
        min_records_per_symbol: min_records.unwrap_or(0),
        max_records_per_symbol: max_records.unwrap_or(0),
    }
}

fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let idx = (p * (n as f64 - 1.0)).round() as usize;
    sorted[idx.clamp(0, n - 1)]
}

fn build_breadth_deterioration_verdict(
    samples: usize,
    baseline_negative_t20_rate: f64,
    negative_t20_rate: f64,
    lift_t20: f64,
    avg_t20: f64,
    baseline_avg_t20: f64,
    sub_stats: &BreadthSubStats,
    distribution: &DeltaDistribution,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "Breadth diagnostic: symbols={}, records per symbol min/max={}/{}, breadth_pct min/max/p50={:.1}/{:.1}/{:.1}. delta_5d n={}, P10={:.1} P25={:.1} P50={:.1} P75={:.1} P90={:.1}; delta_10d n={}, P10={:.1} P25={:.1} P50={:.1} P75={:.1} P90={:.1}.",
        distribution.symbol_count,
        distribution.min_records_per_symbol,
        distribution.max_records_per_symbol,
        distribution.breadth_pct_min,
        distribution.breadth_pct_max,
        distribution.breadth_pct_p50,
        distribution.delta_5d_count,
        distribution.delta_5d_p10,
        distribution.delta_5d_p25,
        distribution.delta_5d_p50,
        distribution.delta_5d_p75,
        distribution.delta_5d_p90,
        distribution.delta_10d_count,
        distribution.delta_10d_p10,
        distribution.delta_10d_p25,
        distribution.delta_10d_p50,
        distribution.delta_10d_p75,
        distribution.delta_10d_p90,
    ));

    if distribution.breadth_pct_min == distribution.breadth_pct_max {
        parts.push(format!(
            "CRITICAL: breadth_pct is constant ({:.1}) across all records. BreadthDeterioration cannot be computed from this data. The ExecutionMarketView.breadth field is not populated with real breadth data. Upstream data pipeline must be fixed before BreadthDeterioration can be validated.",
            distribution.breadth_pct_min
        ));
        return parts.join(" ");
    }

    if samples == 0 {
        parts.push("No BreadthDeterioration samples detected with current thresholds. Use distribution to adjust thresholds.".to_string());
        return parts.join(" ");
    }

    parts.push(format!(
        "BreadthDeterioration samples: {}. Baseline negative T+20: {:.1}%. Signal negative T+20: {:.1}%. Lift: {:.2}.",
        samples,
        baseline_negative_t20_rate * 100.0,
        negative_t20_rate * 100.0,
        lift_t20
    ));

    if samples < 30 {
        parts.push(format!(
            "Sample size below ADR-101 minimum ({} < 30). Result is exploratory, not validated.",
            samples
        ));
    } else if negative_t20_rate >= 0.50 && lift_t20 >= 1.2 {
        parts.push("Meets ADR-101 thresholds (n >= 30, precision >= 50%, lift >= 1.2). Candidate is validated for promotion discussion.".to_string());
    } else if lift_t20 >= 1.2 {
        parts.push("Lift >= 1.2 but precision < 50%. Signal has directional value but insufficient accuracy for exit decisions.".to_string());
    } else if negative_t20_rate >= 0.50 {
        parts.push("Precision >= 50% but lift < 1.2. Signal is not better than naive baseline.".to_string());
    } else {
        parts.push("Does not meet ADR-101 thresholds. Reject or iterate on detection logic.".to_string());
    }

    parts.push(format!(
        "Average T+20: BreadthDeterioration={:.2}%, baseline={:.2}%.",
        avg_t20 * 100.0,
        baseline_avg_t20 * 100.0
    ));

    parts.push(format!(
        "Sub-breakdown T+20 negative rate: delta_5d_only={:.1}% (n={}), delta_10d_only={:.1}% (n={}), both={:.1}% (n={}).",
        sub_stats.delta5_negative_t20 * 100.0,
        sub_stats.delta5_count,
        sub_stats.delta10_negative_t20 * 100.0,
        sub_stats.delta10_count,
        sub_stats.both_negative_t20 * 100.0,
        sub_stats.both_count
    ));

    parts.join(" ")
}

pub struct LeadershipDecaySignal {
    pub leadership_now: f64,
    pub leadership_delta_5d: f64,
    pub leadership_delta_10d: f64,
    pub consecutive_decline_days: usize,
    pub decay_score: f64,
    pub triggered_by: String,
}

impl LeadershipDecaySignal {
    pub fn is_leadership_decay(&self) -> bool {
        self.decay_score >= 1.0
    }
}

/// Breakdown of LeadershipDecay samples by the source of decay.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeadershipDecayBreakdown {
    pub delta_5d_only: usize,
    pub delta_10d_only: usize,
    pub both: usize,
}

fn compute_leadership_decay_analysis(records: &[ExecutionResearchRecord]) -> TransitionAnalysis {
    let total_records = records.len();

    let baseline: Vec<&ExecutionResearchRecord> = records.iter().collect();
    let (base_neg_t20, base_neg_t60, base_count_t20, base_count_t60, base_sum_t20, base_sum_t60) = aggregate_outcomes(&baseline);
    let baseline_negative_t20_rate = safe_rate(base_neg_t20, base_count_t20);
    let baseline_negative_t60_rate = safe_rate(base_neg_t60, base_count_t60);
    let baseline_avg_t20 = safe_avg(base_sum_t20, base_count_t20);
    let _baseline_avg_t60 = safe_avg(base_sum_t60, base_count_t60);

    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> = BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut sample_records: Vec<&ExecutionResearchRecord> = Vec::new();
    let mut breakdown = LeadershipDecayBreakdown::default();
    let mut delta5_records: Vec<&ExecutionResearchRecord> = Vec::new();
    let mut delta10_records: Vec<&ExecutionResearchRecord> = Vec::new();
    let mut both_records: Vec<&ExecutionResearchRecord> = Vec::new();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let signal = detect_leadership_decay(*record, *date, by_date);
            if signal.is_leadership_decay() {
                sample_records.push(record);
                match signal.triggered_by.as_str() {
                    "delta_5d" => {
                        breakdown.delta_5d_only += 1;
                        delta5_records.push(record);
                    }
                    "delta_10d" => {
                        breakdown.delta_10d_only += 1;
                        delta10_records.push(record);
                    }
                    "both" => {
                        breakdown.both += 1;
                        both_records.push(record);
                    }
                    _ => {}
                }
            }
        }
    }

    let (sample_neg_t20, sample_neg_t60, sample_count_t20, sample_count_t60, sample_sum_t20, sample_sum_t60) =
        aggregate_outcomes(&sample_records);
    let samples = sample_records.len();

    let negative_t20_rate = safe_rate(sample_neg_t20, sample_count_t20);
    let negative_t60_rate = safe_rate(sample_neg_t60, sample_count_t60);
    let avg_t20 = safe_avg(sample_sum_t20, sample_count_t20);
    let avg_t60 = safe_avg(sample_sum_t60, sample_count_t60);

    let lift_t20 = if baseline_negative_t20_rate > 0.0 {
        negative_t20_rate / baseline_negative_t20_rate
    } else {
        0.0
    };
    let lift_t60 = if baseline_negative_t60_rate > 0.0 {
        negative_t60_rate / baseline_negative_t60_rate
    } else {
        0.0
    };

    let sub_stats = compute_leadership_sub_stats(&delta5_records, &delta10_records, &both_records);
    let distribution = compute_leadership_distribution(&by_symbol);
    let verdict = build_leadership_decay_verdict(
        samples,
        baseline_negative_t20_rate,
        negative_t20_rate,
        lift_t20,
        avg_t20,
        baseline_avg_t20,
        &sub_stats,
        &distribution,
    );

    TransitionAnalysis {
        candidate: TransitionCandidate::LeadershipDecay,
        total_records,
        samples,
        baseline_negative_t20_rate,
        baseline_negative_t60_rate,
        negative_t20_rate,
        negative_t60_rate,
        precision_t20: negative_t20_rate,
        precision_t60: negative_t60_rate,
        lift_t20,
        lift_t60,
        avg_t20,
        avg_t60,
        breakdown: TransitionBreakdown::LeadershipDecay(breakdown),
        verdict,
    }
}

pub(crate) fn detect_leadership_decay(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> LeadershipDecaySignal {
    const DELTA_5D_THRESHOLD: f64 = -0.15;
    const DELTA_10D_THRESHOLD: f64 = -0.25;

    let leadership_now = record.event.request.market_view.leadership_stability;
    let leadership_delta_5d = compute_leadership_delta(record, date, by_date, 5);
    let leadership_delta_10d = compute_leadership_delta(record, date, by_date, 10);

    let delta_5d_trigger = leadership_delta_5d < DELTA_5D_THRESHOLD;
    let delta_10d_trigger = leadership_delta_10d < DELTA_10D_THRESHOLD;

    let triggered_by = if delta_5d_trigger && delta_10d_trigger {
        "both"
    } else if delta_5d_trigger {
        "delta_5d"
    } else if delta_10d_trigger {
        "delta_10d"
    } else {
        ""
    };

    let decay_score = if delta_5d_trigger || delta_10d_trigger {
        1.0
    } else {
        0.0
    };

    let consecutive_decline_days = count_leadership_decline_days(date, by_date);

    LeadershipDecaySignal {
        leadership_now,
        leadership_delta_5d,
        leadership_delta_10d,
        consecutive_decline_days,
        decay_score,
        triggered_by: triggered_by.to_string(),
    }
}

fn compute_leadership_delta(
    record: &ExecutionResearchRecord,
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
    trading_days_ago: usize,
) -> f64 {
    let current = record.event.request.market_view.leadership_stability;

    let mut count = 0usize;
    for (_past_date, past) in by_date.range(..date).rev() {
        count += 1;
        if count == trading_days_ago {
            let past_leadership = past.event.request.market_view.leadership_stability;
            return current - past_leadership;
        }
    }
    0.0
}

fn count_leadership_decline_days(
    date: NaiveDate,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> usize {
    let mut consecutive = 0usize;
    for days_back in 1..=10 {
        if let Some(prev_date) = date.checked_sub_signed(chrono::Duration::days(days_back)) {
            if let Some(prev) = by_date.get(&prev_date) {
                if let Some(current) = by_date.get(&date) {
                    let current_leadership = current.event.request.market_view.leadership_stability;
                    let prev_leadership = prev.event.request.market_view.leadership_stability;
                    if current_leadership < prev_leadership {
                        consecutive += 1;
                    } else {
                        break;
                    }
                }
            }
        }
    }
    consecutive
}

#[derive(Debug, Clone, Default)]
struct LeadershipSubStats {
    delta5_negative_t20: f64,
    delta10_negative_t20: f64,
    both_negative_t20: f64,
    delta5_count: usize,
    delta10_count: usize,
    both_count: usize,
}

fn compute_leadership_sub_stats(
    delta5: &[&ExecutionResearchRecord],
    delta10: &[&ExecutionResearchRecord],
    both: &[&ExecutionResearchRecord],
) -> LeadershipSubStats {
    let (n5, _, c5, _, _, _) = aggregate_outcomes(delta5);
    let (n10, _, c10, _, _, _) = aggregate_outcomes(delta10);
    let (nb, _, cb, _, _, _) = aggregate_outcomes(both);

    LeadershipSubStats {
        delta5_negative_t20: safe_rate(n5, c5),
        delta10_negative_t20: safe_rate(n10, c10),
        both_negative_t20: safe_rate(nb, cb),
        delta5_count: delta5.len(),
        delta10_count: delta10.len(),
        both_count: both.len(),
    }
}

#[derive(Debug, Clone, Default)]
struct LeadershipDistribution {
    leadership_min: f64,
    leadership_max: f64,
    leadership_p50: f64,
    delta_5d_p10: f64,
    delta_5d_p25: f64,
    delta_5d_p50: f64,
    delta_5d_p75: f64,
    delta_5d_p90: f64,
    delta_10d_p10: f64,
    delta_10d_p25: f64,
    delta_10d_p50: f64,
    delta_10d_p75: f64,
    delta_10d_p90: f64,
}

fn compute_leadership_distribution(
    by_symbol: &BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>>,
) -> LeadershipDistribution {
    let mut leadership_values: Vec<f64> = Vec::new();
    let mut delta_5d_values: Vec<f64> = Vec::new();
    let mut delta_10d_values: Vec<f64> = Vec::new();

    for (_symbol, by_date) in by_symbol {
        for (date, record) in by_date {
            let leadership = record.event.request.market_view.leadership_stability;
            let delta_5d = compute_leadership_delta(*record, *date, by_date, 5);
            let delta_10d = compute_leadership_delta(*record, *date, by_date, 10);

            leadership_values.push(leadership);
            delta_5d_values.push(delta_5d);
            delta_10d_values.push(delta_10d);
        }
    }

    LeadershipDistribution {
        leadership_min: if let Some(&m) = leadership_values.iter().min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) { m } else { 0.0 },
        leadership_max: if let Some(&m) = leadership_values.iter().max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) { m } else { 0.0 },
        leadership_p50: percentile(&leadership_values, 0.50),
        delta_5d_p10: percentile(&delta_5d_values, 0.10),
        delta_5d_p25: percentile(&delta_5d_values, 0.25),
        delta_5d_p50: percentile(&delta_5d_values, 0.50),
        delta_5d_p75: percentile(&delta_5d_values, 0.75),
        delta_5d_p90: percentile(&delta_5d_values, 0.90),
        delta_10d_p10: percentile(&delta_10d_values, 0.10),
        delta_10d_p25: percentile(&delta_10d_values, 0.25),
        delta_10d_p50: percentile(&delta_10d_values, 0.50),
        delta_10d_p75: percentile(&delta_10d_values, 0.75),
        delta_10d_p90: percentile(&delta_10d_values, 0.90),
    }
}

fn build_leadership_decay_verdict(
    samples: usize,
    baseline_negative_t20_rate: f64,
    negative_t20_rate: f64,
    lift_t20: f64,
    avg_t20: f64,
    baseline_avg_t20: f64,
    sub_stats: &LeadershipSubStats,
    distribution: &LeadershipDistribution,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "Leadership diagnostic: stability min/max/p50={:.2}/{:.2}/{:.2}. delta_5d P10={:.2} P25={:.2} P50={:.2} P75={:.2} P90={:.2}; delta_10d P10={:.2} P25={:.2} P50={:.2} P75={:.2} P90={:.2}.",
        distribution.leadership_min,
        distribution.leadership_max,
        distribution.leadership_p50,
        distribution.delta_5d_p10,
        distribution.delta_5d_p25,
        distribution.delta_5d_p50,
        distribution.delta_5d_p75,
        distribution.delta_5d_p90,
        distribution.delta_10d_p10,
        distribution.delta_10d_p25,
        distribution.delta_10d_p50,
        distribution.delta_10d_p75,
        distribution.delta_10d_p90,
    ));

    if distribution.leadership_min == distribution.leadership_max {
        parts.push(format!(
            "CRITICAL: leadership_stability is constant ({:.2}) across all records. LeadershipDecay cannot be computed from this data. Upstream data pipeline must be fixed.",
            distribution.leadership_min
        ));
        return parts.join(" ");
    }

    if samples == 0 {
        parts.push("No LeadershipDecay samples detected with current thresholds. Use distribution to adjust thresholds.".to_string());
        return parts.join(" ");
    }

    parts.push(format!(
        "LeadershipDecay samples: {}. Baseline negative T+20: {:.1}%. Signal negative T+20: {:.1}%. Lift: {:.2}.",
        samples,
        baseline_negative_t20_rate * 100.0,
        negative_t20_rate * 100.0,
        lift_t20
    ));

    if samples < 30 {
        parts.push(format!(
            "Sample size below ADR-101 minimum ({} < 30). Result is exploratory, not validated.",
            samples
        ));
    } else if negative_t20_rate >= 0.50 && lift_t20 >= 1.2 {
        parts.push("Meets ADR-101 thresholds (n >= 30, precision >= 50%, lift >= 1.2). Candidate is validated for promotion discussion.".to_string());
    } else if lift_t20 >= 1.2 {
        parts.push("Lift >= 1.2 but precision < 50%. Signal has directional value but insufficient accuracy for exit decisions.".to_string());
    } else if negative_t20_rate >= 0.50 {
        parts.push("Precision >= 50% but lift < 1.2. Signal is not better than naive baseline.".to_string());
    } else {
        parts.push("Does not meet ADR-101 thresholds. Reject or iterate on detection logic.".to_string());
    }

    parts.push(format!(
        "Average T+20: LeadershipDecay={:.2}%, baseline={:.2}%.",
        avg_t20 * 100.0,
        baseline_avg_t20 * 100.0
    ));

    parts.push(format!(
        "Sub-breakdown T+20 negative rate: delta_5d_only={:.1}% (n={}), delta_10d_only={:.1}% (n={}), both={:.1}% (n={}).",
        sub_stats.delta5_negative_t20 * 100.0,
        sub_stats.delta5_count,
        sub_stats.delta10_negative_t20 * 100.0,
        sub_stats.delta10_count,
        sub_stats.both_negative_t20 * 100.0,
        sub_stats.both_count
    ));

    parts.join(" ")
}

// =============================================================================
// TASK-157: LeadershipDecay Horizon Analysis
// =============================================================================

/// Horizon profile for a single candidate signal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HorizonProfile {
    pub horizon_label: String,
    pub sample_count: usize,
    pub total_count: usize,
    pub baseline_negative_rate: f64,
    pub signal_negative_rate: f64,
    pub lift: f64,
    pub precision: f64,
    pub avg_signal_return: f64,
    pub avg_baseline_return: f64,
    pub median_signal_return: f64,
    pub median_baseline_return: f64,
    pub max_drawdown_mean_signal: f64,
    pub max_drawdown_mean_baseline: f64,
}

/// LeadershipDecay signal profile across multiple horizons.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeadershipDecayHorizonAnalysis {
    pub total_records: usize,
    pub leadership_decay_samples: usize,
    pub breakdown: LeadershipDecayBreakdown,
    pub profiles: Vec<HorizonProfile>,
    pub verdict: String,
}

/// Computes the LeadershipDecay horizon profile across T+5, T+20, T+60, T+120.
///
/// This is a research-only analysis. It does not modify any Observation, Evidence,
/// Assessment, Decision, or Policy code. It answers the question:
///
/// > Is LeadershipDecay a short-term exit signal or a medium-term holding risk signal?
pub fn compute_leadership_decay_horizon_analysis(
    records: &[ExecutionResearchRecord],
) -> LeadershipDecayHorizonAnalysis {
    let total_records = records.len();

    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut sample_records: Vec<&ExecutionResearchRecord> = Vec::new();
    let mut breakdown = LeadershipDecayBreakdown::default();

    for (_symbol, by_date) in &by_symbol {
        for (date, record) in by_date {
            let signal = detect_leadership_decay(*record, *date, by_date);
            if signal.is_leadership_decay() {
                sample_records.push(record);
                match signal.triggered_by.as_str() {
                    "delta_5d" => breakdown.delta_5d_only += 1,
                    "delta_10d" => breakdown.delta_10d_only += 1,
                    "both" => breakdown.both += 1,
                    _ => {}
                }
            }
        }
    }

    let profiles = vec![
        build_horizon_profile(
            "T+5",
            records,
            &sample_records,
            |o| o.t5_return,
            |o| o.t5_return.and_then(|_| o.max_drawdown),
        ),
        build_horizon_profile(
            "T+20",
            records,
            &sample_records,
            |o| o.t20_return,
            |o| o.t20_return.and_then(|_| o.max_drawdown),
        ),
        build_horizon_profile(
            "T+60",
            records,
            &sample_records,
            |o| o.t60_return,
            |o| o.t60_return.and_then(|_| o.max_drawdown),
        ),
        build_horizon_profile(
            "T+120",
            records,
            &sample_records,
            |o| o.t120_return,
            |o| o.t120_return.and_then(|_| o.max_drawdown),
        ),
    ];

    let verdict = build_horizon_verdict(&profiles, total_records, sample_records.len());

    LeadershipDecayHorizonAnalysis {
        total_records,
        leadership_decay_samples: sample_records.len(),
        breakdown,
        profiles,
        verdict,
    }
}

fn build_horizon_profile(
    horizon_label: &str,
    all_records: &[ExecutionResearchRecord],
    samples: &[&ExecutionResearchRecord],
    return_extractor: fn(&ExecutionOutcome) -> Option<f64>,
    dd_extractor: fn(&ExecutionOutcome) -> Option<f64>,
) -> HorizonProfile {
    let baseline_returns: Vec<f64> = all_records
        .iter()
        .filter_map(|r| return_extractor(&r.outcome))
        .collect();
    let signal_returns: Vec<f64> = samples
        .iter()
        .filter_map(|r| return_extractor(&r.outcome))
        .collect();

    let baseline_dds: Vec<f64> = all_records
        .iter()
        .filter_map(|r| dd_extractor(&r.outcome))
        .collect();
    let signal_dds: Vec<f64> = samples
        .iter()
        .filter_map(|r| dd_extractor(&r.outcome))
        .collect();

    let baseline_negative = baseline_returns.iter().filter(|&&r| r < 0.0).count();
    let signal_negative = signal_returns.iter().filter(|&&r| r < 0.0).count();

    let baseline_negative_rate = safe_rate(baseline_negative, baseline_returns.len());
    let signal_negative_rate = safe_rate(signal_negative, signal_returns.len());
    let lift = if baseline_negative_rate > 1e-9 {
        signal_negative_rate / baseline_negative_rate
    } else {
        1.0
    };
    let precision = signal_negative_rate; // negative rate is precision for a risk signal

    let avg_baseline_return = safe_avg(baseline_returns.iter().copied().sum(), baseline_returns.len());
    let avg_signal_return = safe_avg(signal_returns.iter().copied().sum(), signal_returns.len());

    let median_baseline_return = median(&baseline_returns);
    let median_signal_return = median(&signal_returns);

    let avg_baseline_dd = safe_avg(baseline_dds.iter().copied().sum(), baseline_dds.len());
    let avg_signal_dd = safe_avg(signal_dds.iter().copied().sum(), signal_dds.len());

    HorizonProfile {
        horizon_label: horizon_label.to_string(),
        sample_count: samples.len(),
        total_count: all_records.len(),
        baseline_negative_rate,
        signal_negative_rate,
        lift,
        precision,
        avg_signal_return,
        avg_baseline_return,
        median_signal_return,
        median_baseline_return,
        max_drawdown_mean_signal: avg_signal_dd,
        max_drawdown_mean_baseline: avg_baseline_dd,
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

fn build_horizon_verdict(
    profiles: &[HorizonProfile],
    total_records: usize,
    samples: usize,
) -> String {
    let mut parts = Vec::new();
    parts.push(format!(
        "LeadershipDecay samples: {} / {} records. Horizon profile:",
        samples, total_records
    ));

    for p in profiles {
        parts.push(format!(
            "- {}: negative_rate={:.1}%, baseline={:.1}%, lift={:.2}, precision={:.1}%, avg_return={:.2}%, median={:.2}%, avg_dd={:.2}%",
            p.horizon_label,
            p.signal_negative_rate * 100.0,
            p.baseline_negative_rate * 100.0,
            p.lift,
            p.precision * 100.0,
            p.avg_signal_return * 100.0,
            p.median_signal_return * 100.0,
            p.max_drawdown_mean_signal * 100.0
        ));
    }

    let mut classification = "unclear";
    if let (Some(t20), Some(t60)) = (profiles.iter().find(|p| p.horizon_label == "T+20"), profiles.iter().find(|p| p.horizon_label == "T+60")) {
        if t60.lift > t20.lift && t60.lift >= 1.2 && t60.precision >= 0.50 {
            classification = "medium-term holding risk signal";
        } else if t20.lift >= 1.2 && t20.precision >= 0.50 {
            classification = "short-term exit signal";
        } else if t60.lift > 1.0 && t20.lift <= 1.0 {
            classification = "emerging medium-term holding risk signal (requires iteration)";
        }
    }

    parts.push(format!(
        "Classification: LeadershipDecay appears to be a '{}'.",
        classification
    ));

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
        today_return: f64,
        close_position: f64,
        breadth_pct: f64,
        breadth_delta: f64,
        leadership_stability: f64,
        t20_return: f64,
    ) -> ExecutionResearchRecord {
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
                close: 10.0 * (1.0 + today_return),
                volume: 1_000_000.0,
                prev_close: 10.0,
            },
            volume_ma20: 500_000.0,
            market_view: ExecutionMarketView {
                research_version: "1".into(),
                market_regime_label: "Bullish".into(),
                confirmation: ConfirmationSummary {
                    trend: ConfirmationDimension {
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    participation: ConfirmationDimension {
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    risk: ConfirmationDimension {
                        score: 50.0,
                        label: "Moderate".into(),
                    },
                    overall: "Moderate".into(),
                },
                breadth: BreadthSummary {
                    breadth_pct,
                    sma5: None,
                    delta_5d: Some(breadth_delta),
                    condition: "moderate".into(),
                },
                recovery: RecoverySummary {
                    score: 55.0,
                    drivers: vec![],
                },
                rotation_state: "mixed".into(),
                leadership_stability,
            },
            policy,
        };

        let features = IntradayFeatures {
            symbol: "000001".into(),
            today_return,
            open_return: 0.0,
            gap_pct: 0.0,
            close_position,
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
                kind: EvidenceKind::Recovery,
                confidence: 0.8,
                direction: 1.0,
                source: EvidenceSource::ResearchContext,
                payload: EvidencePayload::Empty,
            }],
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };

        let event = ExecutionEvent::new(request, features, vec![], vec![], assessment, decision);
        ExecutionResearchRecord {
            event,
            outcome: crate::ExecutionOutcome {
                t20_return: Some(t20_return),
                ..Default::default()
            },
            evaluation: crate::ExecutionEvaluation::AwaitingOutcome,
            evaluation_version: "v1.0.0-rule-based".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn recovery_failure_detected_after_pressure() {
        let d0 = NaiveDate::from_ymd_opt(2026, 7, 10).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2026, 7, 11).unwrap();

        let pressure = make_record(d0, -0.03, 0.1, 35.0, -5.0, 0.6, 0.0);
        // Recovery attempt but breadth and leadership still weak.
        let recovery = make_record(d1, 0.005, 0.5, 32.0, -4.0, 0.5, -0.05);

        let analysis = compute_recovery_failure_analysis(&[pressure, recovery]);
        assert_eq!(analysis.samples, 1);
        assert!(analysis.negative_t20_rate >= 0.99);
    }

    #[test]
    fn no_recovery_failure_without_pressure() {
        let d0 = NaiveDate::from_ymd_opt(2026, 7, 10).unwrap();
        let d1 = NaiveDate::from_ymd_opt(2026, 7, 11).unwrap();

        let normal = make_record(d0, 0.01, 0.5, 60.0, 2.0, 0.7, 0.0);
        let next = make_record(d1, 0.005, 0.5, 58.0, 1.0, 0.65, 0.05);

        let analysis = compute_recovery_failure_analysis(&[normal, next]);
        assert_eq!(analysis.samples, 0);
    }

    #[test]
    fn parse_candidate_recovery_failure() {
        let candidate: TransitionCandidate = "recovery_failure".parse().unwrap();
        assert_eq!(candidate, TransitionCandidate::RecoveryFailure);
    }
}

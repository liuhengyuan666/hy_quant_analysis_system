use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::{
    holding_risk_calibration::compute_holding_risk_score,
    ExecutionResearchRecord,
};

/// TASK-163: Holding Risk Lifecycle Modeling.
///
/// Builds a risk state machine around HoldingRiskScore:
/// - Risk Entry: score >= 0.75 for >= 2 consecutive days
/// - Risk Peak: local maximum score during the event
/// - Risk Recovery: score < 0.5 for >= 2 consecutive days
/// - Holding Period: duration from entry to recovery
/// - False Alarm: event with T+60 return >= 0
///
/// Research-only; does not modify the Execution Pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLifecycleAnalysis {
    pub total_records: usize,
    pub total_events: usize,
    pub avg_duration_days: f64,
    pub median_duration_days: f64,
    pub avg_peak_score: f64,
    pub avg_recovery_days: f64,
    pub false_alarm_rate: f64,
    pub avg_t60_return: f64,
    pub avg_max_drawdown: f64,
    pub events: Vec<RiskLifecycleEvent>,
    pub verdict: String,
}

/// A single Holding Risk lifecycle event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLifecycleEvent {
    pub symbol: String,
    pub entry_date: NaiveDate,
    pub peak_date: NaiveDate,
    pub recovery_date: Option<NaiveDate>,
    pub duration_days: usize,
    pub peak_score: f64,
    pub avg_t60_return: Option<f64>,
    pub max_drawdown: Option<f64>,
    pub is_false_alarm: bool,
}

/// Computes the Holding Risk Lifecycle analysis.
pub fn compute_risk_lifecycle_analysis(
    records: &[ExecutionResearchRecord],
) -> RiskLifecycleAnalysis {
    let total_records = records.len();

    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut events = Vec::new();
    for (symbol, by_date) in &by_symbol {
        let symbol_events = detect_lifecycle_events(symbol, by_date);
        events.extend(symbol_events);
    }

    let stats = compute_event_stats(&events);

    let verdict = build_verdict(&stats);

    RiskLifecycleAnalysis {
        total_records,
        total_events: events.len(),
        avg_duration_days: stats.avg_duration,
        median_duration_days: stats.median_duration,
        avg_peak_score: stats.avg_peak,
        avg_recovery_days: stats.avg_recovery_days,
        false_alarm_rate: stats.false_alarm_rate,
        avg_t60_return: stats.avg_t60_return,
        avg_max_drawdown: stats.avg_max_drawdown,
        events,
        verdict,
    }
}

struct EventStats {
    avg_duration: f64,
    median_duration: f64,
    avg_peak: f64,
    avg_recovery_days: f64,
    false_alarm_rate: f64,
    avg_t60_return: f64,
    avg_max_drawdown: f64,
}

fn detect_lifecycle_events(
    symbol: &str,
    by_date: &BTreeMap<NaiveDate, &ExecutionResearchRecord>,
) -> Vec<RiskLifecycleEvent> {
    const ENTRY_THRESHOLD: f64 = 0.75;
    const RECOVERY_THRESHOLD: f64 = 0.50;
    const ENTRY_PERSISTENCE: usize = 2;
    const RECOVERY_PERSISTENCE: usize = 2;

    let mut events = Vec::new();
    let mut in_event = false;
    let mut entry_date = None;
    let mut peak_date = None;
    let mut peak_score = 0.0;
    let mut event_scores = Vec::new();

    let dates: Vec<NaiveDate> = by_date.keys().copied().collect();
    for (i, date) in dates.iter().enumerate() {
        let record = by_date[date];
        let score = compute_holding_risk_score(record, *date, by_date);

        if !in_event {
            // Check for entry: score >= ENTRY_THRESHOLD for ENTRY_PERSISTENCE consecutive days
            if score >= ENTRY_THRESHOLD {
                let mut persistence = 0usize;
                for j in (i.saturating_sub(ENTRY_PERSISTENCE - 1))..=i {
                    let prev_record = by_date[&dates[j]];
                    let prev_score = compute_holding_risk_score(prev_record, dates[j], by_date);
                    if prev_score >= ENTRY_THRESHOLD {
                        persistence += 1;
                    } else {
                        break;
                    }
                }
                if persistence >= ENTRY_PERSISTENCE {
                    in_event = true;
                    entry_date = Some(*date);
                    peak_date = Some(*date);
                    peak_score = score;
                    event_scores.push((date, score, record));
                }
            }
        } else {
            event_scores.push((date, score, record));
            if score > peak_score {
                peak_score = score;
                peak_date = Some(*date);
            }

            // Check for recovery: score < RECOVERY_THRESHOLD for RECOVERY_PERSISTENCE consecutive days
            if score < RECOVERY_THRESHOLD {
                let mut persistence = 0usize;
                for j in i..=(i + RECOVERY_PERSISTENCE - 1).min(dates.len() - 1) {
                    let next_record = by_date[&dates[j]];
                    let next_score = compute_holding_risk_score(next_record, dates[j], by_date);
                    if next_score < RECOVERY_THRESHOLD {
                        persistence += 1;
                    } else {
                        break;
                    }
                }
                if persistence >= RECOVERY_PERSISTENCE {
                    let recovery_date = Some(dates[i + RECOVERY_PERSISTENCE - 1]);
                    let duration = recovery_date
                        .unwrap()
                        .signed_duration_since(entry_date.unwrap())
                        .num_days() as usize;

                    let avg_t60 = event_scores
                        .iter()
                        .filter_map(|(_, _, r)| r.outcome.t60_return)
                        .sum::<f64>()
                        / event_scores.len() as f64;
                    let max_dd = event_scores
                        .iter()
                        .filter_map(|(_, _, r)| r.outcome.max_drawdown)
                        .fold(f64::NEG_INFINITY, |a, b| a.max(b));
                    let is_false_alarm = avg_t60 >= 0.0;

                    events.push(RiskLifecycleEvent {
                        symbol: symbol.to_string(),
                        entry_date: entry_date.unwrap(),
                        peak_date: peak_date.unwrap(),
                        recovery_date,
                        duration_days: duration,
                        peak_score,
                        avg_t60_return: Some(avg_t60),
                        max_drawdown: Some(max_dd),
                        is_false_alarm,
                    });

                    in_event = false;
                    entry_date = None;
                    peak_date = None;
                    peak_score = 0.0;
                    event_scores.clear();
                }
            }
        }
    }

    // Handle unclosed event at end of data
    if in_event {
        let last_date = *dates.last().unwrap();
        let duration = last_date
            .signed_duration_since(entry_date.unwrap())
            .num_days() as usize;
        let avg_t60 = event_scores
            .iter()
            .filter_map(|(_, _, r)| r.outcome.t60_return)
            .sum::<f64>()
            / event_scores.len() as f64;
        let max_dd = event_scores
            .iter()
            .filter_map(|(_, _, r)| r.outcome.max_drawdown)
            .fold(f64::NEG_INFINITY, |a, b| a.max(b));
        let is_false_alarm = avg_t60 >= 0.0;

        events.push(RiskLifecycleEvent {
            symbol: symbol.to_string(),
            entry_date: entry_date.unwrap(),
            peak_date: peak_date.unwrap(),
            recovery_date: None,
            duration_days: duration,
            peak_score,
            avg_t60_return: Some(avg_t60),
            max_drawdown: Some(max_dd),
            is_false_alarm,
        });
    }

    events
}

fn compute_event_stats(events: &[RiskLifecycleEvent]) -> EventStats {
    if events.is_empty() {
        return EventStats {
            avg_duration: 0.0,
            median_duration: 0.0,
            avg_peak: 0.0,
            avg_recovery_days: 0.0,
            false_alarm_rate: 0.0,
            avg_t60_return: 0.0,
            avg_max_drawdown: 0.0,
        };
    }

    let durations: Vec<f64> = events.iter().map(|e| e.duration_days as f64).collect();
    let avg_duration = durations.iter().sum::<f64>() / durations.len() as f64;
    let median_duration = median(&durations);

    let peaks: Vec<f64> = events.iter().map(|e| e.peak_score).collect();
    let avg_peak = peaks.iter().sum::<f64>() / peaks.len() as f64;

    let recovery_days: Vec<f64> = events
        .iter()
        .filter_map(|e| e.recovery_date.map(|r| r.signed_duration_since(e.entry_date).num_days() as f64))
        .collect();
    let avg_recovery_days = if recovery_days.is_empty() {
        0.0
    } else {
        recovery_days.iter().sum::<f64>() / recovery_days.len() as f64
    };

    let false_alarms = events.iter().filter(|e| e.is_false_alarm).count();
    let false_alarm_rate = false_alarms as f64 / events.len() as f64;

    let t60_returns: Vec<f64> = events
        .iter()
        .filter_map(|e| e.avg_t60_return)
        .collect();
    let avg_t60_return = if t60_returns.is_empty() {
        0.0
    } else {
        t60_returns.iter().sum::<f64>() / t60_returns.len() as f64
    };

    let drawdowns: Vec<f64> = events
        .iter()
        .filter_map(|e| e.max_drawdown)
        .collect();
    let avg_max_drawdown = if drawdowns.is_empty() {
        0.0
    } else {
        drawdowns.iter().sum::<f64>() / drawdowns.len() as f64
    };

    EventStats {
        avg_duration,
        median_duration,
        avg_peak,
        avg_recovery_days,
        false_alarm_rate,
        avg_t60_return,
        avg_max_drawdown,
    }
}

fn build_verdict(stats: &EventStats) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Risk lifecycle statistics: avg duration={:.1} days, avg peak score={:.2}, avg recovery={:.1} days, false alarm rate={:.1}%.",
        stats.avg_duration,
        stats.avg_peak,
        stats.avg_recovery_days,
        stats.false_alarm_rate * 100.0
    ));

    if stats.false_alarm_rate < 0.40 && stats.avg_t60_return < 0.0 {
        lines.push("Risk lifecycle events are consistent with negative T+60 outcomes. The state machine is consistent with Holding Risk semantics.".into());
    } else if stats.avg_t60_return < 0.0 {
        lines.push("Risk lifecycle events show negative T+60 but high false alarm rate. Consider tightening entry conditions.".into());
    } else {
        lines.push("Risk lifecycle events do not consistently predict negative T+60. Re-evaluate entry thresholds.".into());
    }

    lines.join("\n")
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
}

use chrono::NaiveDate;
use core_domain::{
    DailyBar, MarketRegimeSnapshot, PersistenceMechanicsDistribution,
    PersistenceMechanicsEpisode, PersistenceMechanicsPoint, PersistenceMechanicsReport,
};
use std::collections::HashMap;

// ============================================================
// TASK-034B: Persistence Mechanics Audit
// Answers Q1/Q2/Q3 about why the persistence frontier has
// its unusual shape.
// ============================================================

fn classify_raw_regime(trend_score: f64, risk_score: f64, liquidity_score: f64) -> String {
    if trend_score >= 60.0 && liquidity_score >= 50.0 && risk_score >= 55.0 {
        "risk_on".to_string()
    } else if trend_score < 40.0 || risk_score < 40.0 {
        "risk_off".to_string()
    } else {
        "neutral".to_string()
    }
}

fn apply_persistence(raw_labels: &[String], days: usize) -> Vec<String> {
    if days == 0 {
        return raw_labels.to_vec();
    }
    let mut persisted = Vec::with_capacity(raw_labels.len());
    let mut current_regime = "neutral".to_string();
    let mut streak = 0;

    for label in raw_labels {
        if label == &current_regime {
            streak += 1;
        } else {
            streak = 1;
            current_regime = label.clone();
        }

        if streak >= days {
            persisted.push(current_regime.clone());
        } else {
            if persisted.is_empty() {
                persisted.push("neutral".to_string());
            } else {
                persisted.push(persisted.last().unwrap().clone());
            }
        }
    }

    persisted
}

fn extract_episodes(labels: &[String], dates: &[NaiveDate]) -> Vec<(String, NaiveDate, NaiveDate, usize)> {
    if labels.is_empty() {
        return Vec::new();
    }
    let mut episodes = Vec::new();
    let mut current = labels[0].clone();
    let mut start = dates[0];
    let mut count = 1;

    for i in 1..labels.len() {
        if labels[i] == current {
            count += 1;
        } else {
            episodes.push((current.clone(), start, dates[i - 1], count));
            current = labels[i].clone();
            start = dates[i];
            count = 1;
        }
    }
    episodes.push((current, start, dates[dates.len() - 1], count));
    episodes
}

fn count_single_day_flips(episodes: &[(String, NaiveDate, NaiveDate, usize)]) -> usize {
    episodes.iter().filter(|e| e.3 == 1).count()
}

fn compute_distribution(labels: &[String]) -> PersistenceMechanicsDistribution {
    let mut risk_on = 0;
    let mut neutral = 0;
    let mut risk_off = 0;
    for label in labels {
        match label.as_str() {
            "risk_on" => risk_on += 1,
            "neutral" => neutral += 1,
            "risk_off" => risk_off += 1,
            _ => neutral += 1,
        }
    }
    PersistenceMechanicsDistribution {
        risk_on_days: risk_on,
        neutral_days: neutral,
        risk_off_days: risk_off,
        total_days: labels.len(),
    }
}

fn compute_mechanics_for_days(
    raw_labels: &[String],
    dates: &[NaiveDate],
    days: usize,
) -> PersistenceMechanicsPoint {
    let persisted_labels = apply_persistence(raw_labels, days);
    let distribution = compute_distribution(&persisted_labels);
    let raw_episodes = extract_episodes(raw_labels, dates);

    let single_day_flips = count_single_day_flips(&raw_episodes);
    let total_transitions = if raw_episodes.len() > 1 {
        raw_episodes.len() - 1
    } else {
        0
    };

    let mut total_delay = 0usize;
    let mut swallowed = 0usize;
    let mut merged = 0usize;
    let mut mechanics_episodes = Vec::new();

    for (raw_regime, raw_start, raw_end, raw_dur) in &raw_episodes {
        let (delay, confirmed_at_day, is_swallowed) = if days == 0 {
            (0, 1, false)
        } else if *raw_dur >= days {
            (days - 1, days, false)
        } else {
            (*raw_dur, 0, true)
        };

        total_delay += delay;
        if is_swallowed {
            swallowed += 1;
        }

        mechanics_episodes.push(PersistenceMechanicsEpisode {
            regime: raw_regime.clone(),
            start_date: *raw_start,
            end_date: *raw_end,
            duration_days: *raw_dur,
            confirmed_at_day,
            delayed_days: delay,
            swallowed: is_swallowed,
        });
    }

    // Count merged regimes: consecutive same-label episodes separated by swallowed episodes
    if days > 0 {
        let mut i = 0;
        while i < raw_episodes.len() {
            let (regime, _, _, dur) = &raw_episodes[i];
            if *dur >= days {
                let mut j = i + 1;
                while j < raw_episodes.len() && raw_episodes[j].3 < days {
                    j += 1;
                }
                if j < raw_episodes.len() && &raw_episodes[j].0 == regime {
                    merged += 1;
                }
                i = j;
            } else {
                i += 1;
            }
        }
    }

    let avg_delay = if !mechanics_episodes.is_empty() {
        total_delay as f64 / mechanics_episodes.len() as f64
    } else {
        0.0
    };

    PersistenceMechanicsPoint {
        confirmation_days: days,
        distribution,
        single_day_flips,
        total_transitions,
        episodes: mechanics_episodes,
        avg_delay_days: avg_delay,
        swallowed_regimes: swallowed,
        merged_regimes: merged,
    }
}

pub fn compute_persistence_mechanics(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<PersistenceMechanicsReport> {
    if regimes.is_empty() || bars.is_empty() {
        return None;
    }

    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    let regimes_filtered: Vec<_> = regimes
        .iter()
        .filter(|r| close_by_date.contains_key(&r.date))
        .collect();

    if regimes_filtered.len() < 30 {
        return None;
    }

    let total_days = regimes_filtered.len();
    let window_from = regimes_filtered.first().map(|r| r.date).unwrap_or(bars[0].date);
    let window_to = regimes_filtered.last().map(|r| r.date).unwrap_or(bars[bars.len() - 1].date);

    let raw_labels: Vec<String> = regimes_filtered
        .iter()
        .map(|r| classify_raw_regime(r.trend_score, r.risk_score, r.liquidity_score))
        .collect();

    let dates: Vec<NaiveDate> = regimes_filtered.iter().map(|r| r.date).collect();
    let persistence_configs = vec![0, 1, 2, 3, 5, 7, 10];
    let mut points = Vec::new();

    for days in persistence_configs {
        let point = compute_mechanics_for_days(&raw_labels, &dates, days);
        points.push(point);
    }

    // Q1: Single day flips (from 0d/1d point)
    let q1_flips = points.first().map(|p| p.single_day_flips).unwrap_or(0);

    // Q2: Build state distribution comparison string
    let mut q2_parts = Vec::new();
    for point in &points {
        let d = &point.distribution;
        q2_parts.push(format!(
            "{}d: RiskOn={}({:.1}%) Neutral={}({:.1}%) RiskOff={}({:.1}%)",
            point.confirmation_days,
            d.risk_on_days,
            d.risk_on_days as f64 / d.total_days as f64 * 100.0,
            d.neutral_days,
            d.neutral_days as f64 / d.total_days as f64 * 100.0,
            d.risk_off_days,
            d.risk_off_days as f64 / d.total_days as f64 * 100.0,
        ));
    }
    let q2_comparison = q2_parts.join("\n");

    // Q3: Delayed confirmation analysis (from 10d point)
    let q3_analysis = if let Some(p10) = points.iter().find(|p| p.confirmation_days == 10) {
        let swallowed_dur: usize = p10.episodes.iter().filter(|e| e.swallowed).map(|e| e.duration_days).sum();
        let total_dur: usize = p10.episodes.iter().map(|e| e.duration_days).sum();
        format!(
            "10d persistence: {} of {} episodes swallowed ({}% of days), {} merged, avg delay={:.1} days per episode, {} total swallowed-days",
            p10.swallowed_regimes,
            p10.episodes.len(),
            if total_dur > 0 { swallowed_dur as f64 / total_dur as f64 * 100.0 } else { 0.0 },
            p10.merged_regimes,
            p10.avg_delay_days,
            swallowed_dur
        )
    } else {
        "10d data not available".to_string()
    };

    let conclusion = if q1_flips == 0 {
        format!(
            "MECHANICS_EXPLAINED: 0 single-day flips detected. 1d persistence identical to 0d because implementation counts streak from 1, not 0. 2d+ cliff occurs because any regime shorter than confirmation_days gets swallowed entirely. {} total transitions in raw labels.",
            points.first().map(|p| p.total_transitions).unwrap_or(0)
        )
    } else {
        format!(
            "CHURN_PRESENT: {} single-day flips detected ({}% of all transitions). 1d persistence identical to 0d because implementation counts streak from 1, not 0. Single-day flips are swallowed at 2d+ but since 0d==1d, they are already visible in raw labels. The real damage comes from swallowing multi-day regimes.",
            q1_flips,
            if points.first().map(|p| p.total_transitions).unwrap_or(0) > 0 {
                q1_flips as f64 / points.first().map(|p| p.total_transitions).unwrap_or(1) as f64 * 100.0
            } else {
                0.0
            }
        )
    };

    Some(PersistenceMechanicsReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        points,
        q1_single_day_flip_count: q1_flips,
        q2_state_distribution_comparison: q2_comparison,
        q3_delayed_confirmation_analysis: q3_analysis,
        conclusion,
    })
}

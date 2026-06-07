use chrono::NaiveDate;
use core_domain::{
    DailyBar, LabelDistributionPoint, LabelDistributionReport, MarketRegimeSnapshot,
};
use std::collections::HashMap;

// ============================================================
// TASK-035A.0: Label Distribution Audit
// Baseline panel for Wave 8 Post-Persistence Revalidation.
// Compares label distributions across persistence levels (0d/1d/10d).
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

fn compute_distribution(labels: &[String]) -> (f64, f64, f64) {
    let mut risk_on = 0usize;
    let mut neutral = 0usize;
    let mut risk_off = 0usize;
    for label in labels {
        match label.as_str() {
            "risk_on" => risk_on += 1,
            "neutral" => neutral += 1,
            "risk_off" => risk_off += 1,
            _ => neutral += 1,
        }
    }
    let total = labels.len() as f64;
    (
        risk_on as f64 / total * 100.0,
        neutral as f64 / total * 100.0,
        risk_off as f64 / total * 100.0,
    )
}

fn compute_information(labels: &[String]) -> f64 {
    let total = labels.len() as f64;
    if total == 0.0 {
        return 0.0;
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for label in labels {
        *counts.entry(label.clone()).or_insert(0) += 1;
    }
    let mut entropy = 0.0;
    for (_, count) in counts {
        let p = count as f64 / total;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    let max_entropy = (3.0f64).log2();
    (entropy / max_entropy).clamp(0.0, 1.0)
}

fn extract_episodes(labels: &[String]) -> Vec<(String, usize)> {
    if labels.is_empty() {
        return Vec::new();
    }
    let mut episodes = Vec::new();
    let mut current = labels[0].clone();
    let mut count = 1;

    for i in 1..labels.len() {
        if labels[i] == current {
            count += 1;
        } else {
            episodes.push((current.clone(), count));
            current = labels[i].clone();
            count = 1;
        }
    }
    episodes.push((current, count));
    episodes
}

fn compute_episode_stats(episodes: &[(String, usize)]) -> (usize, f64, f64) {
    if episodes.is_empty() {
        return (0, 0.0, 0.0);
    }
    let durations: Vec<usize> = episodes.iter().map(|e| e.1).collect();
    let count = episodes.len();
    let avg = durations.iter().sum::<usize>() as f64 / count as f64;
    let mut sorted = durations.clone();
    sorted.sort_unstable();
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) as f64 / 2.0
    } else {
        sorted[sorted.len() / 2] as f64
    };
    (count, median, avg)
}

fn compute_alignment(
    labels: &[(NaiveDate, String)],
    close_by_date: &HashMap<NaiveDate, f64>,
) -> f64 {
    let mut dd20_tp = 0usize;
    let mut dd20_fp = 0usize;
    let mut dd20_fn = 0usize;
    let mut uptrend_tp = 0usize;
    let mut uptrend_fp = 0usize;
    let mut uptrend_fn = 0usize;

    let dates: Vec<NaiveDate> = labels.iter().map(|(d, _)| *d).collect();
    let mut closes: Vec<Option<f64>> = Vec::new();
    let mut ma20_vec: Vec<Option<f64>> = Vec::new();
    let mut ma60_vec: Vec<Option<f64>> = Vec::new();

    for (i, date) in dates.iter().enumerate() {
        closes.push(close_by_date.get(date).copied());
        if i >= 19 {
            let window: Vec<f64> = closes[i - 19..=i].iter().filter_map(|&c| c).collect();
            if window.len() == 20 {
                ma20_vec.push(Some(window.iter().sum::<f64>() / 20.0));
            } else {
                ma20_vec.push(None);
            }
        } else {
            ma20_vec.push(None);
        }
        if i >= 59 {
            let window: Vec<f64> = closes[i - 59..=i].iter().filter_map(|&c| c).collect();
            if window.len() == 60 {
                ma60_vec.push(Some(window.iter().sum::<f64>() / 60.0));
            } else {
                ma60_vec.push(None);
            }
        } else {
            ma60_vec.push(None);
        }
    }

    for (i, (_date, label)) in labels.iter().enumerate() {
        let is_riskoff = label.eq_ignore_ascii_case("risk_off");
        let is_riskon = label.eq_ignore_ascii_case("risk_on");

        let close = closes.get(i).copied().flatten().unwrap_or(0.0);
        let recent_high = closes[..=i].iter().filter_map(|&c| c).fold(0.0, f64::max);
        let dd = if recent_high > 0.0 {
            ((close - recent_high) / recent_high * 100.0).clamp(-100.0, 0.0)
        } else {
            0.0
        };
        let is_dd20 = dd < -20.0;

        let is_uptrend = if let (Some(m20), Some(m60)) = (ma20_vec[i], ma60_vec[i]) {
            close > m20 && m20 > m60
        } else {
            false
        };

        if is_riskoff {
            if is_dd20 {
                dd20_tp += 1;
            } else {
                dd20_fp += 1;
            }
        } else if is_dd20 {
            dd20_fn += 1;
        }

        if is_riskon {
            if is_uptrend {
                uptrend_tp += 1;
            } else {
                uptrend_fp += 1;
            }
        } else if is_uptrend {
            uptrend_fn += 1;
        }
    }

    let dd20_f1 = {
        let tp = dd20_tp;
        let fp = dd20_fp;
        let fn_ = dd20_fn;
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        }
    };

    let uptrend_f1 = {
        let tp = uptrend_tp;
        let fp = uptrend_fp;
        let fn_ = uptrend_fn;
        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        }
    };

    (dd20_f1 + uptrend_f1) / 2.0
}

fn compute_point(
    labels: &[String],
    dates: &[NaiveDate],
    close_by_date: &HashMap<NaiveDate, f64>,
    persistence_days: usize,
) -> LabelDistributionPoint {
    let persisted_labels = apply_persistence(labels, persistence_days);
    let (risk_on_pct, neutral_pct, risk_off_pct) = compute_distribution(&persisted_labels);

    let mut distinct_states = std::collections::HashSet::new();
    for label in &persisted_labels {
        distinct_states.insert(label.clone());
    }
    let effective_states = distinct_states.len();

    let information = compute_information(&persisted_labels);
    let episodes = extract_episodes(&persisted_labels);
    let (episode_count, median_episode, avg_episode) = compute_episode_stats(&episodes);
    let transition_count = if episodes.len() > 1 { episodes.len() - 1 } else { 0 };

    let labels_with_dates: Vec<(NaiveDate, String)> = dates
        .iter()
        .zip(persisted_labels.iter())
        .map(|(d, l)| (*d, l.clone()))
        .collect();
    let alignment = compute_alignment(&labels_with_dates, close_by_date);

    LabelDistributionPoint {
        persistence_days,
        risk_on_pct,
        neutral_pct,
        risk_off_pct,
        effective_states,
        information_score: information,
        episode_count,
        median_episode_days: median_episode,
        avg_episode_days: avg_episode,
        alignment_score: alignment,
        transition_count,
    }
}

pub fn compute_label_distribution(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<LabelDistributionReport> {
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
    let persistence_levels = vec![0, 1, 10];
    let mut points = Vec::new();

    for days in persistence_levels {
        let point = compute_point(&raw_labels, &dates, &close_by_date, days);
        points.push(point);
    }

    let conclusion = if let (Some(p0), Some(p1), Some(p10)) = (points.get(0), points.get(1), points.get(2)) {
        format!(
            "Wave8_BASELINE: 0d→1d→10d comparison. CN/HK state sequences are SHORT-LIVED (median {}d at 0d, {}d at 10d). 10d persistence swallows {}% of episodes, reducing effective states from {} to {}. This is the foundational panel for all Wave 8 revalidation.",
            p0.median_episode_days,
            p10.median_episode_days,
            if p0.episode_count > 0 { (p0.episode_count - p10.episode_count) as f64 / p0.episode_count as f64 * 100.0 } else { 0.0 },
            p0.effective_states,
            p10.effective_states
        )
    } else {
        "Baseline comparison incomplete".to_string()
    };

    Some(LabelDistributionReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        points,
        conclusion,
    })
}

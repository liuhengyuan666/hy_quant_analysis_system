use chrono::NaiveDate;
use core_domain::{
    DailyBar, EpisodeSurvivalBucket, EpisodeSurvivalPoint, EpisodeSurvivalReport,
    MarketRegimeSnapshot,
};
use std::collections::HashMap;

// ============================================================
// TASK-034C: Episode Survival Audit
// Measures raw episode length distribution to validate whether
// a given confirmation_days is reasonable relative to state
// persistence in the data.
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

fn percentile(sorted: &[usize], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    let idx = (p / 100.0) * (n - 1) as f64;
    let lower = idx.floor() as usize;
    let upper = idx.ceil() as usize;
    if lower == upper {
        sorted[lower] as f64
    } else {
        let frac = idx - lower as f64;
        sorted[lower] as f64 * (1.0 - frac) + sorted[upper] as f64 * frac
    }
}

pub fn compute_episode_survival(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<EpisodeSurvivalReport> {
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
    let episodes = extract_episodes(&raw_labels, &dates);

    if episodes.is_empty() {
        return None;
    }

    let durations: Vec<usize> = episodes.iter().map(|e| e.3).collect();
    let total_episodes = episodes.len();
    let avg = durations.iter().sum::<usize>() as f64 / total_episodes as f64;

    let mut sorted = durations.clone();
    sorted.sort_unstable();

    let median = percentile(&sorted, 50.0);
    let p25 = percentile(&sorted, 25.0);
    let p75 = percentile(&sorted, 75.0);
    let p95 = percentile(&sorted, 95.0);

    // Buckets
    let mut bucket_counts: HashMap<&str, usize> = HashMap::new();
    for d in &durations {
        let key = if *d == 1 {
            "1d"
        } else if *d <= 3 {
            "2-3d"
        } else if *d <= 7 {
            "4-7d"
        } else if *d <= 14 {
            "8-14d"
        } else if *d <= 30 {
            "15-30d"
        } else {
            "30d+"
        };
        *bucket_counts.entry(key).or_insert(0) += 1;
    }

    let bucket_order = ["1d", "2-3d", "4-7d", "8-14d", "15-30d", "30d+"];
    let mut buckets = Vec::new();
    for key in &bucket_order {
        if let Some(&count) = bucket_counts.get(key) {
            buckets.push(EpisodeSurvivalBucket {
                bucket_label: key.to_string(),
                count,
                percentage: count as f64 / total_episodes as f64 * 100.0,
            });
        }
    }

    // Survival curve: for each confirmation_days, what % of episodes survive?
    let survival_days = vec![1, 2, 3, 5, 7, 10, 15, 20, 30];
    let mut survival_curve = Vec::new();
    for days in &survival_days {
        let swallowed = durations.iter().filter(|&&d| d < *days).count();
        let survived = total_episodes - swallowed;
        survival_curve.push(EpisodeSurvivalPoint {
            confirmation_days: *days,
            survival_rate: survived as f64 / total_episodes as f64 * 100.0,
            swallowed_count: swallowed,
            survived_count: survived,
        });
    }

    // Recommendation
    let recommendation = if p95 < 10.0 {
        format!(
            "CRITICAL: Even 95th percentile episode length ({:.1}d) < 10d. confirmation_days=10 would swallow virtually all regimes. Recommend confirmation_days <= {} based on median ({:.1}d).",
            p95,
            std::cmp::max(1, median.floor() as usize / 2),
            median
        )
    } else if median < 10.0 {
        format!(
            "WARNING: Median episode length ({:.1}d) < 10d. confirmation_days=10 exceeds typical state lifetime. Recommend confirmation_days <= {} (roughly median/2).",
            median,
            std::cmp::max(1, median.floor() as usize / 2)
        )
    } else {
        format!(
            "ACCEPTABLE: Median episode length ({:.1}d) >= 10d. confirmation_days=10 is within typical state lifetime, but still swallows {}% of episodes. Consider shorter for responsiveness.",
            median,
            survival_curve.iter().find(|s| s.confirmation_days == 10).map(|s| 100.0 - s.survival_rate).unwrap_or(0.0)
        )
    };

    Some(EpisodeSurvivalReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        total_episodes,
        avg_episode_days: avg,
        median_episode_days: median,
        p25_episode_days: p25,
        p75_episode_days: p75,
        p95_episode_days: p95,
        buckets,
        survival_curve,
        recommendation,
    })
}

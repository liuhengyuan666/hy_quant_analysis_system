use chrono::NaiveDate;
use core_domain::{
    DailyBar, MarketRegimeSnapshot, ScoreDistribution, ScoreDistributionReport,
    ScoreHistogramBucket, ScoreThresholdHit,
};
use std::collections::HashMap;

// ============================================================
// TASK-035A.1: Score Distribution Audit
// Answers: Is HK RiskOn=0% due to threshold too high,
// or AND structure impossible?
// ============================================================

fn compute_histogram(scores: &[f64]) -> (Vec<ScoreHistogramBucket>, f64, f64, f64, f64, f64) {
    if scores.is_empty() {
        return (Vec::new(), 0.0, 0.0, 0.0, 0.0, 0.0);
    }

    let mut sorted = scores.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };
    let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
    let std = variance.sqrt();

    // Buckets: 0-10, 10-20, ..., 90-100
    let mut buckets = vec![0usize; 10];
    for &score in scores {
        let idx = (score / 10.0).floor() as usize;
        let idx = idx.min(9); // clamp 100 to bucket 9
        buckets[idx] += 1;
    }

    let total = scores.len() as f64;
    let bucket_labels = vec![
        "0-10", "10-20", "20-30", "30-40", "40-50",
        "50-60", "60-70", "70-80", "80-90", "90-100",
    ];

    let bucket_vec: Vec<ScoreHistogramBucket> = buckets
        .into_iter()
        .enumerate()
        .map(|(i, count)| ScoreHistogramBucket {
            range: bucket_labels[i].to_string(),
            count,
            percentage: count as f64 / total * 100.0,
        })
        .collect();

    (bucket_vec, mean, median, std, min, max)
}

pub fn compute_score_distribution(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
) -> Option<ScoreDistributionReport> {
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

    let trend_scores: Vec<f64> = regimes_filtered.iter().map(|r| r.trend_score).collect();
    let risk_scores: Vec<f64> = regimes_filtered.iter().map(|r| r.risk_score).collect();
    let liquidity_scores: Vec<f64> = regimes_filtered.iter().map(|r| r.liquidity_score).collect();

    let (trend_buckets, trend_mean, trend_median, trend_std, trend_min, trend_max) =
        compute_histogram(&trend_scores);
    let (risk_buckets, risk_mean, risk_median, risk_std, risk_min, risk_max) =
        compute_histogram(&risk_scores);
    let (liquidity_buckets, liquidity_mean, liquidity_median, liquidity_std, liquidity_min, liquidity_max) =
        compute_histogram(&liquidity_scores);

    // Threshold hit analysis
    let mut trend_ge_60 = 0usize;
    let mut liquidity_ge_50 = 0usize;
    let mut risk_ge_55 = 0usize;
    let mut risk_on_and = 0usize;
    let mut risk_off_or = 0usize;
    let mut risk_lt_40 = 0usize;
    let mut trend_lt_40 = 0usize;

    for i in 0..total_days {
        let t = trend_scores[i];
        let r = risk_scores[i];
        let l = liquidity_scores[i];

        if t >= 60.0 {
            trend_ge_60 += 1;
        }
        if l >= 50.0 {
            liquidity_ge_50 += 1;
        }
        if r >= 55.0 {
            risk_ge_55 += 1;
        }
        if t >= 60.0 && l >= 50.0 && r >= 55.0 {
            risk_on_and += 1;
        }
        if r < 40.0 {
            risk_lt_40 += 1;
        }
        if t < 40.0 {
            trend_lt_40 += 1;
        }
        if r < 40.0 || t < 40.0 {
            risk_off_or += 1;
        }
    }

    let total = total_days as f64;
    let threshold_hits = vec![
        ScoreThresholdHit {
            condition: "trend >= 60".to_string(),
            days_met: trend_ge_60,
            percentage: trend_ge_60 as f64 / total * 100.0,
        },
        ScoreThresholdHit {
            condition: "liquidity >= 50".to_string(),
            days_met: liquidity_ge_50,
            percentage: liquidity_ge_50 as f64 / total * 100.0,
        },
        ScoreThresholdHit {
            condition: "risk >= 55".to_string(),
            days_met: risk_ge_55,
            percentage: risk_ge_55 as f64 / total * 100.0,
        },
        ScoreThresholdHit {
            condition: "RiskOn: AND(all three)".to_string(),
            days_met: risk_on_and,
            percentage: risk_on_and as f64 / total * 100.0,
        },
        ScoreThresholdHit {
            condition: "risk < 40".to_string(),
            days_met: risk_lt_40,
            percentage: risk_lt_40 as f64 / total * 100.0,
        },
        ScoreThresholdHit {
            condition: "trend < 40".to_string(),
            days_met: trend_lt_40,
            percentage: trend_lt_40 as f64 / total * 100.0,
        },
        ScoreThresholdHit {
            condition: "RiskOff: OR(risk<40, trend<40)".to_string(),
            days_met: risk_off_or,
            percentage: risk_off_or as f64 / total * 100.0,
        },
    ];

    let conclusion = if risk_on_and == 0 {
        format!(
            "CRITICAL: RiskOn AND condition NEVER met (0/{} days). Individual rates: trend>=60={:.1}%, liquidity>=50={:.1}%, risk>=55={:.1}%. The AND structure makes RiskOn impossible in this market. This is a THRESHOLD DESIGN problem, not a persistence problem.",
            total_days,
            trend_ge_60 as f64 / total * 100.0,
            liquidity_ge_50 as f64 / total * 100.0,
            risk_ge_55 as f64 / total * 100.0,
        )
    } else if risk_on_and as f64 / total * 100.0 < 5.0 {
        format!(
            "WARNING: RiskOn AND condition rarely met ({:.1}% of days). Individual rates: trend>=60={:.1}%, liquidity>=50={:.1}%, risk>=55={:.1}%. The AND structure severely restricts RiskOn occurrence. Consider whether AND is appropriate for this market.",
            risk_on_and as f64 / total * 100.0,
            trend_ge_60 as f64 / total * 100.0,
            liquidity_ge_50 as f64 / total * 100.0,
            risk_ge_55 as f64 / total * 100.0,
        )
    } else {
        format!(
            "ACCEPTABLE: RiskOn AND condition met {:.1}% of days. Individual rates: trend>=60={:.1}%, liquidity>=50={:.1}%, risk>=55={:.1}%.",
            risk_on_and as f64 / total * 100.0,
            trend_ge_60 as f64 / total * 100.0,
            liquidity_ge_50 as f64 / total * 100.0,
            risk_ge_55 as f64 / total * 100.0,
        )
    };

    Some(ScoreDistributionReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        trend_distribution: ScoreDistribution {
            metric: "trend_score".to_string(),
            mean: trend_mean,
            median: trend_median,
            std: trend_std,
            min: trend_min,
            max: trend_max,
            buckets: trend_buckets,
        },
        risk_distribution: ScoreDistribution {
            metric: "risk_score".to_string(),
            mean: risk_mean,
            median: risk_median,
            std: risk_std,
            min: risk_min,
            max: risk_max,
            buckets: risk_buckets,
        },
        liquidity_distribution: ScoreDistribution {
            metric: "liquidity_score".to_string(),
            mean: liquidity_mean,
            median: liquidity_median,
            std: liquidity_std,
            min: liquidity_min,
            max: liquidity_max,
            buckets: liquidity_buckets,
        },
        threshold_hits,
        conclusion,
    })
}

use crate::common::apply_persistence;
use chrono::NaiveDate;
use core_domain::{
    DailyBar, GroundTruthClassMetrics, GroundTruthConfusionCell, GroundTruthDistribution,
    GroundTruthReport, MarketRegimeSnapshot,
};
use std::collections::HashMap;

// ============================================================
// TASK-035B: Ground Truth Audit
// Investigates why Alignment is low but returns are high.
// Examines the Ground Truth definitions used for Alignment.
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

fn compute_ground_truth_labels(
    dates: &[NaiveDate],
    close_by_date: &HashMap<NaiveDate, f64>,
) -> Vec<String> {
    let mut labels = Vec::new();
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

    for i in 0..dates.len() {
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

        // Ground truth definition:
        // RiskOff: drawdown > 20%
        // RiskOn: close > ma20 && ma20 > ma60
        // Neutral: neither
        if is_dd20 {
            labels.push("risk_off".to_string());
        } else if is_uptrend {
            labels.push("risk_on".to_string());
        } else {
            labels.push("neutral".to_string());
        }
    }

    labels
}

fn compute_distribution(labels: &[String]) -> Vec<GroundTruthDistribution> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for label in labels {
        *counts.entry(label.clone()).or_insert(0) += 1;
    }
    let total = labels.len() as f64;
    let mut dist = Vec::new();
    for (label, count) in counts {
        dist.push(GroundTruthDistribution {
            label,
            count,
            percentage: count as f64 / total * 100.0,
        });
    }
    dist.sort_by(|a, b| b.count.cmp(&a.count));
    dist
}

fn compute_confusion(
    predicted: &[String],
    actual: &[String],
) -> (Vec<GroundTruthConfusionCell>, Vec<GroundTruthClassMetrics>, f64, f64) {
    let classes = vec!["risk_on", "neutral", "risk_off"];
    let mut matrix: HashMap<(String, String), usize> = HashMap::new();

    for i in 0..predicted.len().min(actual.len()) {
        let key = (predicted[i].clone(), actual[i].clone());
        *matrix.entry(key).or_insert(0) += 1;
    }

    let mut confusion_cells = Vec::new();
    let total = predicted.len() as f64;
    for pred in &classes {
        for act in &classes {
            let count = matrix.get(&(pred.to_string(), act.to_string())).copied().unwrap_or(0);
            confusion_cells.push(GroundTruthConfusionCell {
                predicted: pred.to_string(),
                actual: act.to_string(),
                count,
                percentage: count as f64 / total * 100.0,
            });
        }
    }

    let mut class_metrics = Vec::new();
    let mut macro_f1 = 0.0;
    let mut correct = 0usize;

    for class in &classes {
        let tp = matrix.get(&(class.to_string(), class.to_string())).copied().unwrap_or(0);
        let fp: usize = classes
            .iter()
            .filter(|&c| c != class)
            .map(|c| matrix.get(&(class.to_string(), c.to_string())).copied().unwrap_or(0))
            .sum();
        let fn_: usize = classes
            .iter()
            .filter(|&c| c != class)
            .map(|c| matrix.get(&(c.to_string(), class.to_string())).copied().unwrap_or(0))
            .sum();
        let support = tp + fn_;

        let precision = if tp + fp > 0 {
            tp as f64 / (tp + fp) as f64
        } else {
            0.0
        };
        let recall = if tp + fn_ > 0 {
            tp as f64 / (tp + fn_) as f64
        } else {
            0.0
        };
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        macro_f1 += f1;
        correct += tp;

        class_metrics.push(GroundTruthClassMetrics {
            class: class.to_string(),
            precision,
            recall,
            f1,
            support,
        });
    }

    macro_f1 /= classes.len() as f64;
    let accuracy = if !predicted.is_empty() {
        correct as f64 / predicted.len() as f64
    } else {
        0.0
    };

    (confusion_cells, class_metrics, accuracy, macro_f1)
}

pub fn compute_ground_truth_audit(
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
    scope_str: &str,
    anchor_symbol: &str,
    persistence_days: usize,
) -> Option<GroundTruthReport> {
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

    let predicted_labels = apply_persistence(&raw_labels, persistence_days);
    let dates: Vec<NaiveDate> = regimes_filtered.iter().map(|r| r.date).collect();
    let actual_labels = compute_ground_truth_labels(&dates, &close_by_date);

    let predicted_dist = compute_distribution(&predicted_labels);
    let actual_dist = compute_distribution(&actual_labels);
    let (confusion, class_metrics, accuracy, macro_f1) = compute_confusion(&predicted_labels, &actual_labels);

    // Find the class with biggest precision-recall gap
    let mut max_gap_class = "";
    let mut max_gap = 0.0;
    for m in &class_metrics {
        let gap = (m.precision - m.recall).abs();
        if gap > max_gap {
            max_gap = gap;
            max_gap_class = &m.class;
        }
    }

    let conclusion = format!(
        "GROUND_TRUTH_AUDIT: {}@{}d. Predicted vs Actual distributions differ significantly? {}. Overall accuracy={:.1}%, macro_f1={:.3}. Biggest gap in '{}': precision={:.2}, recall={:.2}. If returns are high but alignment is low, the regime may be capturing valuable states NOT covered by the ground truth definition (dd20 + uptrend).",
        scope_str,
        persistence_days,
        if predicted_dist != actual_dist { "YES" } else { "NO" },
        accuracy * 100.0,
        macro_f1,
        max_gap_class,
        class_metrics.iter().find(|m| m.class == max_gap_class).map(|m| m.precision).unwrap_or(0.0),
        class_metrics.iter().find(|m| m.class == max_gap_class).map(|m| m.recall).unwrap_or(0.0),
    );

    Some(GroundTruthReport {
        scope: scope_str.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days,
        predicted_distribution: predicted_dist,
        actual_distribution: actual_dist,
        confusion_matrix: confusion,
        class_metrics,
        overall_accuracy: accuracy,
        macro_f1,
        conclusion,
    })
}

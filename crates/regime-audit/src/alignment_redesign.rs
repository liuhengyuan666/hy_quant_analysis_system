use chrono::NaiveDate;
use core_domain::MarketRegimeSnapshot;
use std::collections::HashMap;

// ============================================================
// TASK-060C: Alignment Redesign
// Compares regime predictions against multiple Ground Truth definitions.
// Computes: Accuracy, Macro F1, Per-class Precision/Recall, Information Score
// ============================================================

#[derive(Debug, Clone)]
pub struct ClassMetrics {
    pub class: String,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub support: usize,
}

#[derive(Debug, Clone)]
pub struct AlignmentReport {
    pub market: String,
    pub gt_scheme: String,
    pub total_samples: usize,
    pub accuracy: f64,
    pub macro_precision: f64,
    pub macro_recall: f64,
    pub macro_f1: f64,
    pub information_score: f64,
    pub class_metrics: Vec<ClassMetrics>,
    pub confusion_matrix: Vec<ConfusionCell>,
}

#[derive(Debug, Clone)]
pub struct ConfusionCell {
    pub predicted: String,
    pub actual: String,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone)]
pub struct AlignmentComparisonReport {
    pub cn_reports: Vec<AlignmentReport>,
    pub hk_reports: Vec<AlignmentReport>,
    pub old_technical_gt: Vec<AlignmentReport>,
}

fn compute_information_score(predictions: &[String], ground_truth: &[String]) -> f64 {
    if predictions.len() != ground_truth.len() || predictions.is_empty() {
        return 0.0;
    }

    let n = predictions.len() as f64;

    // Compute joint distribution
    let mut joint_counts: HashMap<(String, String), usize> = HashMap::new();
    let mut pred_counts: HashMap<String, usize> = HashMap::new();
    let mut gt_counts: HashMap<String, usize> = HashMap::new();

    for (pred, gt) in predictions.iter().zip(ground_truth.iter()) {
        *joint_counts.entry((pred.clone(), gt.clone())).or_insert(0) += 1;
        *pred_counts.entry(pred.clone()).or_insert(0) += 1;
        *gt_counts.entry(gt.clone()).or_insert(0) += 1;
    }

    // Mutual information
    let mut mi = 0.0;
    for ((pred, gt), count) in &joint_counts {
        let p_xy = *count as f64 / n;
        let p_x = *pred_counts.get(pred).unwrap() as f64 / n;
        let p_y = *gt_counts.get(gt).unwrap() as f64 / n;
        if p_x > 0.0 && p_y > 0.0 && p_xy > 0.0 {
            mi += p_xy * (p_xy / (p_x * p_y)).ln();
        }
    }

    // Entropy of ground truth
    let mut gt_entropy = 0.0;
    for count in gt_counts.values() {
        let p = *count as f64 / n;
        if p > 0.0 {
            gt_entropy -= p * p.ln();
        }
    }

    if gt_entropy > 0.0 {
        (mi / gt_entropy).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn compute_alignment(
    market: &str,
    gt_scheme: &str,
    predictions: &[String],
    ground_truth: &[String],
) -> AlignmentReport {
    assert_eq!(predictions.len(), ground_truth.len());
    let n = predictions.len();

    if n == 0 {
        return AlignmentReport {
            market: market.to_string(),
            gt_scheme: gt_scheme.to_string(),
            total_samples: 0,
            accuracy: 0.0,
            macro_precision: 0.0,
            macro_recall: 0.0,
            macro_f1: 0.0,
            information_score: 0.0,
            class_metrics: Vec::new(),
            confusion_matrix: Vec::new(),
        };
    }

    // Overall accuracy
    let correct = predictions.iter().zip(ground_truth.iter()).filter(|(p, g)| p == g).count();
    let accuracy = correct as f64 / n as f64;

    // Per-class metrics
    let classes = vec!["risk_off", "neutral", "risk_on"];
    let mut class_metrics = Vec::new();
    let mut macro_precision = 0.0;
    let mut macro_recall = 0.0;
    let mut macro_f1 = 0.0;
    let mut valid_classes = 0;

    for class in &classes {
        let tp = predictions.iter().zip(ground_truth.iter()).filter(|(p, g)| p == class && g == class).count();
        let fp = predictions.iter().zip(ground_truth.iter()).filter(|(p, g)| p == class && g != class).count();
        let fn_ = predictions.iter().zip(ground_truth.iter()).filter(|(p, g)| p != class && g == class).count();
        let support = ground_truth.iter().filter(|g| g == class).count();

        let precision = if tp + fp > 0 { tp as f64 / (tp + fp) as f64 } else { 0.0 };
        let recall = if tp + fn_ > 0 { tp as f64 / (tp + fn_) as f64 } else { 0.0 };
        let f1 = if precision + recall > 0.0 { 2.0 * precision * recall / (precision + recall) } else { 0.0 };

        if support > 0 {
            macro_precision += precision;
            macro_recall += recall;
            macro_f1 += f1;
            valid_classes += 1;
        }

        class_metrics.push(ClassMetrics {
            class: class.to_string(),
            precision,
            recall,
            f1,
            support,
        });
    }

    if valid_classes > 0 {
        macro_precision /= valid_classes as f64;
        macro_recall /= valid_classes as f64;
        macro_f1 /= valid_classes as f64;
    }

    // Confusion matrix
    let mut confusion_counts: HashMap<(String, String), usize> = HashMap::new();
    for (pred, gt) in predictions.iter().zip(ground_truth.iter()) {
        *confusion_counts.entry((pred.clone(), gt.clone())).or_insert(0) += 1;
    }

    let mut confusion_matrix = Vec::new();
    for ((pred, gt), count) in &confusion_counts {
        confusion_matrix.push(ConfusionCell {
            predicted: pred.clone(),
            actual: gt.clone(),
            count: *count,
            percentage: (*count as f64 / n as f64) * 100.0,
        });
    }

    // Information score
    let information_score = compute_information_score(predictions, ground_truth);

    AlignmentReport {
        market: market.to_string(),
        gt_scheme: gt_scheme.to_string(),
        total_samples: n,
        accuracy,
        macro_precision,
        macro_recall,
        macro_f1,
        information_score,
        class_metrics,
        confusion_matrix,
    }
}

pub fn compute_forward_return_alignment(
    market: &str,
    regimes: &[MarketRegimeSnapshot],
    bars: &[core_domain::DailyBar],
    horizon_days: usize,
    schemes: &[(String, f64, f64)], // (name, risk_off_pct, risk_on_pct)
) -> Vec<AlignmentReport> {
    // Compute forward returns
    let n = bars.len();
    if n <= horizon_days {
        return Vec::new();
    }

    let mut returns_by_date: HashMap<NaiveDate, f64> = HashMap::new();
    for i in 0..n - horizon_days {
        let current = bars[i].close;
        let future = bars[i + horizon_days].close;
        let ret = (future - current) / current;
        returns_by_date.insert(bars[i].date, ret);
    }

    // Build predictions map from regimes
    let mut predictions_by_date: HashMap<NaiveDate, String> = HashMap::new();
    for regime in regimes {
        predictions_by_date.insert(regime.date, regime.regime_label.clone());
    }

    // Find common dates
    let common_dates: Vec<NaiveDate> = returns_by_date
        .keys()
        .filter(|d| predictions_by_date.contains_key(d))
        .copied()
        .collect();

    if common_dates.is_empty() {
        return Vec::new();
    }

    let mut reports = Vec::new();

    for (scheme_name, risk_off_pct, risk_on_pct) in schemes {
        // Extract returns for common dates and sort
        let mut returns: Vec<(NaiveDate, f64)> = common_dates
            .iter()
            .map(|d| (*d, *returns_by_date.get(d).unwrap()))
            .collect();
        returns.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let n_returns = returns.len();
        let risk_off_idx = ((n_returns as f64 * *risk_off_pct).ceil() as usize).clamp(1, n_returns - 1);
        let risk_on_idx = ((n_returns as f64 * *risk_on_pct).floor() as usize).clamp(1, n_returns - 1);
        let risk_off_threshold = returns[risk_off_idx - 1].1;
        let risk_on_threshold = returns[risk_on_idx].1;

        // Build ground truth labels
        let mut ground_truth: Vec<String> = Vec::new();
        let mut predictions: Vec<String> = Vec::new();

        for date in &common_dates {
            let ret = returns_by_date.get(date).unwrap();
            let gt_label = if *ret <= risk_off_threshold {
                "risk_off".to_string()
            } else if *ret >= risk_on_threshold {
                "risk_on".to_string()
            } else {
                "neutral".to_string()
            };

            if let Some(pred) = predictions_by_date.get(date) {
                ground_truth.push(gt_label);
                predictions.push(pred.clone());
            }
        }

        let report = compute_alignment(market, scheme_name, &predictions, &ground_truth);
        reports.push(report);
    }

    reports
}

pub fn compute_technical_ground_truth_alignment(
    market: &str,
    regimes: &[MarketRegimeSnapshot],
    bars: &[core_domain::DailyBar],
) -> AlignmentReport {
    // Old technical GT: RiskOff = drawdown > 20%, RiskOn = close > MA20 && MA20 > MA60
    let mut predictions: Vec<String> = Vec::new();
    let mut ground_truth: Vec<String> = Vec::new();

    let close_by_date: HashMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();

    for regime in regimes {
        if let Some(close) = close_by_date.get(&regime.date) {
            // Compute MA20 and MA60
            let dates: Vec<NaiveDate> = bars.iter().map(|b| b.date).collect();
            let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();

            if let Some(idx) = dates.iter().position(|d| d == &regime.date) {
                let ma20 = if idx >= 19 {
                    let sum: f64 = closes[idx - 19..=idx].iter().sum();
                    sum / 20.0
                } else {
                    *close
                };

                let ma60 = if idx >= 59 {
                    let sum: f64 = closes[idx - 59..=idx].iter().sum();
                    sum / 60.0
                } else {
                    *close
                };

                // Compute drawdown from recent high
                let recent_high = if idx > 0 {
                    closes[0..=idx].iter().copied().fold(f64::NEG_INFINITY, f64::max)
                } else {
                    *close
                };
                let drawdown = (close - recent_high) / recent_high;

                let gt_label = if drawdown < -0.20 {
                    "risk_off".to_string()
                } else if *close > ma20 && ma20 > ma60 {
                    "risk_on".to_string()
                } else {
                    "neutral".to_string()
                };

                predictions.push(regime.regime_label.clone());
                ground_truth.push(gt_label);
            }
        }
    }

    compute_alignment(market, "Technical-GT", &predictions, &ground_truth)
}

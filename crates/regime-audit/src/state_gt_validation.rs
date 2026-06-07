use chrono::NaiveDate;
use core_domain::{DailyBar, MacroSnapshot, MarketRegimeSnapshot};
use std::collections::{BTreeMap, HashMap};

// ============================================================
// TASK-071A: State Layer Ground Truth Demonstration
// Computes new State GT based on current market conditions
// and evaluates regime predictions against it.
// ============================================================

fn rolling_mean(values: &[f64], index: usize, period: usize) -> Option<f64> {
    if index + 1 < period {
        return None;
    }
    let window = &values[index + 1 - period..=index];
    Some(window.iter().sum::<f64>() / period as f64)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}

#[derive(Debug, Clone)]
pub struct StateGtReport {
    pub market: String,
    pub total_samples: usize,
    pub accuracy: f64,
    pub macro_f1: f64,
    pub macro_precision: f64,
    pub macro_recall: f64,
    pub information_score: f64,
    pub class_metrics: Vec<ClassMetrics>,
    pub gt_distribution: (usize, usize, usize), // (risk_off, neutral, risk_on)
}

#[derive(Debug, Clone)]
pub struct ClassMetrics {
    pub class: String,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub support: usize,
}

pub struct AlignmentComparison {
    pub old_technical: Option<StateGtReport>,
    pub new_state_gt: StateGtReport,
}

fn compute_information_score(predictions: &[String], ground_truth: &[String]) -> f64 {
    if predictions.len() != ground_truth.len() || predictions.is_empty() {
        return 0.0;
    }

    let n = predictions.len() as f64;
    let mut joint_counts: HashMap<(String, String), usize> = HashMap::new();
    let mut pred_counts: HashMap<String, usize> = HashMap::new();
    let mut gt_counts: HashMap<String, usize> = HashMap::new();

    for (pred, gt) in predictions.iter().zip(ground_truth.iter()) {
        *joint_counts.entry((pred.clone(), gt.clone())).or_insert(0) += 1;
        *pred_counts.entry(pred.clone()).or_insert(0) += 1;
        *gt_counts.entry(gt.clone()).or_insert(0) += 1;
    }

    let mut mi = 0.0;
    for ((pred, gt), count) in &joint_counts {
        let p_xy = *count as f64 / n;
        let p_x = *pred_counts.get(pred).unwrap() as f64 / n;
        let p_y = *gt_counts.get(gt).unwrap() as f64 / n;
        if p_x > 0.0 && p_y > 0.0 && p_xy > 0.0 {
            mi += p_xy * (p_xy / (p_x * p_y)).ln();
        }
    }

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

fn compute_metrics(
    market: &str,
    predictions: &[String],
    ground_truth: &[String],
) -> StateGtReport {
    assert_eq!(predictions.len(), ground_truth.len());
    let n = predictions.len();

    let correct = predictions.iter().zip(ground_truth.iter()).filter(|(p, g)| p == g).count();
    let accuracy = if n > 0 { correct as f64 / n as f64 } else { 0.0 };

    let classes = vec!["risk_off", "neutral", "risk_on"];
    let mut class_metrics = Vec::new();
    let mut macro_precision = 0.0;
    let mut macro_recall = 0.0;
    let mut macro_f1 = 0.0;
    let mut valid_classes = 0;

    let mut risk_off_count = 0;
    let mut neutral_count = 0;
    let mut risk_on_count = 0;

    for gt in ground_truth {
        match gt.as_str() {
            "risk_off" => risk_off_count += 1,
            "neutral" => neutral_count += 1,
            "risk_on" => risk_on_count += 1,
            _ => {}
        }
    }

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

    let information_score = compute_information_score(predictions, ground_truth);

    StateGtReport {
        market: market.to_string(),
        total_samples: n,
        accuracy,
        macro_f1,
        macro_precision,
        macro_recall,
        information_score,
        class_metrics,
        gt_distribution: (risk_off_count, neutral_count, risk_on_count),
    }
}

pub fn compute_state_layer_gt_alignment(
    market: &str,
    regimes: &[MarketRegimeSnapshot],
    macro_snapshots: &[MacroSnapshot],
    bars: &[DailyBar],
) -> AlignmentComparison {
    // Build macro scores by date
    let mut vix_by_date: BTreeMap<NaiveDate, f64> = BTreeMap::new();
    let mut dollar_by_date: BTreeMap<NaiveDate, f64> = BTreeMap::new();

    for snap in macro_snapshots {
        match snap.factor_name.as_str() {
            "vix" => { vix_by_date.insert(snap.date, snap.factor_score); }
            "dollar_index" => { dollar_by_date.insert(snap.date, snap.factor_score); }
            _ => {}
        }
    }

    // Compute VIX and Dollar percentiles
    let vix_values: Vec<f64> = vix_by_date.values().copied().collect();
    let dollar_values: Vec<f64> = dollar_by_date.values().copied().collect();

    let mut vix_sorted = vix_values.clone();
    let mut dollar_sorted = dollar_values.clone();
    vix_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    dollar_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let vix_p75 = percentile(&vix_sorted, 0.75);
    let vix_p50 = percentile(&vix_sorted, 0.50);
    let dollar_p75 = percentile(&dollar_sorted, 0.75);
    let dollar_p50 = percentile(&dollar_sorted, 0.50);

    // Build bar data
    let close_by_date: BTreeMap<NaiveDate, f64> = bars.iter().map(|b| (b.date, b.close)).collect();
    let dates: Vec<NaiveDate> = bars.iter().map(|b| b.date).collect();
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();

    // Build new State GT
    let mut predictions: Vec<String> = Vec::new();
    let mut new_gt: Vec<String> = Vec::new();
    let mut old_gt: Vec<String> = Vec::new();

    for regime in regimes {
        let date = regime.date;

        // Get VIX and Dollar scores
        let vix = vix_by_date.get(&date).copied().unwrap_or(50.0);
        let dollar = dollar_by_date.get(&date).copied().unwrap_or(50.0);

        // Get trend data
        let mut trend_gt = "neutral";
        let mut old_label = "neutral";
        if let Some(idx) = dates.iter().position(|d| d == &date) {
            let ma20 = rolling_mean(&closes, idx, 20);
            let ma60 = rolling_mean(&closes, idx, 60);
            let close = close_by_date.get(&date).copied().unwrap_or(0.0);

            // New State GT logic
            let is_risk_off = vix > vix_p75 || dollar > dollar_p75 || (ma60.is_some() && close < ma60.unwrap());
            let is_risk_on = ma20.is_some() && close > ma20.unwrap() && vix < vix_p50 && dollar < dollar_p50;

            if is_risk_off {
                trend_gt = "risk_off";
            } else if is_risk_on {
                trend_gt = "risk_on";
            } else {
                trend_gt = "neutral";
            }

            // Old Technical GT logic
            let ma20_val = ma20.unwrap_or(close);
            let ma60_val = ma60.unwrap_or(close);
            let recent_high = if idx > 0 {
                closes[0..=idx].iter().copied().fold(f64::NEG_INFINITY, f64::max)
            } else {
                close
            };
            let drawdown = (close - recent_high) / recent_high;

            old_label = if drawdown < -0.20 {
                "risk_off"
            } else if close > ma20_val && ma20_val > ma60_val {
                "risk_on"
            } else {
                "neutral"
            };
        }

        old_gt.push(old_label.to_string());
        new_gt.push(trend_gt.to_string());
        predictions.push(regime.regime_label.clone());
    }

    let new_report = compute_metrics(market, &predictions, &new_gt);

    let old_report = if old_gt.len() == predictions.len() {
        Some(compute_metrics(market, &predictions, &old_gt))
    } else {
        None
    };

    AlignmentComparison {
        old_technical: old_report,
        new_state_gt: new_report,
    }
}

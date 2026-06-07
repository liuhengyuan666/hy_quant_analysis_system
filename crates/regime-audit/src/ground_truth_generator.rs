use chrono::NaiveDate;
use core_domain::DailyBar;
use std::collections::HashMap;

// ============================================================
// TASK-060B: Ground Truth Label Generator
// Generates 3 sets of Ground Truth labels based on forward return percentiles.
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum GroundTruthLabel {
    RiskOff,
    Neutral,
    RiskOn,
}

impl GroundTruthLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            GroundTruthLabel::RiskOff => "risk_off",
            GroundTruthLabel::Neutral => "neutral",
            GroundTruthLabel::RiskOn => "risk_on",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroundTruthScheme {
    pub name: String,
    pub risk_off_pct: f64,  // bottom percentile (e.g., 0.25 = bottom 25%)
    pub risk_on_pct: f64,   // top percentile (e.g., 0.75 = top 25%)
}

#[derive(Debug, Clone)]
pub struct LabeledDate {
    pub date: NaiveDate,
    pub forward_return: f64,
    pub label: GroundTruthLabel,
}

#[derive(Debug, Clone)]
pub struct GroundTruthSet {
    pub market: String,
    pub scheme: GroundTruthScheme,
    pub horizon_days: usize,
    pub labels: Vec<LabeledDate>,
}

#[derive(Debug, Clone)]
pub struct GroundTruthGenerationReport {
    pub cn_sets: Vec<GroundTruthSet>,
    pub hk_sets: Vec<GroundTruthSet>,
}

fn compute_forward_returns(bars: &[DailyBar], horizon: usize) -> Vec<(NaiveDate, f64)> {
    let n = bars.len();
    if n <= horizon {
        return Vec::new();
    }

    let mut result = Vec::with_capacity(n - horizon);
    for i in 0..n - horizon {
        let current = bars[i].close;
        let future = bars[i + horizon].close;
        let ret = (future - current) / current;
        result.push((bars[i].date, ret));
    }
    result
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}

fn generate_labels(
    market: &str,
    returns: &[(NaiveDate, f64)],
    scheme: &GroundTruthScheme,
    horizon_days: usize,
) -> GroundTruthSet {
    let mut sorted_returns: Vec<f64> = returns.iter().map(|(_, r)| *r).collect();
    sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let risk_off_threshold = percentile(&sorted_returns, scheme.risk_off_pct);
    let risk_on_threshold = percentile(&sorted_returns, scheme.risk_on_pct);

    let labels: Vec<LabeledDate> = returns
        .iter()
        .map(|(date, ret)| {
            let label = if *ret <= risk_off_threshold {
                GroundTruthLabel::RiskOff
            } else if *ret >= risk_on_threshold {
                GroundTruthLabel::RiskOn
            } else {
                GroundTruthLabel::Neutral
            };
            LabeledDate {
                date: *date,
                forward_return: *ret,
                label,
            }
        })
        .collect();

    GroundTruthSet {
        market: market.to_string(),
        scheme: scheme.clone(),
        horizon_days,
        labels,
    }
}

pub fn generate_ground_truth_labels(
    cn_bars: &[DailyBar],
    hk_bars: &[DailyBar],
    horizon_days: usize,
) -> GroundTruthGenerationReport {
    let schemes = vec![
        GroundTruthScheme {
            name: "GT-25".to_string(),
            risk_off_pct: 0.25,
            risk_on_pct: 0.75,
        },
        GroundTruthScheme {
            name: "GT-33".to_string(),
            risk_off_pct: 0.33,
            risk_on_pct: 0.67,
        },
        GroundTruthScheme {
            name: "GT-10".to_string(),
            risk_off_pct: 0.10,
            risk_on_pct: 0.90,
        },
    ];

    let cn_returns = compute_forward_returns(cn_bars, horizon_days);
    let hk_returns = compute_forward_returns(hk_bars, horizon_days);

    let mut cn_sets = Vec::new();
    let mut hk_sets = Vec::new();

    for scheme in &schemes {
        if !cn_returns.is_empty() {
            cn_sets.push(generate_labels("CN", &cn_returns, scheme, horizon_days));
        }
        if !hk_returns.is_empty() {
            hk_sets.push(generate_labels("HK", &hk_returns, scheme, horizon_days));
        }
    }

    GroundTruthGenerationReport {
        cn_sets,
        hk_sets,
    }
}

pub fn compute_label_distribution(labels: &[LabeledDate]) -> (usize, usize, usize) {
    let mut risk_off = 0;
    let mut neutral = 0;
    let mut risk_on = 0;
    for ld in labels {
        match ld.label {
            GroundTruthLabel::RiskOff => risk_off += 1,
            GroundTruthLabel::Neutral => neutral += 1,
            GroundTruthLabel::RiskOn => risk_on += 1,
        }
    }
    (risk_off, neutral, risk_on)
}

pub fn compute_mean_return_by_label(labels: &[LabeledDate]) -> (f64, f64, f64) {
    let mut risk_off_sum = 0.0;
    let mut risk_off_count = 0;
    let mut neutral_sum = 0.0;
    let mut neutral_count = 0;
    let mut risk_on_sum = 0.0;
    let mut risk_on_count = 0;

    for ld in labels {
        match ld.label {
            GroundTruthLabel::RiskOff => {
                risk_off_sum += ld.forward_return;
                risk_off_count += 1;
            }
            GroundTruthLabel::Neutral => {
                neutral_sum += ld.forward_return;
                neutral_count += 1;
            }
            GroundTruthLabel::RiskOn => {
                risk_on_sum += ld.forward_return;
                risk_on_count += 1;
            }
        }
    }

    let risk_off_mean = if risk_off_count > 0 { risk_off_sum / risk_off_count as f64 } else { 0.0 };
    let neutral_mean = if neutral_count > 0 { neutral_sum / neutral_count as f64 } else { 0.0 };
    let risk_on_mean = if risk_on_count > 0 { risk_on_sum / risk_on_count as f64 } else { 0.0 };

    (risk_off_mean, neutral_mean, risk_on_mean)
}

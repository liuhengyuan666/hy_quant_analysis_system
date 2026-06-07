use chrono::NaiveDate;
use core_domain::DailyBar;

// ============================================================
// TASK-060A.1: Forward Return Distribution Audit
// Computes forward return distributions for CN/HK at multiple horizons.
// Output: percentile statistics (P1, P5, P10, P25, P50, P75, P90, P95, Mean, Std)
// ============================================================

#[derive(Debug, Clone)]
pub struct ForwardReturnDistribution {
    pub market: String,
    pub horizon_days: usize,
    pub sample_count: usize,
    pub p01: f64,
    pub p05: f64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub mean: f64,
    pub std: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone)]
pub struct DistributionAuditReport {
    pub cn_distributions: Vec<ForwardReturnDistribution>,
    pub hk_distributions: Vec<ForwardReturnDistribution>,
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

fn compute_distribution(
    market: &str,
    horizon_days: usize,
    returns: &[(NaiveDate, f64)],
) -> ForwardReturnDistribution {
    let mut rets: Vec<f64> = returns.iter().map(|(_, r)| *r).collect();
    rets.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = rets.len();
    let mean = rets.iter().sum::<f64>() / n as f64;
    let variance = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1).max(1) as f64;
    let std = variance.sqrt();

    ForwardReturnDistribution {
        market: market.to_string(),
        horizon_days,
        sample_count: n,
        p01: percentile(&rets, 0.01),
        p05: percentile(&rets, 0.05),
        p10: percentile(&rets, 0.10),
        p25: percentile(&rets, 0.25),
        p50: percentile(&rets, 0.50),
        p75: percentile(&rets, 0.75),
        p90: percentile(&rets, 0.90),
        p95: percentile(&rets, 0.95),
        mean,
        std,
        min: *rets.first().unwrap_or(&0.0),
        max: *rets.last().unwrap_or(&0.0),
    }
}

pub fn audit_forward_return_distribution(
    cn_bars: &[DailyBar],
    hk_bars: &[DailyBar],
) -> DistributionAuditReport {
    let horizons = vec![20, 60, 120];

    let mut cn_distributions = Vec::new();
    let mut hk_distributions = Vec::new();

    for &horizon in &horizons {
        let cn_returns = compute_forward_returns(cn_bars, horizon);
        let hk_returns = compute_forward_returns(hk_bars, horizon);

        if !cn_returns.is_empty() {
            cn_distributions.push(compute_distribution("CN", horizon, &cn_returns));
        }
        if !hk_returns.is_empty() {
            hk_distributions.push(compute_distribution("HK", horizon, &hk_returns));
        }
    }

    DistributionAuditReport {
        cn_distributions,
        hk_distributions,
    }
}

//! Forward-outcome profiling for historical matches.
//!
//! Given a set of matched dates and a provider that can look up forward returns,
//! computes aggregate statistics such as median/mean/best/worst return and
//! win rate.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::matcher::HistoricalMatch;

/// Provider of forward-looking return data for a given date.
///
/// This trait keeps `market-fingerprint-engine` decoupled from `market-store`
/// and other I/O crates. The implementation lives in the orchestration layer
/// (`app-service`).
pub trait ForwardReturnProvider {
    /// Return the forward price return over `horizon_days` trading days
    /// starting from `date` (exclusive of `date` in the period).
    ///
    /// Returns `None` if the required data is not available (e.g. not enough
    /// future trading days in the bar series).
    fn forward_return(&self, date: NaiveDate, horizon_days: usize) -> Option<f64>;

    /// Return the maximum drawdown over `horizon_days` trading days
    /// starting from `date`.
    ///
    /// Returns `None` if the required data is not available.
    fn forward_max_drawdown(&self, date: NaiveDate, horizon_days: usize) -> Option<f64>;
}

/// Aggregate forward-outcome statistics for a set of historical matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeProfile {
    /// Number of trading days in the forward horizon.
    pub horizon_days: usize,
    /// Median forward return (as a decimal; 0.02 = +2%).
    pub median: f64,
    /// Mean forward return.
    pub mean: f64,
    /// Best (maximum) forward return.
    pub best: f64,
    /// Worst (minimum) forward return.
    pub worst: f64,
    /// Fraction of matched dates with positive forward returns (0.0–1.0).
    pub win_rate: f64,
    /// Median maximum drawdown over the forward horizon (as a negative decimal).
    pub median_max_drawdown: f64,
}

/// Profiles forward outcomes for a set of historical matches.
pub struct OutcomeProfiler;

impl OutcomeProfiler {
    /// Compute an `OutcomeProfile` for a set of matched dates.
    ///
    /// For each match, queries `provider` for the forward return and
    /// max drawdown. Returns `None` if no matches had valid forward data.
    pub fn profile(
        matches: &[HistoricalMatch],
        horizon_days: usize,
        provider: &impl ForwardReturnProvider,
    ) -> Option<OutcomeProfile> {
        let mut returns: Vec<f64> = Vec::with_capacity(matches.len());
        let mut drawdowns: Vec<f64> = Vec::with_capacity(matches.len());

        for m in matches {
            if let Some(ret) = provider.forward_return(m.date, horizon_days) {
                returns.push(ret);
            }
            if let Some(dd) = provider.forward_max_drawdown(m.date, horizon_days) {
                drawdowns.push(dd);
            }
        }

        if returns.is_empty() {
            return None;
        }

        // Sort for percentile computation
        let mut sorted_returns = returns.clone();
        sorted_returns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted_returns.len();
        let median = percentile(&sorted_returns, 0.50);
        let mean = returns.iter().sum::<f64>() / n as f64;
        let best = sorted_returns.last().copied().unwrap_or(0.0);
        let worst = sorted_returns.first().copied().unwrap_or(0.0);
        let win_rate = returns.iter().filter(|&&r| r > 0.0).count() as f64 / n as f64;

        let median_max_drawdown = if !drawdowns.is_empty() {
            let mut sorted_dds = drawdowns.clone();
            sorted_dds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            percentile(&sorted_dds, 0.50)
        } else {
            0.0
        };

        Some(OutcomeProfile {
            horizon_days,
            median,
            mean,
            best,
            worst,
            win_rate,
            median_max_drawdown,
        })
    }
}

/// Compute the value at a given percentile from a sorted slice.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = p * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = idx - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock provider that returns predefined forward returns.
    struct MockProvider {
        returns: std::collections::HashMap<NaiveDate, f64>,
        drawdowns: std::collections::HashMap<NaiveDate, f64>,
    }

    impl ForwardReturnProvider for MockProvider {
        fn forward_return(&self, date: NaiveDate, _horizon_days: usize) -> Option<f64> {
            self.returns.get(&date).copied()
        }

        fn forward_max_drawdown(&self, date: NaiveDate, _horizon_days: usize) -> Option<f64> {
            self.drawdowns.get(&date).copied()
        }
    }

    #[test]
    fn profile_computes_statistics() {
        let mut ret = std::collections::HashMap::new();
        let mut dd = std::collections::HashMap::new();
        let d1 = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 1, 6).unwrap();
        let d3 = NaiveDate::from_ymd_opt(2026, 1, 7).unwrap();
        ret.insert(d1, 0.05);
        ret.insert(d2, -0.02);
        ret.insert(d3, 0.10);
        dd.insert(d1, -0.03);
        dd.insert(d2, -0.05);
        dd.insert(d3, -0.02);

        let provider = MockProvider {
            returns: ret,
            drawdowns: dd,
        };

        let matches = vec![
            HistoricalMatch { date: d1, level: crate::matcher::MatchLevel::VeryHigh },
            HistoricalMatch { date: d2, level: crate::matcher::MatchLevel::High },
            HistoricalMatch { date: d3, level: crate::matcher::MatchLevel::Moderate },
        ];

        let profile = OutcomeProfiler::profile(&matches, 20, &provider).unwrap();

        assert_eq!(profile.horizon_days, 20);
        assert!((profile.mean - (0.05 - 0.02 + 0.10) / 3.0).abs() < 1e-10);
        assert!((profile.best - 0.10).abs() < 1e-10);
        assert!((profile.worst - (-0.02)).abs() < 1e-10);
        assert!((profile.win_rate - 2.0 / 3.0).abs() < 1e-10);
        assert!((profile.median_max_drawdown - (-0.03)).abs() < 1e-10); // median of [-0.05, -0.03, -0.02]
    }

    #[test]
    fn profile_empty_matches_returns_none() {
        let provider = MockProvider {
            returns: std::collections::HashMap::new(),
            drawdowns: std::collections::HashMap::new(),
        };
        let profile = OutcomeProfiler::profile(&[], 20, &provider);
        assert!(profile.is_none());
    }

    #[test]
    fn percentile_single_element() {
        let v = vec![5.0];
        assert!((percentile(&v, 0.5) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn percentile_interpolated() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        assert!((percentile(&v, 0.5) - 2.5).abs() < 1e-10);
    }

    #[test]
    fn percentile_empty() {
        assert!((percentile(&[], 0.5) - 0.0).abs() < 1e-10);
    }
}

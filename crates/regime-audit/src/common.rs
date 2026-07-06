// ============================================================
// Common audit utilities
// Extracted from multiple audit modules to eliminate duplication
// while preserving standalone reproducibility per module.
// ============================================================

/// Apply persistence smoothing to a sequence of raw regime labels.
///
/// A regime label only becomes "active" after `days` consecutive occurrences.
/// Until then, the previous persisted label is carried forward.
/// This is the core mechanism tested across all Wave 7/8 audit modules.
pub fn apply_persistence(raw_labels: &[String], days: usize) -> Vec<String> {
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

/// Calculate max drawdown from a starting close and a series of forward closes.
///
/// Returns the maximum peak-to-trough decline as a positive ratio.
/// Example: 0.15 means a 15% drawdown.
pub fn calculate_max_drawdown(start_close: f64, forward_closes: &[f64]) -> f64 {
    if start_close <= 0.0 {
        return 0.0;
    }
    let mut peak = start_close;
    let mut max_dd = 0.0;
    for close in forward_closes {
        if *close > peak {
            peak = *close;
        }
        let dd = (peak - *close) / peak;
        if dd > max_dd {
            max_dd = dd;
        }
    }
    max_dd
}

/// Calculate annualized realized volatility from a starting close and forward closes.
///
/// Uses log returns and assumes 252 trading days per year.
pub fn calculate_realized_volatility_from_closes(start_close: f64, forward_closes: &[f64]) -> f64 {
    let mut prices = vec![start_close];
    prices.extend_from_slice(forward_closes);

    if prices.len() < 5 {
        return 0.0;
    }

    let mut log_returns = Vec::with_capacity(prices.len() - 1);
    for window in prices.windows(2) {
        if window[0] > 0.0 {
            log_returns.push((window[1] / window[0]).ln());
        }
    }

    if log_returns.len() < 2 {
        return 0.0;
    }

    let mean = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
    let variance = log_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / log_returns.len() as f64;

    variance.sqrt() * (252.0_f64).sqrt()
}

/// Return the percentile of a sorted slice (0.0 = lowest, 1.0 = highest).
///
/// The slice must be sorted in ascending order. Uses nearest-rank interpolation.
pub fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = (p * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.clamp(0, sorted.len() - 1)]
}


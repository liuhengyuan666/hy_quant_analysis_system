//! Percentile helpers used by research metrics.

/// Compute the percentile rank of `value` within a sorted ascending slice.
pub fn percentile_rank(value: f64, sorted: &[f64]) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let below = sorted.iter().filter(|&&v| v <= value).count();
    (below as f64 / sorted.len() as f64) * 100.0
}

/// Convert a percentile into a qualitative label.
pub fn percentile_label(p: f64) -> &'static str {
    if p < 20.0 {
        "Very Low"
    } else if p < 40.0 {
        "Low"
    } else if p < 60.0 {
        "Moderate"
    } else if p < 80.0 {
        "High"
    } else {
        "Very High"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_rank_at_exact_values() {
        let sorted = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        // value <= 40.0 covers 4 of 8 entries
        assert_eq!(percentile_rank(40.0, &sorted), 50.0);
    }

    #[test]
    fn percentile_rank_empty() {
        assert_eq!(percentile_rank(40.0, &[]), 0.0);
    }

    #[test]
    fn percentile_label_buckets() {
        assert_eq!(percentile_label(0.0), "Very Low");
        assert_eq!(percentile_label(19.9), "Very Low");
        assert_eq!(percentile_label(20.0), "Low");
        assert_eq!(percentile_label(39.9), "Low");
        assert_eq!(percentile_label(40.0), "Moderate");
        assert_eq!(percentile_label(59.9), "Moderate");
        assert_eq!(percentile_label(60.0), "High");
        assert_eq!(percentile_label(79.9), "High");
        assert_eq!(percentile_label(80.0), "Very High");
        assert_eq!(percentile_label(100.0), "Very High");
    }
}

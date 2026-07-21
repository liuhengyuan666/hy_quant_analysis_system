use serde::{Deserialize, Serialize};

use crate::v2::request::QuoteSnapshot;

/// Inputs to the FeatureExtractor.
///
/// FeatureExtractor is intentionally narrow: it only needs a real-time quote
/// and a pre-computed volume MA20. It does not depend on ResearchContext,
/// SignalSnapshot, or StrategyStateSnapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureExtractorInputs {
    pub quote: QuoteSnapshot,
    pub volume_ma20: f64,
}

/// Pure mathematical features derived from a single quote snapshot.
///
/// IntradayFeatures carries no semantic interpretation. It is the raw
/// material for ObservationEngine and, later, for Replay and ML calibration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntradayFeatures {
    pub symbol: String,

    // Raw features
    pub today_return: f64,
    pub open_return: f64,
    pub gap_pct: f64,
    pub close_position: f64,
    pub amplitude_pct: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub volume_ratio: f64,

    // Derived features
    pub body_ratio: f64,
    pub gap_fill_ratio: f64,
}

/// Replay record for the Feature layer.
///
/// Because FeatureExtractor is a pure function, this record is sufficient to
/// reproduce the exact IntradayFeatures for a given timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureReplayRecord {
    pub quote: QuoteSnapshot,
    pub volume_ma20: f64,
    pub features: IntradayFeatures,
}

/// Extracts mathematical features from a quote snapshot.
pub trait FeatureExtractor {
    fn extract(&self, inputs: &FeatureExtractorInputs) -> IntradayFeatures;
}

/// Default feature extractor implementing the MVP feature set.
#[derive(Debug, Clone, Default)]
pub struct DefaultFeatureExtractor;

impl FeatureExtractor for DefaultFeatureExtractor {
    fn extract(&self, inputs: &FeatureExtractorInputs) -> IntradayFeatures {
        let quote = &inputs.quote;
        let prev_close = quote.prev_close;

        let today_return = (quote.close - prev_close) / prev_close;
        let open_return = (quote.open - prev_close) / prev_close;
        let gap_pct = open_return;
        let amplitude_pct = (quote.high - quote.low) / prev_close;

        let high_low_range = quote.high - quote.low;
        let close_position = if high_low_range > 0.0 {
            (quote.close - quote.low) / high_low_range
        } else {
            0.5
        };

        let upper_shadow_pct = if high_low_range > 0.0 {
            (quote.high - quote.close.max(quote.open)) / high_low_range
        } else {
            0.0
        };

        let lower_shadow_pct = if high_low_range > 0.0 {
            (quote.close.min(quote.open) - quote.low) / high_low_range
        } else {
            0.0
        };

        let body_ratio = if high_low_range > 0.0 {
            (quote.close - quote.open).abs() / high_low_range
        } else {
            0.0
        };

        let gap_fill_ratio = if gap_pct > 0.0 {
            // Bullish gap: how much of the gap has been filled by pullback
            let gap_fill = (quote.open - quote.low).max(0.0);
            let gap_size = quote.open - prev_close;
            if gap_size > 0.0 {
                (gap_fill / gap_size).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else if gap_pct < 0.0 {
            // Bearish gap: how much of the gap has been filled by rebound
            let gap_fill = (quote.high - quote.open).max(0.0);
            let gap_size = prev_close - quote.open;
            if gap_size > 0.0 {
                (gap_fill / gap_size).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let volume_ratio = if inputs.volume_ma20 > 0.0 {
            quote.volume / inputs.volume_ma20
        } else {
            1.0
        };

        IntradayFeatures {
            symbol: quote.symbol.clone(),
            today_return,
            open_return,
            gap_pct,
            close_position,
            amplitude_pct,
            upper_shadow_pct,
            lower_shadow_pct,
            volume_ratio,
            body_ratio,
            gap_fill_ratio,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_quote(
        symbol: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        prev_close: f64,
    ) -> QuoteSnapshot {
        QuoteSnapshot {
            symbol: symbol.into(),
            ts: Utc::now(),
            open,
            high,
            low,
            close,
            volume,
            prev_close,
        }
    }

    #[test]
    fn basic_feature_extraction() {
        let quote = make_quote("000001", 10.0, 11.0, 9.5, 10.5, 1_000_000.0, 10.0);
        let inputs = FeatureExtractorInputs {
            quote,
            volume_ma20: 500_000.0,
        };
        let f = DefaultFeatureExtractor.extract(&inputs);

        assert!((f.today_return - 0.05).abs() < 1e-9);
        assert!((f.open_return - 0.0).abs() < 1e-9);
        assert!((f.gap_pct - 0.0).abs() < 1e-9);
        assert!((f.amplitude_pct - 0.15).abs() < 1e-9);
        assert!((f.close_position - 0.6666666666666666).abs() < 1e-9);
        assert!((f.volume_ratio - 2.0).abs() < 1e-9);
        assert!((f.body_ratio - 0.3333333333333333).abs() < 1e-9);
    }

    #[test]
    fn bullish_gap_with_fill() {
        // open 2% gap up, then low pulls back to close 50% of the gap
        let quote = make_quote("000001", 10.2, 10.5, 10.1, 10.4, 1_000_000.0, 10.0);
        let inputs = FeatureExtractorInputs {
            quote,
            volume_ma20: 500_000.0,
        };
        let f = DefaultFeatureExtractor.extract(&inputs);

        assert!((f.gap_pct - 0.02).abs() < 1e-9);
        // gap size = 0.2, gap fill = open - low = 0.1, fill ratio = 0.5
        assert!((f.gap_fill_ratio - 0.5).abs() < 1e-9);
    }

    #[test]
    fn bearish_gap_with_fill() {
        // open 2% gap down, then high rebounds to close 50% of the gap
        let quote = make_quote("000001", 9.8, 9.9, 9.5, 9.6, 1_000_000.0, 10.0);
        let inputs = FeatureExtractorInputs {
            quote,
            volume_ma20: 500_000.0,
        };
        let f = DefaultFeatureExtractor.extract(&inputs);

        assert!((f.gap_pct - (-0.02)).abs() < 1e-9);
        // gap size = 0.2, gap fill = high - open = 0.1, fill ratio = 0.5
        assert!((f.gap_fill_ratio - 0.5).abs() < 1e-9);
    }

    #[test]
    fn long_upper_shadow() {
        let quote = make_quote("000001", 10.0, 12.0, 10.0, 10.0, 1_000_000.0, 10.0);
        let inputs = FeatureExtractorInputs {
            quote,
            volume_ma20: 500_000.0,
        };
        let f = DefaultFeatureExtractor.extract(&inputs);

        assert!((f.upper_shadow_pct - 1.0).abs() < 1e-9);
        assert!(f.lower_shadow_pct.abs() < 1e-9);
        assert!(f.body_ratio.abs() < 1e-9);
    }
}

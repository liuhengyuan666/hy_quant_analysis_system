use serde::{Deserialize, Serialize};

use crate::v2::feature::IntradayFeatures;

/// A semantic observation of intraday market behavior.
///
/// Observation is the first semantic layer in the Execution Pipeline. It is
/// produced from pure mathematical `IntradayFeatures` and carries no trading
/// decision. Multiple observations can co-exist for the same symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntradayObservation {
    pub kind: ObservationKind,
    pub confidence: f64,
    pub direction: f64,
    pub payload: ObservationPayload,
}

/// Category of an observation, used by Formatters and Evidence builders to
/// group related market phenomena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObservationCategory {
    Trend,
    Risk,
    Volatility,
    Structure,
}

impl ObservationKind {
    pub fn category(&self) -> ObservationCategory {
        match self {
            Self::TrendPersistence | Self::CloseStrength | Self::BuyingPressure => {
                ObservationCategory::Trend
            }
            Self::Distribution | Self::FailedBreakout | Self::LiquidityDryUp => {
                ObservationCategory::Risk
            }
            Self::VolatilityExpansion => ObservationCategory::Volatility,
            Self::BreakoutAttempt => ObservationCategory::Structure,
        }
    }
}

/// Semantic kind of an intraday observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObservationKind {
    // Trend
    TrendPersistence,
    CloseStrength,
    BuyingPressure,

    // Risk
    Distribution,
    FailedBreakout,
    LiquidityDryUp,

    // Volatility
    VolatilityExpansion,

    // Structure
    BreakoutAttempt,
}

/// Typed payload for an observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ObservationPayload {
    TrendPersistence { close_position: f64, body_ratio: f64 },
    CloseStrength { close_position: f64, volume_ratio: f64 },
    BuyingPressure { close_position: f64, body_ratio: f64 },
    Distribution { close_position: f64, volume_ratio: f64 },
    FailedBreakout { gap_pct: f64, gap_fill_ratio: f64 },
    LiquidityDryUp { volume_ratio: f64 },
    VolatilityExpansion { amplitude_pct: f64 },
    BreakoutAttempt { today_return: f64, close_position: f64 },
    Empty,
}

/// Replay record for the Observation layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationReplayRecord {
    pub features: IntradayFeatures,
    pub observations: Vec<IntradayObservation>,
}

/// Converts mathematical features into semantic observations.
///
/// The engine is stateless and only consumes `IntradayFeatures`. It never reads
/// the original quote, signal, or market context.
pub trait ObservationEngine {
    fn observe(&self, features: &IntradayFeatures) -> Vec<IntradayObservation>;
}

/// Default observation engine implementing the MVP observation set.
#[derive(Debug, Clone, Default)]
pub struct DefaultObservationEngine;

impl ObservationEngine for DefaultObservationEngine {
    fn observe(&self, features: &IntradayFeatures) -> Vec<IntradayObservation> {
        let mut obs = Vec::new();

        // Trend: TrendPersistence
        if features.close_position > 0.5 && features.body_ratio > 0.6 {
            obs.push(IntradayObservation {
                kind: ObservationKind::TrendPersistence,
                confidence: scale(features.body_ratio, 0.6, 1.0)
                    * scale(features.close_position, 0.5, 0.9),
                direction: 1.0,
                payload: ObservationPayload::TrendPersistence {
                    close_position: features.close_position,
                    body_ratio: features.body_ratio,
                },
            });
        }

        // Trend: CloseStrength
        if features.close_position > 0.75 && features.volume_ratio > 1.3 {
            obs.push(IntradayObservation {
                kind: ObservationKind::CloseStrength,
                confidence: scale(features.close_position, 0.75, 0.95)
                    * scale(features.volume_ratio, 1.3, 2.5),
                direction: 1.0,
                payload: ObservationPayload::CloseStrength {
                    close_position: features.close_position,
                    volume_ratio: features.volume_ratio,
                },
            });
        }

        // Trend: BuyingPressure
        if features.close_position > 0.6 && features.body_ratio > 0.5 {
            obs.push(IntradayObservation {
                kind: ObservationKind::BuyingPressure,
                confidence: scale(features.close_position, 0.6, 0.9)
                    * scale(features.body_ratio, 0.5, 1.0),
                direction: 1.0,
                payload: ObservationPayload::BuyingPressure {
                    close_position: features.close_position,
                    body_ratio: features.body_ratio,
                },
            });
        }

        // Structure: BreakoutAttempt
        if features.today_return > 0.03 && features.close_position > 0.8 {
            obs.push(IntradayObservation {
                kind: ObservationKind::BreakoutAttempt,
                confidence: scale(features.today_return, 0.03, 0.07)
                    * scale(features.close_position, 0.8, 0.95),
                direction: 1.0,
                payload: ObservationPayload::BreakoutAttempt {
                    today_return: features.today_return,
                    close_position: features.close_position,
                },
            });
        }

        // Risk: FailedBreakout
        if features.today_return > 0.02 && features.close_position < 0.3 {
            obs.push(IntradayObservation {
                kind: ObservationKind::FailedBreakout,
                confidence: scale(features.today_return, 0.02, 0.05)
                    * (1.0 - scale(features.close_position, 0.1, 0.3))
                    * scale(features.gap_fill_ratio, 0.5, 1.0),
                direction: -1.0,
                payload: ObservationPayload::FailedBreakout {
                    gap_pct: features.gap_pct,
                    gap_fill_ratio: features.gap_fill_ratio,
                },
            });
        }

        // Risk: Distribution
        if features.close_position < 0.2
            && features.volume_ratio > 1.5
            && features.today_return < 0.0
        {
            obs.push(IntradayObservation {
                kind: ObservationKind::Distribution,
                confidence: scale(features.volume_ratio, 1.5, 2.5)
                    * (1.0 - scale(features.close_position, 0.0, 0.2))
                    * (-features.today_return).min(0.05) / 0.05,
                direction: -1.0,
                payload: ObservationPayload::Distribution {
                    close_position: features.close_position,
                    volume_ratio: features.volume_ratio,
                },
            });
        }

        // Risk: LiquidityDryUp
        if features.volume_ratio < 0.7 {
            obs.push(IntradayObservation {
                kind: ObservationKind::LiquidityDryUp,
                confidence: 1.0 - scale(features.volume_ratio, 0.3, 0.7),
                direction: -1.0,
                payload: ObservationPayload::LiquidityDryUp {
                    volume_ratio: features.volume_ratio,
                },
            });
        }

        // Volatility: VolatilityExpansion
        if features.amplitude_pct > 0.05 {
            obs.push(IntradayObservation {
                kind: ObservationKind::VolatilityExpansion,
                confidence: scale(features.amplitude_pct, 0.05, 0.10),
                direction: -1.0,
                payload: ObservationPayload::VolatilityExpansion {
                    amplitude_pct: features.amplitude_pct,
                },
            });
        }

        obs
    }
}

/// Linearly scale `value` from [low, high] to [0.0, 1.0], clamped.
fn scale(value: f64, low: f64, high: f64) -> f64 {
    if high <= low {
        return if value >= low { 1.0 } else { 0.0 };
    }
    ((value - low) / (high - low)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_features(
        today_return: f64,
        close_position: f64,
        volume_ratio: f64,
        body_ratio: f64,
        amplitude_pct: f64,
        gap_pct: f64,
        gap_fill_ratio: f64,
    ) -> IntradayFeatures {
        IntradayFeatures {
            symbol: "000001".into(),
            today_return,
            open_return: gap_pct,
            gap_pct,
            close_position,
            amplitude_pct,
            upper_shadow_pct: 0.0,
            lower_shadow_pct: 0.0,
            volume_ratio,
            body_ratio,
            gap_fill_ratio,
        }
    }

    #[test]
    fn strong_close_observed() {
        // Close at 85% of range, volume 1.5x
        let f = make_features(0.02, 0.85, 1.5, 0.8, 0.03, 0.0, 0.0);
        let obs = DefaultObservationEngine.observe(&f);

        let close_strength = obs
            .iter()
            .find(|o| matches!(o.kind, ObservationKind::CloseStrength));
        assert!(close_strength.is_some());
        assert_eq!(close_strength.unwrap().direction, 1.0);
    }

    #[test]
    fn failed_breakout_observed() {
        // Gapped up 3%, but closed at 20% of range and filled 70% of gap
        let f = make_features(0.03, 0.20, 1.0, 0.3, 0.05, 0.03, 0.7);
        let obs = DefaultObservationEngine.observe(&f);

        let failed = obs
            .iter()
            .find(|o| matches!(o.kind, ObservationKind::FailedBreakout));
        assert!(failed.is_some());
        assert_eq!(failed.unwrap().direction, -1.0);
    }

    #[test]
    fn distribution_observed() {
        // Down on heavy volume, close at bottom
        let f = make_features(-0.02, 0.10, 2.0, 0.2, 0.04, 0.0, 0.0);
        let obs = DefaultObservationEngine.observe(&f);

        let dist = obs
            .iter()
            .find(|o| matches!(o.kind, ObservationKind::Distribution));
        assert!(dist.is_some());
        assert_eq!(dist.unwrap().direction, -1.0);
    }

    #[test]
    fn multiple_observations_can_coexist() {
        // Strong trend day: buying pressure, close strength, trend persistence
        let f = make_features(0.04, 0.85, 1.5, 0.8, 0.04, 0.0, 0.0);
        let obs = DefaultObservationEngine.observe(&f);

        assert!(obs.iter().any(|o| matches!(o.kind, ObservationKind::TrendPersistence)));
        assert!(obs.iter().any(|o| matches!(o.kind, ObservationKind::CloseStrength)));
        assert!(obs.iter().any(|o| matches!(o.kind, ObservationKind::BuyingPressure)));
    }

    #[test]
    fn observation_is_stateless() {
        let f = make_features(0.04, 0.85, 1.5, 0.8, 0.04, 0.0, 0.0);
        let obs1 = DefaultObservationEngine.observe(&f);
        let obs2 = DefaultObservationEngine.observe(&f);
        assert_eq!(obs1.len(), obs2.len());
        for (a, b) in obs1.iter().zip(obs2.iter()) {
            assert_eq!(a.kind, b.kind);
            assert!((a.confidence - b.confidence).abs() < 1e-12);
            assert_eq!(a.direction, b.direction);
        }
    }
}

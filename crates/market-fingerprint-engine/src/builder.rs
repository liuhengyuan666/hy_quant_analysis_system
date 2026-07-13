//! Market Fingerprint Builder.
//!
//! Adapts from domain sources (ResearchContext, and in future DashboardSnapshot)
//! into the canonical `MarketFingerprint`.

use crate::fingerprint::{EvolutionVector, MarketFingerprint, ObservationVector};
use research_context::ResearchContext;

/// Builder that converts domain snapshots into a canonical `MarketFingerprint`.
///
/// V7.2A: supports `ResearchContext`.
/// Future: may support `DashboardSnapshot` or other sources as adapters.
pub struct MarketFingerprintBuilder;

impl MarketFingerprintBuilder {
    /// Build a `MarketFingerprint` from a V6/V7 `ResearchContext`.
    ///
    /// The builder extracts the cross-consumer semantic summaries and converts
    /// them into the canonical historical feature representation. It does not
    /// perform similarity matching or normalization — those are consumers of
    /// the fingerprint, defined in V7.2B.
    pub fn build(research: &ResearchContext) -> MarketFingerprint {
        let environment = research.market_state.confidence * 100.0;
        let signal = research.signal.average_score;
        let stretch = 0.0; // Stretch is not yet a top-level ResearchContext score; placeholder for V7.2B.

        let rotation: Vec<(String, f64)> = research
            .rotation
            .top
            .iter()
            .take(10)
            .map(|item| (item.symbol.clone(), item.momentum_score))
            .collect();

        let observation = ObservationVector {
            environment,
            signal,
            stretch,
            rotation,
        };

        let confirmation = (research.confirmation.trend.score
            + research.confirmation.participation.score
            + research.confirmation.risk.score)
            / 3.0;
        let recovery = research.recovery.score;

        let evolution = EvolutionVector {
            confirmation,
            recovery,
        };

        MarketFingerprint {
            scope: research.scope,
            date: research.date,
            observation,
            evolution,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use core_domain::AnalysisScope;
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, DivergenceSummary,
        MarketStateSummary, RecoverySummary, RotationItem, RotationSummary, SignalItem,
        SignalSummary, TrustLevel, TrustSummary,
    };

    fn dummy_context() -> ResearchContext {
        ResearchContext {
            version: 1,
            scope: AnalysisScope::Global,
            date: NaiveDate::from_ymd_opt(2026, 7, 8).unwrap(),
            market_state: MarketStateSummary {
                label: "risk_on".to_string(),
                trend_score: 75.0,
                liquidity_score: 60.0,
                risk_score: 40.0,
                confidence: 0.8,
            },
            breadth: BreadthSummary {
                breadth_pct: 65.0,
                sma5: Some(62.0),
                delta_5d: Some(3.0),
                condition: "Strong".to_string(),
            },
            rotation: RotationSummary {
                top: vec![RotationItem {
                    rank: 1,
                    symbol: "TECH".to_string(),
                    momentum_score: 95.0,
                }],
                bottom: vec![],
                rotation_state: "Concentrated".to_string(),
                leadership_stability: 0.7,
                leadership_transition: "Stable".to_string(),
                rotation_acceleration: None,
                theme_dispersion: None,
            },
            signal: SignalSummary {
                signals: vec![SignalItem {
                    symbol: "TECH".to_string(),
                    final_score: 88.0,
                    signal_label: "StrongBuy".to_string(),
                }],
                bullish_count: 5,
                strong_buy_count: 2,
                average_score: 72.0,
            },
            divergence: DivergenceSummary {
                divergence_duration: 0,
                samples: vec![],
            },
            trust: TrustSummary {
                level: TrustLevel::Unassessed,
                headline: "Data healthy".to_string(),
                is_data_complete: true,
            },
            confirmation: ConfirmationSummary {
                trend: ConfirmationDimension {
                    score: 75.0,
                    label: "Strong".to_string(),
                },
                participation: ConfirmationDimension {
                    score: 45.0,
                    label: "Moderate".to_string(),
                },
                risk: ConfirmationDimension {
                    score: 70.0,
                    label: "Strong".to_string(),
                },
                overall: "Moderate".to_string(),
            },
            recovery: RecoverySummary {
                score: 42.0,
                drivers: vec!["Breadth improving".to_string()],
            },
        }
    }

    #[test]
    fn builder_extracts_observation_and_evolution() {
        let ctx = dummy_context();
        let fp = MarketFingerprintBuilder::build(&ctx);

        assert_eq!(fp.scope, AnalysisScope::Global);
        assert_eq!(fp.date, NaiveDate::from_ymd_opt(2026, 7, 8).unwrap());
        assert_eq!(fp.observation.environment, 80.0); // confidence * 100
        assert_eq!(fp.observation.signal, 72.0);
        assert_eq!(fp.observation.rotation.len(), 1);
        assert_eq!(fp.evolution.confirmation, 63.333333333333336); // (75+45+70)/3
        assert_eq!(fp.evolution.recovery, 42.0);
    }

    #[test]
    fn builder_top_rotation_limited_to_ten() {
        let mut ctx = dummy_context();
        ctx.rotation.top = (0..20)
            .map(|i| RotationItem {
                rank: i as i32 + 1,
                symbol: format!("SYM{}", i),
                momentum_score: 100.0 - i as f64,
            })
            .collect();
        let fp = MarketFingerprintBuilder::build(&ctx);
        assert_eq!(fp.observation.rotation.len(), 10);
    }
}

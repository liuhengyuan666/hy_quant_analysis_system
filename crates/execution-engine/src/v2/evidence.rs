use core_domain::{SignalLabel, StrategyState};
use serde::{Deserialize, Serialize};

use crate::v2::observation::{IntradayObservation, ObservationKind};
use crate::v2::request::ExecutionMarketView;

/// Unified semantic unit across Research, Execution, and Review.
///
/// Evidence carries a semantic kind, a confidence, a direction, a source, and a
/// typed payload. It is the only cross-layer reasoning primitive in the
/// Execution Platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    /// Confidence in [0.0, 1.0].
    pub confidence: f64,
    /// Direction: +1.0 bullish, -1.0 bearish, 0.0 neutral.
    pub direction: f64,
    pub source: EvidenceSource,
    pub payload: EvidencePayload,
}

/// Semantic kind of an Evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceKind {
    // Intraday observations
    TrendParticipation,
    MarketAcceptance,
    MomentumExpansion,
    MomentumFailure,
    Distribution,
    RiskCompression,
    RiskExpansion,
    LeadershipRotation,
    LiquidityConfirmation,

    // Research context projections
    Confirmation,
    Recovery,
    Breadth,
    Stretch,

    // Strategy state and signal
    StrategyState,
    SignalStrength,
}

/// Source of an Evidence, used for audit and replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceSource {
    ResearchContext,
    IntradayObservation,
    StrategyState,
    SignalModel,
}

/// Typed payload for an Evidence.
///
/// Using a typed enum instead of `serde_json::Value` preserves type safety and
/// lets Formatters pattern-match on structured data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum EvidencePayload {
    Gap { gap_pct: f64 },
    Volume { volume_ratio: f64 },
    Breadth { breadth_pct: f64, delta_5d: f64 },
    Close { close_position: f64 },
    Distribution { distribution_score: f64 },
    Confirmation {
        trend_score: f64,
        participation_score: f64,
        risk_score: f64,
    },
    Rotation {
        rotation_state: String,
        leadership_stability: f64,
    },
    StrategyState {
        state_label: String,
        recommended_position_pct: f64,
    },
    Signal {
        final_score: f64,
        signal_label: String,
    },
    Empty,
}

/// Builds a unified Evidence list from all input layers.
///
/// This is the bridge between the Observation layer, the Research layer, the
/// Strategy state, and the Signal model. Every non-Evidence input must be
/// converted here before assessment.
pub trait EvidenceBuilder {
    fn build(
        &self,
        observations: &[IntradayObservation],
        market_view: &ExecutionMarketView,
        signal: &core_domain::SignalSnapshot,
        state: &core_domain::StrategyStateSnapshot,
    ) -> Vec<Evidence>;
}

/// Default evidence builder implementing the MVP conversion rules.
#[derive(Debug, Clone, Default)]
pub struct DefaultEvidenceBuilder;

impl EvidenceBuilder for DefaultEvidenceBuilder {
    fn build(
        &self,
        observations: &[IntradayObservation],
        market_view: &ExecutionMarketView,
        signal: &core_domain::SignalSnapshot,
        state: &core_domain::StrategyStateSnapshot,
    ) -> Vec<Evidence> {
        let mut evidences = Vec::new();

        // Convert intraday observations to evidence.
        for obs in observations {
            let kind = match obs.kind {
                ObservationKind::TrendPersistence => EvidenceKind::TrendParticipation,
                ObservationKind::CloseStrength => EvidenceKind::MarketAcceptance,
                ObservationKind::BuyingPressure => EvidenceKind::MomentumExpansion,
                ObservationKind::BreakoutAttempt => EvidenceKind::MomentumExpansion,
                ObservationKind::FailedBreakout => EvidenceKind::MomentumFailure,
                ObservationKind::Distribution => EvidenceKind::Distribution,
                ObservationKind::LiquidityDryUp => EvidenceKind::LiquidityConfirmation,
                ObservationKind::VolatilityExpansion => EvidenceKind::RiskExpansion,
            };

            evidences.push(Evidence {
                kind,
                confidence: obs.confidence.clamp(0.0, 1.0),
                direction: obs.direction,
                source: EvidenceSource::IntradayObservation,
                payload: EvidencePayload::Empty,
            });
        }

        // Research context: Confirmation.
        let confirmation = &market_view.confirmation;
        let confirmation_confidence = (confirmation.trend.score
            + confirmation.participation.score
            + (100.0 - confirmation.risk.score))
            / 300.0;
        let confirmation_direction = match confirmation.overall.as_str() {
            "Very Strong" | "Strong" => 1.0,
            "Moderate" => 0.0,
            _ => -1.0,
        };
        evidences.push(Evidence {
            kind: EvidenceKind::Confirmation,
            confidence: confirmation_confidence.clamp(0.0, 1.0),
            direction: confirmation_direction,
            source: EvidenceSource::ResearchContext,
            payload: EvidencePayload::Confirmation {
                trend_score: confirmation.trend.score,
                participation_score: confirmation.participation.score,
                risk_score: confirmation.risk.score,
            },
        });

        // Research context: Breadth.
        let breadth = &market_view.breadth;
        let breadth_direction = if breadth.breadth_pct > 50.0 { 1.0 } else { -1.0 };
        evidences.push(Evidence {
            kind: EvidenceKind::Breadth,
            confidence: (breadth.breadth_pct / 100.0).clamp(0.0, 1.0),
            direction: breadth_direction,
            source: EvidenceSource::ResearchContext,
            payload: EvidencePayload::Breadth {
                breadth_pct: breadth.breadth_pct,
                delta_5d: breadth.delta_5d.unwrap_or(0.0),
            },
        });

        // Research context: Recovery.
        let recovery = &market_view.recovery;
        evidences.push(Evidence {
            kind: EvidenceKind::Recovery,
            confidence: (recovery.score / 100.0).clamp(0.0, 1.0),
            direction: 1.0,
            source: EvidenceSource::ResearchContext,
            payload: EvidencePayload::Empty,
        });

        // Research context: Rotation / Leadership.
        evidences.push(Evidence {
            kind: EvidenceKind::LeadershipRotation,
            confidence: market_view.leadership_stability.clamp(0.0, 1.0),
            direction: 0.0,
            source: EvidenceSource::ResearchContext,
            payload: EvidencePayload::Rotation {
                rotation_state: market_view.rotation_state.clone(),
                leadership_stability: market_view.leadership_stability,
            },
        });

        // Strategy state.
        let state_direction = match state.state {
            StrategyState::FullTrend | StrategyState::ConfirmAdd => 1.0,
            StrategyState::LeftProbe => 0.5,
            StrategyState::DeRisk => -0.5,
            StrategyState::NoTrade => -1.0,
        };
        evidences.push(Evidence {
            kind: EvidenceKind::StrategyState,
            confidence: (state.state_score / 100.0).clamp(0.0, 1.0),
            direction: state_direction,
            source: EvidenceSource::StrategyState,
            payload: EvidencePayload::StrategyState {
                state_label: state.state.as_str().to_string(),
                recommended_position_pct: state.recommended_position_pct,
            },
        });

        // Signal.
        let signal_score_norm = (signal.final_score / 100.0).clamp(0.0, 1.0);
        let signal_direction = match signal.signal_label {
            SignalLabel::StrongBuy | SignalLabel::Buy => 1.0,
            SignalLabel::Watch | SignalLabel::Hold => 0.0,
            SignalLabel::Reduce | SignalLabel::Sell => -1.0,
        };
        evidences.push(Evidence {
            kind: EvidenceKind::SignalStrength,
            confidence: signal_score_norm,
            direction: signal_direction,
            source: EvidenceSource::SignalModel,
            payload: EvidencePayload::Signal {
                final_score: signal.final_score,
                signal_label: signal.signal_label.to_string(),
            },
        });

        evidences
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    fn make_signal(label: SignalLabel, score: f64) -> core_domain::SignalSnapshot {
        use core_domain::{RegimeReason, RotationReason, SignalReason, StrategyKind};
        core_domain::SignalSnapshot {
            date: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            symbol: "000001".into(),
            final_score: score,
            signal_label: label.clone(),
            analysis_scope: "CN".into(),
            regime_basis_scope: "CN".into(),
            reason: SignalReason {
                best_strategy: StrategyKind::MomentumRight,
                strategy_score: 0.0,
                strategy_contribution: 0.0,
                alignment: 0,
                aligned_strategies: vec![],
                alignment_contribution: 0.0,
                regime: RegimeReason {
                    trend_score: 0.0,
                    risk_score: 0.0,
                    combined_score: 0.0,
                    contribution: 0.0,
                },
                rotation: RotationReason {
                    momentum_score: 0.0,
                    rank: None,
                    combined_score: 0.0,
                    contribution: 0.0,
                },
                final_score: score,
                label,
                summary: "test".into(),
            },
        }
    }

    fn make_state(state: StrategyState) -> core_domain::StrategyStateSnapshot {
        core_domain::StrategyStateSnapshot {
            date: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            scope: "CN".into(),
            state,
            state_score: 50.0,
            transition_reason: "test".into(),
            recommended_position_pct: 0.0,
        }
    }

    fn make_market_view(overall: &str) -> ExecutionMarketView {
        ExecutionMarketView {
            research_version: "1".into(),
            market_regime_label: "Bullish".into(),
            confirmation: ConfirmationSummary {
                trend: ConfirmationDimension {
                    score: 60.0,
                    label: "Strong".into(),
                },
                participation: ConfirmationDimension {
                    score: 50.0,
                    label: "Moderate".into(),
                },
                risk: ConfirmationDimension {
                    score: 40.0,
                    label: "Low".into(),
                },
                overall: overall.into(),
            },
            breadth: BreadthSummary {
                breadth_pct: 45.0,
                sma5: None,
                delta_5d: Some(-5.0),
                condition: "weakening".into(),
            },
            recovery: RecoverySummary {
                score: 55.0,
                drivers: vec!["Breadth improving".into()],
            },
            rotation_state: "concentrated".into(),
            leadership_stability: 0.6,
        }
    }

    #[test]
    fn evidence_from_observation_preserves_direction() {
        let obs = IntradayObservation {
            kind: ObservationKind::CloseStrength,
            confidence: 0.8,
            direction: 1.0,
            payload: crate::v2::observation::ObservationPayload::Empty,
        };

        let signal = make_signal(SignalLabel::Buy, 82.0);
        let state = make_state(StrategyState::ConfirmAdd);
        let view = make_market_view("Strong");

        let evidences = DefaultEvidenceBuilder.build(&[obs], &view, &signal, &state);

        let e = evidences
            .iter()
            .find(|e| matches!(e.kind, EvidenceKind::MarketAcceptance))
            .expect("market acceptance evidence");
        assert!((e.confidence - 0.8).abs() < 1e-9);
        assert_eq!(e.direction, 1.0);
        assert!(matches!(e.source, EvidenceSource::IntradayObservation));
    }

    #[test]
    fn evidence_from_state_no_trade_is_negative() {
        let signal = make_signal(SignalLabel::StrongBuy, 92.0);
        let state = make_state(StrategyState::NoTrade);
        let view = make_market_view("Strong");

        let evidences = DefaultEvidenceBuilder.build(&[], &view, &signal, &state);

        let e = evidences
            .iter()
            .find(|e| matches!(e.kind, EvidenceKind::StrategyState))
            .expect("strategy state evidence");
        assert_eq!(e.direction, -1.0);
    }

    #[test]
    fn evidence_from_signal_strong_buy_is_positive() {
        let signal = make_signal(SignalLabel::StrongBuy, 92.0);
        let state = make_state(StrategyState::ConfirmAdd);
        let view = make_market_view("Strong");

        let evidences = DefaultEvidenceBuilder.build(&[], &view, &signal, &state);

        let e = evidences
            .iter()
            .find(|e| matches!(e.kind, EvidenceKind::SignalStrength))
            .expect("signal evidence");
        assert_eq!(e.direction, 1.0);
        assert!((e.confidence - 0.92).abs() < 1e-9);
    }

    #[test]
    fn all_input_layers_produce_evidence() {
        let obs = IntradayObservation {
            kind: ObservationKind::TrendPersistence,
            confidence: 0.7,
            direction: 1.0,
            payload: crate::v2::observation::ObservationPayload::Empty,
        };
        let signal = make_signal(SignalLabel::Buy, 80.0);
        let state = make_state(StrategyState::LeftProbe);
        let view = make_market_view("Moderate");

        let evidences = DefaultEvidenceBuilder.build(&[obs], &view, &signal, &state);

        assert!(evidences.iter().any(|e| e.source == EvidenceSource::IntradayObservation));
        assert!(evidences.iter().any(|e| e.source == EvidenceSource::ResearchContext));
        assert!(evidences.iter().any(|e| e.source == EvidenceSource::StrategyState));
        assert!(evidences.iter().any(|e| e.source == EvidenceSource::SignalModel));
    }
}


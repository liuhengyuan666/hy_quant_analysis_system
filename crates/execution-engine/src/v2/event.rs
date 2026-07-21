use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

use crate::v2::assessment::ExecutionAssessment;
use crate::v2::decision::ExecutionDecision;
use crate::v2::evidence::Evidence;
use crate::v2::feature::IntradayFeatures;
use crate::v2::observation::IntradayObservation;
use crate::v2::request::{ExecutionPolicy, ExecutionRequest};

/// Version metadata for an ExecutionEvent.
///
/// These versions are deliberately granular so that Historical Replay and
/// Research Asset consumers can reproduce or explain an event without ambiguity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEventVersions {
    pub schema_version: String,
    pub engine_version: String,
    pub policy_version: String,
    pub research_version: String,
}

/// The single, deterministic output of the Execution Platform.
///
/// `ExecutionEvent` is the canonical fact produced by the Execution Platform.
/// All downstream consumers—Replay, Research Asset, Report Engine, Desktop, and
/// LLM—must derive their outputs from this event, not from raw engine internals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub execution_id: String,
    pub timestamp: DateTime<Utc>,
    pub versions: ExecutionEventVersions,
    pub policy: ExecutionPolicy,
    pub policy_hash: String,
    pub request: ExecutionRequest,
    pub features: IntradayFeatures,
    pub observations: Vec<IntradayObservation>,
    pub evidences: Vec<Evidence>,
    pub assessment: ExecutionAssessment,
    pub decision: ExecutionDecision,
}

impl ExecutionEvent {
    /// Constructs an ExecutionEvent from the final pipeline components.
    ///
    /// The caller is responsible for running the pipeline stages in order. This
    /// constructor packages the provenance into the canonical event and computes
    /// version/hashing metadata for long-term replay.
    pub fn new(
        request: ExecutionRequest,
        features: IntradayFeatures,
        observations: Vec<IntradayObservation>,
        evidences: Vec<Evidence>,
        assessment: ExecutionAssessment,
        decision: ExecutionDecision,
    ) -> Self {
        let policy_version = policy_hash(&request.policy);
        let research_version = request.market_view.research_version.clone();

        Self {
            execution_id: format!("EE-{}", Uuid::new_v4().to_string().split('-').next().unwrap()),
            timestamp: Utc::now(),
            versions: ExecutionEventVersions {
                schema_version: "v2.1".into(),
                engine_version: "v2.0.0-mvp".into(),
                policy_version,
                research_version,
            },
            policy: request.policy.clone(),
            policy_hash: policy_hash(&request.policy),
            request,
            features,
            observations,
            evidences,
            assessment,
            decision,
        }
    }

    /// Returns the symbol this event was evaluated for.
    pub fn symbol(&self) -> &str {
        &self.request.symbol
    }

    /// Returns the analysis date.
    pub fn date(&self) -> NaiveDate {
        self.request.date
    }
}

fn policy_hash(policy: &ExecutionPolicy) -> String {
    let json = serde_json::to_string(policy).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use research_context::{
        BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
    };

    use crate::v2::assessment::RiskLevel;
    use crate::v2::evidence::{EvidenceKind, EvidencePayload, EvidenceSource};
    use crate::v2::request::{ExecutionMarketView, ExecutionPolicy, QuoteSnapshot};

    fn make_minimal_event() -> ExecutionEvent {
        let request = ExecutionRequest {
            symbol: "000001".into(),
            date: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
            signal: core_domain::SignalSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                symbol: "000001".into(),
                final_score: 82.0,
                signal_label: SignalLabel::Buy,
                analysis_scope: "CN".into(),
                regime_basis_scope: "CN".into(),
                reason: core_domain::SignalReason {
                    best_strategy: StrategyKind::MomentumRight,
                    strategy_score: 0.0,
                    strategy_contribution: 0.0,
                    alignment: 0,
                    aligned_strategies: vec![],
                    alignment_contribution: 0.0,
                    regime: core_domain::RegimeReason {
                        trend_score: 0.0,
                        risk_score: 0.0,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    rotation: core_domain::RotationReason {
                        momentum_score: 0.0,
                        rank: None,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    final_score: 82.0,
                    label: SignalLabel::Buy,
                    summary: "test".into(),
                },
            },
            strategy_state: core_domain::StrategyStateSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 7, 17).unwrap(),
                scope: "CN".into(),
                state: StrategyState::ConfirmAdd,
                state_score: 60.0,
                transition_reason: "test".into(),
                recommended_position_pct: 60.0,
            },
            quote: QuoteSnapshot {
                symbol: "000001".into(),
                ts: Utc::now(),
                open: 10.0,
                high: 11.0,
                low: 9.5,
                close: 10.5,
                volume: 1_000_000.0,
                prev_close: 10.0,
            },
            volume_ma20: 500_000.0,
            market_view: ExecutionMarketView {
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
                    overall: "Strong".into(),
                },
                breadth: BreadthSummary {
                    breadth_pct: 55.0,
                    sma5: None,
                    delta_5d: Some(0.0),
                    condition: "strong".into(),
                },
                recovery: RecoverySummary {
                    score: 55.0,
                    drivers: vec![],
                },
                rotation_state: "broad".into(),
                leadership_stability: 0.7,
            },
            policy: ExecutionPolicy::default(),
        };

        let features = crate::v2::feature::IntradayFeatures {
            symbol: "000001".into(),
            today_return: 0.05,
            open_return: 0.0,
            gap_pct: 0.0,
            close_position: 0.8,
            amplitude_pct: 0.15,
            upper_shadow_pct: 0.0,
            lower_shadow_pct: 0.0,
            volume_ratio: 2.0,
            body_ratio: 0.3,
            gap_fill_ratio: 0.0,
        };

        let observations = vec![];

        let evidences = vec![crate::v2::evidence::Evidence {
            kind: EvidenceKind::SignalStrength,
            confidence: 0.82,
            direction: 1.0,
            source: EvidenceSource::SignalModel,
            payload: EvidencePayload::Signal {
                final_score: 82.0,
                signal_label: "Buy".into(),
            },
        }];

        let assessment = ExecutionAssessment {
            confidence: 0.82,
            consensus: 1.0,
            coverage: 1.0,
            risk: RiskLevel::Low,
            dominant_direction: 1.0,
            supporting_evidence: evidences.clone(),
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };

        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: crate::types::ExecutionState::Increase,
            confidence: 0.82,
            risk: RiskLevel::Low,
            evidences: evidences.clone(),
            assessment: assessment.clone(),
            decision_reasons: vec![crate::v2::decision::DecisionReason::PositiveConsensus],
        };

        ExecutionEvent::new(request, features, observations, evidences, assessment, decision)
    }

    #[test]
    fn event_has_id_and_versions() {
        let e = make_minimal_event();
        assert!(e.execution_id.starts_with("EE-"));
        assert_eq!(e.versions.schema_version, "v2.1");
        assert_eq!(e.versions.engine_version, "v2.0.0-mvp");
        assert_eq!(e.versions.research_version, "1");
        assert!(!e.versions.policy_version.is_empty());
    }

    #[test]
    fn event_carries_policy_and_hash() {
        let e = make_minimal_event();
        assert!(!e.policy_hash.is_empty());
        assert_eq!(e.versions.policy_version, e.policy_hash);
    }

    #[test]
    fn event_symbol_and_date_accessors() {
        let e = make_minimal_event();
        assert_eq!(e.symbol(), "000001");
        assert_eq!(e.date().to_string(), "2026-07-17");
    }

    #[test]
    fn event_is_serializable() {
        let e = make_minimal_event();
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("execution_id"));
        assert!(json.contains("EE-"));
    }

    #[test]
    fn policy_hash_is_stable_for_same_policy() {
        let p1 = ExecutionPolicy::default();
        let p2 = ExecutionPolicy::default();
        assert_eq!(policy_hash(&p1), policy_hash(&p2));
    }
}

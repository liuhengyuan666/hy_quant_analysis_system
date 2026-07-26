use serde::{Deserialize, Serialize};

use crate::types::ExecutionState;
use crate::v2::assessment::{ExecutionAssessment, RiskLevel};
use crate::v2::evidence::Evidence;
use crate::v2::request::ExecutionPolicy;

/// Final execution decision for a single symbol.
///
/// This is a Consumer-facing DTO. It exposes the decided state, confidence,
/// risk, and the evidence/assessment that led to it. It does not expose any
/// internal score from the AssessmentEngine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDecision {
    pub symbol: String,
    pub state: ExecutionState,
    pub confidence: f64,
    pub risk: RiskLevel,
    pub evidences: Vec<Evidence>,
    pub assessment: ExecutionAssessment,
    pub decision_reasons: Vec<DecisionReason>,
}

/// Machine-readable reason why the DecisionEngine chose a particular state.
///
/// Reasons are derived from the Assessment and Policy, not from formatter text.
/// They are intended for Replay, Desktop UI, and LLM Explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisionReason {
    ConfidenceBelowThreshold,
    ConsensusBelowThreshold,
    CriticalRisk,
    RiskTooHigh,
    WeakDirection,
    PositiveConsensus,
    NegativeConsensus,
}

/// Maps an ExecutionAssessment into an ExecutionState.
///
/// The DecisionEngine does not reason about the market. It only applies the
/// policy thresholds to the already-fused assessment.
pub trait DecisionEngine {
    fn decide(
        &self,
        symbol: &str,
        assessment: &ExecutionAssessment,
        policy: &ExecutionPolicy,
    ) -> ExecutionDecision;
}

/// Default decision engine applying the MVP policy rules.
#[derive(Debug, Clone, Default)]
pub struct DefaultDecisionEngine;

impl DecisionEngine for DefaultDecisionEngine {
    fn decide(
        &self,
        symbol: &str,
        assessment: &ExecutionAssessment,
        policy: &ExecutionPolicy,
    ) -> ExecutionDecision {
        let (state, reasons) = if assessment.risk == RiskLevel::Critical {
            (ExecutionState::Maintain, vec![DecisionReason::CriticalRisk])
        } else if assessment.risk == RiskLevel::High {
            (ExecutionState::Maintain, vec![DecisionReason::RiskTooHigh])
        } else if assessment.confidence < policy.confidence_threshold {
            (
                ExecutionState::Maintain,
                vec![DecisionReason::ConfidenceBelowThreshold],
            )
        } else if assessment.consensus < policy.consensus_threshold {
            (
                ExecutionState::Maintain,
                vec![DecisionReason::ConsensusBelowThreshold],
            )
        } else if assessment.dominant_direction > policy.buy_threshold {
            (ExecutionState::Increase, vec![DecisionReason::PositiveConsensus])
        } else if assessment.dominant_direction < policy.reduce_threshold {
            (ExecutionState::Reduce, vec![DecisionReason::NegativeConsensus])
        } else {
            (ExecutionState::Maintain, vec![DecisionReason::WeakDirection])
        };

        ExecutionDecision {
            symbol: symbol.into(),
            state,
            confidence: assessment.confidence,
            risk: assessment.risk,
            evidences: assessment
                .supporting_evidence
                .iter()
                .chain(assessment.conflicting_evidence.iter())
                .chain(assessment.neutral_evidence.iter())
                .cloned()
                .collect(),
            assessment: assessment.clone(),
            decision_reasons: reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::assessment::RiskLevel;

    fn make_assessment(
        confidence: f64,
        consensus: f64,
        risk: RiskLevel,
        direction: f64,
    ) -> ExecutionAssessment {
        ExecutionAssessment {
            confidence,
            consensus,
            coverage: 1.0,
            risk,
            dominant_direction: direction,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        }
    }

    fn policy() -> ExecutionPolicy {
        ExecutionPolicy::default()
    }

    #[test]
    fn bullish_assessment_becomes_increase() {
        let a = make_assessment(0.8, 0.8, RiskLevel::Low, 0.7);
        let d = DefaultDecisionEngine.decide("000001", &a, &policy());

        assert_eq!(d.state, ExecutionState::Increase);
        assert!(d
            .decision_reasons
            .contains(&DecisionReason::PositiveConsensus));
    }

    #[test]
    fn bearish_assessment_becomes_reduce() {
        let a = make_assessment(0.8, 0.8, RiskLevel::Medium, -0.5);
        let d = DefaultDecisionEngine.decide("000001", &a, &policy());

        assert_eq!(d.state, ExecutionState::Reduce);
        assert!(d
            .decision_reasons
            .contains(&DecisionReason::NegativeConsensus));
    }

    #[test]
    fn critical_risk_always_maintain() {
        let a = make_assessment(0.9, 0.9, RiskLevel::Critical, 0.8);
        let d = DefaultDecisionEngine.decide("000001", &a, &policy());

        assert_eq!(d.state, ExecutionState::Maintain);
        assert!(d.decision_reasons.contains(&DecisionReason::CriticalRisk));
    }

    #[test]
    fn low_confidence_maintain() {
        let a = make_assessment(0.3, 0.8, RiskLevel::Low, 0.7);
        let d = DefaultDecisionEngine.decide("000001", &a, &policy());

        assert_eq!(d.state, ExecutionState::Maintain);
        assert!(d
            .decision_reasons
            .contains(&DecisionReason::ConfidenceBelowThreshold));
    }

    #[test]
    fn weak_direction_maintain() {
        let a = make_assessment(0.8, 0.8, RiskLevel::Low, 0.2);
        let d = DefaultDecisionEngine.decide("000001", &a, &policy());

        assert_eq!(d.state, ExecutionState::Maintain);
        assert!(d
            .decision_reasons
            .contains(&DecisionReason::WeakDirection));
    }
}

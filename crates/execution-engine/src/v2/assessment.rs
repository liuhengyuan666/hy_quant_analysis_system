use serde::{Deserialize, Serialize};

use crate::v2::evidence::{Evidence, EvidenceKind};
use crate::v2::request::ExecutionPolicy;

/// Result of fusing multiple pieces of Evidence into a single market assessment.
///
/// Assessment is an analysis, not a decision. It answers: "What is the overall
/// evidence saying?" It does not answer "Should we buy?" — that is the job of
/// the `DecisionEngine`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionAssessment {
    /// Overall confidence in the assessment, in [0.0, 1.0].
    pub confidence: f64,
    /// Consensus among evidence directions, in [0.0, 1.0]. High consensus means
    /// most evidence points the same way; low consensus means evidence is split.
    pub consensus: f64,
    /// Coverage of the expected evidence categories, in [0.0, 1.0]. Low
    /// coverage means important evidence layers are missing.
    pub coverage: f64,
    pub risk: RiskLevel,
    /// Dominant direction: +1.0 bullish, -1.0 bearish, 0.0 neutral.
    pub dominant_direction: f64,
    pub supporting_evidence: Vec<Evidence>,
    pub conflicting_evidence: Vec<Evidence>,
    pub neutral_evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Fuses Evidence into an ExecutionAssessment.
///
/// This is the only component in the Execution Pipeline that is allowed to
/// combine multiple Evidence items into a single judgment. All other layers
/// (Observation, EvidenceBuilder) must only produce or transform individual
/// Evidence items.
pub trait AssessmentEngine {
    fn assess(&self, evidences: &[Evidence], policy: &ExecutionPolicy) -> ExecutionAssessment;
}

/// Default assessment engine using equal-weight evidence fusion.
///
/// The algorithm is intentionally simple for the MVP. It is a baseline, not a
/// final model. Future engines may use Bayesian updating, rule-based scoring,
/// Research Asset calibration, or ML ranking — as long as they implement the
/// `AssessmentEngine` trait and produce the same `ExecutionAssessment` contract.
#[derive(Debug, Clone, Default)]
pub struct EqualWeightAssessmentEngine;

impl AssessmentEngine for EqualWeightAssessmentEngine {
    fn assess(&self, evidences: &[Evidence], policy: &ExecutionPolicy) -> ExecutionAssessment {
        if evidences.is_empty() {
            return ExecutionAssessment {
                confidence: 0.0,
                consensus: 0.0,
                coverage: 0.0,
                risk: RiskLevel::Medium,
                dominant_direction: 0.0,
                supporting_evidence: vec![],
                conflicting_evidence: vec![],
                neutral_evidence: vec![],
            };
        }

        let total_confidence: f64 = evidences.iter().map(|e| e.confidence).sum();
        let n = evidences.len() as f64;

        // Direction-weighted average, normalized by total confidence.
        let dominant_direction = if total_confidence > 0.0 {
            evidences
                .iter()
                .map(|e| e.confidence * e.direction)
                .sum::<f64>()
                / total_confidence
        } else {
            0.0
        };

        // Average confidence across all evidence.
        let confidence = total_confidence / n;

        // Consensus: 1 - normalized standard deviation of signed confidence.
        let weighted_values: Vec<f64> = evidences
            .iter()
            .map(|e| e.confidence * e.direction)
            .collect();
        let mean = weighted_values.iter().sum::<f64>() / n;
        let variance = weighted_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();
        let max_possible_std_dev = 1.0; // signed confidence is in [-1.0, 1.0]
        let consensus = (1.0 - std_dev / max_possible_std_dev).clamp(0.0, 1.0);

        // Coverage: ratio of distinct evidence sources present.
        let expected_sources = 4; // IntradayObservation, ResearchContext, StrategyState, SignalModel
        let present_sources = evidences
            .iter()
            .map(|e| e.source)
            .collect::<std::collections::HashSet<_>>()
            .len() as f64;
        let coverage = (present_sources / expected_sources as f64).clamp(0.0, 1.0);

        // Classify evidence by direction.
        let (supporting, conflicting, neutral): (Vec<_>, Vec<_>, Vec<_>) = evidences
            .iter()
            .cloned()
            .fold((vec![], vec![], vec![]), |(mut sup, mut con, mut neu), e| {
                if e.direction > 0.1 {
                    sup.push(e);
                } else if e.direction < -0.1 {
                    con.push(e);
                } else {
                    neu.push(e);
                }
                (sup, con, neu)
            });

        // Risk is computed independently from direction.
        let risk_score = compute_risk_score(evidences, &supporting, &conflicting, &neutral);
        let risk = classify_risk(risk_score, policy);

        ExecutionAssessment {
            confidence: confidence.clamp(0.0, 1.0),
            consensus,
            coverage,
            risk,
            dominant_direction: dominant_direction.clamp(-1.0, 1.0),
            supporting_evidence: supporting,
            conflicting_evidence: conflicting,
            neutral_evidence: neutral,
        }
    }
}

fn compute_risk_score(
    evidences: &[Evidence],
    supporting: &[Evidence],
    conflicting: &[Evidence],
    neutral: &[Evidence],
) -> f64 {
    // Risk evidence: direct market risks.
    let risk_kinds = [
        EvidenceKind::Distribution,
        EvidenceKind::MomentumFailure,
        EvidenceKind::RiskExpansion,
        EvidenceKind::LiquidityConfirmation,
    ];
    let risk_confidence: f64 = evidences
        .iter()
        .filter(|e| risk_kinds.contains(&e.kind))
        .map(|e| e.confidence)
        .sum();

    // Conflict premium: when evidence is split, risk rises.
    let total_directional = (supporting.len() + conflicting.len()) as f64;
    let conflict_ratio = if total_directional > 0.0 {
        (conflicting.len() as f64 / total_directional).min(0.5) * 2.0 // scale to [0, 1]
    } else {
        0.0
    };

    // Low coverage premium: missing evidence layers increase uncertainty.
    let total = evidences.len() as f64;
    let coverage_ratio = if total > 0.0 {
        (neutral.len() as f64 / total).min(0.5) * 2.0
    } else {
        0.0
    };

    // Combine: risk evidence dominates, conflict and coverage add premium.
    (risk_confidence * 0.5 + conflict_ratio * 0.25 + coverage_ratio * 0.25).clamp(0.0, 1.0)
}

fn classify_risk(score: f64, policy: &ExecutionPolicy) -> RiskLevel {
    if score >= policy.risk_threshold_high {
        RiskLevel::High
    } else if score >= policy.risk_threshold_low {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::evidence::EvidenceSource;

    fn make_evidence(kind: EvidenceKind, confidence: f64, direction: f64) -> Evidence {
        Evidence {
            kind,
            confidence,
            direction,
            source: EvidenceSource::IntradayObservation,
            payload: crate::v2::evidence::EvidencePayload::Empty,
        }
    }

    fn policy() -> ExecutionPolicy {
        ExecutionPolicy::default()
    }

    #[test]
    fn strong_bullish_assessment() {
        let evidences = vec![
            make_evidence(EvidenceKind::MarketAcceptance, 0.8, 1.0),
            make_evidence(EvidenceKind::MomentumExpansion, 0.7, 1.0),
            make_evidence(EvidenceKind::TrendParticipation, 0.6, 1.0),
        ];

        let a = EqualWeightAssessmentEngine.assess(&evidences, &policy());

        assert!(a.dominant_direction > 0.9);
        assert!(a.confidence > 0.6);
        assert!(a.consensus > 0.8);
        assert_eq!(a.risk, RiskLevel::Low);
    }

    #[test]
    fn split_evidence_increases_risk() {
        let evidences = vec![
            make_evidence(EvidenceKind::MarketAcceptance, 0.8, 1.0),
            make_evidence(EvidenceKind::Distribution, 0.8, -1.0),
        ];

        let a = EqualWeightAssessmentEngine.assess(&evidences, &policy());

        assert!(a.dominant_direction.abs() < 0.1);
        assert!(a.confidence > 0.7);
        assert!(a.consensus < 0.5);
        assert!(matches!(a.risk, RiskLevel::High | RiskLevel::Medium));
    }

    #[test]
    fn risk_evidences_increase_risk_independent_of_direction() {
        // Bullish evidence stronger than risk evidence, but risk evidence still
        // elevates the overall risk assessment.
        let evidences = vec![
            make_evidence(EvidenceKind::MomentumExpansion, 0.9, 1.0),
            make_evidence(EvidenceKind::RiskExpansion, 0.5, -1.0),
        ];

        let a = EqualWeightAssessmentEngine.assess(&evidences, &policy());

        assert!(a.dominant_direction > 0.0); // still bullish
        assert!(matches!(a.risk, RiskLevel::High | RiskLevel::Medium));
    }

    #[test]
    fn no_evidence_assessment_is_neutral() {
        let a = EqualWeightAssessmentEngine.assess(&[], &policy());

        assert_eq!(a.confidence, 0.0);
        assert_eq!(a.consensus, 0.0);
        assert_eq!(a.coverage, 0.0);
        assert_eq!(a.dominant_direction, 0.0);
    }
}

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    confirmation_decay::detect_confirmation_decay_v4,
    holding_risk_bundle::detect_liquidity_pressure_v3,
    holding_risk_calibration::compute_holding_risk_score,
    risk_lifecycle::compute_risk_lifecycle_analysis,
    transition_analysis::detect_leadership_decay,
    ExecutionResearchRecord,
};

/// TASK-169: Shadow Deployment Contract.
///
/// This module defines the formal boundary for Phase 2C Shadow Validation.
/// It consumes real `ResearchContext` (via `ExecutionResearchRecord`) and emits
/// `ShadowRiskAssessment`. It is explicitly forbidden for `DecisionEngine` to
/// consume `ShadowRiskAssessment`; this type is intended for observation and
/// reporting only.
///
/// The contract is:
/// - Input: real market data through ResearchContext
/// - Output: `ShadowRiskAssessment` (observation-only)
/// - Prohibition: `DecisionEngine` must NOT consume this output
///
/// No changes to ObservationEngine, EvidenceBuilder, AssessmentEngine,
/// DecisionEngine, or ExecutionPolicy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowDeploymentReport {
    pub generated_at: DateTime<Utc>,
    pub scope: String,
    pub contract_version: String,
    pub assessments: Vec<ShadowRiskAssessment>,
    pub summary: ShadowDeploymentSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowRiskAssessment {
    pub date: NaiveDate,
    pub regime: String,
    pub holding_risk_score: f64,
    pub evidence: EvidenceSummary,
    pub lifecycle_state: String,
    pub research_interpretation: String,
    pub decision_engine_consumption_allowed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub leadership_decay_persistence: bool,
    pub liquidity_pressure: bool,
    pub confirmation_decay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowDeploymentSummary {
    pub total_days: usize,
    pub high_risk_days: usize,
    pub elevated_risk_days: usize,
    pub normal_days: usize,
    pub transition_detected_days: usize,
    pub avg_holding_risk_score: f64,
    pub lifecycle_events: usize,
    pub false_alarms: usize,
    pub validation_status: ShadowValidationStatus,
}

/// Shadow Validation monitoring state.
///
/// TASK-172: Tracks whether the Shadow Validation phase is producing enough
/// Transition Detection events to be statistically meaningful. This is NOT a
/// failure state; it is a data sufficiency indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShadowValidationStatus {
    Normal,
    InsufficientEvents,
    Active,
}

/// Computes the Shadow Deployment report for a set of records.
///
/// This is the formal Phase 2C entry point. It is read-only and does not
/// modify any Execution Pipeline component.
pub fn compute_shadow_deployment_report(
    records: &[ExecutionResearchRecord],
    scope: &str,
) -> ShadowDeploymentReport {
    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let lifecycle_analysis = compute_risk_lifecycle_analysis(records);
    let lifecycle_events = lifecycle_analysis.events.len();
    let false_alarms = lifecycle_analysis
        .events
        .iter()
        .filter(|e| e.is_false_alarm)
        .count();

    let mut assessments = Vec::new();
    let mut date_map: BTreeMap<NaiveDate, Vec<&ExecutionResearchRecord>> = BTreeMap::new();
    for r in records {
        date_map.entry(r.event.date()).or_default().push(r);
    }

    for (date, _date_records) in date_map {
        let mut scores = Vec::new();
        let mut regime_labels = Vec::new();
        let mut evidence = EvidenceSummary::default();

        for (_symbol, by_date) in &by_symbol {
            if let Some(record) = by_date.get(&date) {
                let score = compute_holding_risk_score(record, date, by_date);
                scores.push(score);
                regime_labels.push(record.event.request.market_view.market_regime_label.clone());

                let leadership = detect_leadership_decay(record, date, by_date);
                if leadership.is_leadership_decay() && leadership.consecutive_decline_days >= 5 {
                    evidence.leadership_decay_persistence = true;
                }
                if detect_liquidity_pressure_v3(record, date, by_date) {
                    evidence.liquidity_pressure = true;
                }
                if detect_confirmation_decay_v4(record, date, by_date) {
                    evidence.confirmation_decay = true;
                }
            }
        }

        let avg_score = if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        };
        let regime = regime_labels
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".into());

        let transition_detected = avg_score >= 0.75;
        let lifecycle_state: String = if regime == "risk_off" {
            "HIGH_RISK".into()
        } else if transition_detected {
            "HIGH_RISK".into()
        } else if avg_score >= 0.5 || regime == "neutral" {
            "ELEVATED_RISK".into()
        } else {
            "NORMAL".into()
        };

        let research_interpretation = match lifecycle_state.as_str() {
            "HIGH_RISK" => "monitor_risk_transition".into(),
            "ELEVATED_RISK" => "observe_market_structure".into(),
            _ => "normal_conditions".into(),
        };

        assessments.push(ShadowRiskAssessment {
            date,
            regime,
            holding_risk_score: avg_score,
            evidence,
            lifecycle_state,
            research_interpretation,
            decision_engine_consumption_allowed: false,
        });
    }

    let summary = compute_summary(&assessments, lifecycle_events, false_alarms);

    ShadowDeploymentReport {
        generated_at: Utc::now(),
        scope: scope.to_string(),
        contract_version: "v2c.1.0".into(),
        assessments,
        summary,
    }
}

fn compute_summary(
    assessments: &[ShadowRiskAssessment],
    lifecycle_events: usize,
    false_alarms: usize,
) -> ShadowDeploymentSummary {
    let total_days = assessments.len();
    let high_risk_days = assessments
        .iter()
        .filter(|a| a.lifecycle_state == "HIGH_RISK")
        .count();
    let elevated_risk_days = assessments
        .iter()
        .filter(|a| a.lifecycle_state == "ELEVATED_RISK")
        .count();
    let normal_days = assessments
        .iter()
        .filter(|a| a.lifecycle_state == "NORMAL")
        .count();
    let transition_detected_days = assessments
        .iter()
        .filter(|a| a.holding_risk_score >= 0.75)
        .count();
    let avg_score = if total_days == 0 {
        0.0
    } else {
        assessments
            .iter()
            .map(|a| a.holding_risk_score)
            .sum::<f64>()
            / total_days as f64
    };

    // TASK-172: Determine validation status based on event frequency.
    // If total_days >= 20 and transition_detected_days == 0, enter INSUFFICIENT_EVENTS.
    let validation_status = if total_days >= 20 && transition_detected_days == 0 {
        ShadowValidationStatus::InsufficientEvents
    } else if total_days > 0 {
        ShadowValidationStatus::Active
    } else {
        ShadowValidationStatus::Normal
    };

    ShadowDeploymentSummary {
        total_days,
        high_risk_days,
        elevated_risk_days,
        normal_days,
        transition_detected_days,
        avg_holding_risk_score: avg_score,
        lifecycle_events,
        false_alarms,
        validation_status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_on_empty_assessments() {
        let summary = compute_summary(&[], 0, 0);
        assert_eq!(summary.total_days, 0);
        assert_eq!(summary.high_risk_days, 0);
    }
}

use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    confirmation_decay::detect_confirmation_decay_v4,
    holding_risk_bundle::detect_liquidity_pressure_v3,
    holding_risk_calibration::compute_holding_risk_score,
    transition_analysis::detect_leadership_decay,
    ExecutionResearchRecord,
};

/// TASK-167: Shadow Mode Runtime Wiring.
///
/// Generates daily shadow-mode output by combining:
/// - State Context: `market_regime_label` (already available from ResearchContext)
/// - Transition Evidence: `HoldingRiskScore` (validated in TASK-161)
///
/// This is a read-only bypass. It does not modify the Execution Pipeline,
/// ObservationEngine, EvidenceBuilder, AssessmentEngine, DecisionEngine, or
/// ExecutionPolicy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowModeReport {
    pub generated_at: DateTime<Utc>,
    pub scope: String,
    pub outputs: Vec<ShadowModeOutput>,
    pub summary: ShadowModeSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowModeOutput {
    pub date: NaiveDate,
    pub market_regime: String,
    pub holding_risk_score: f64,
    pub risk_state: String,
    pub transition_detected: bool,
    pub decision_candidate: String,
    pub evidence_details: EvidenceDetails,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceDetails {
    pub leadership_decay_persistence: bool,
    pub liquidity_pressure: bool,
    pub confirmation_decay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowModeSummary {
    pub total_days: usize,
    pub high_risk_days: usize,
    pub elevated_risk_days: usize,
    pub normal_days: usize,
    pub transition_detected_days: usize,
    pub avg_holding_risk_score: f64,
}

/// Computes the Shadow Mode report for a set of records.
pub fn compute_shadow_mode_report(
    records: &[ExecutionResearchRecord],
    scope: &str,
) -> ShadowModeReport {
    let mut by_symbol: BTreeMap<String, BTreeMap<NaiveDate, &ExecutionResearchRecord>> =
        BTreeMap::new();
    for r in records {
        by_symbol
            .entry(r.event.symbol().to_string())
            .or_default()
            .insert(r.event.date(), r);
    }

    let mut outputs = Vec::new();
    let mut date_map: BTreeMap<NaiveDate, Vec<&ExecutionResearchRecord>> = BTreeMap::new();
    for r in records {
        date_map.entry(r.event.date()).or_default().push(r);
    }

    for (date, _date_records) in date_map {
        let mut scores = Vec::new();
        let mut regime_labels = Vec::new();
        let mut evidence_details = EvidenceDetails::default();

        for (_symbol, by_date) in &by_symbol {
            if let Some(record) = by_date.get(&date) {
                let score = compute_holding_risk_score(record, date, by_date);
                scores.push(score);
                regime_labels.push(record.event.request.market_view.market_regime_label.clone());

                let leadership = detect_leadership_decay(record, date, by_date);
                if leadership.is_leadership_decay() && leadership.consecutive_decline_days >= 5 {
                    evidence_details.leadership_decay_persistence = true;
                }
                if detect_liquidity_pressure_v3(record, date, by_date) {
                    evidence_details.liquidity_pressure = true;
                }
                if detect_confirmation_decay_v4(record, date, by_date) {
                    evidence_details.confirmation_decay = true;
                }
            }
        }

        let avg_score = if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        };
        let market_regime = regime_labels
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".into());

        let transition_detected = avg_score >= 0.75;
        let risk_state: String = if market_regime == "risk_off" {
            "HIGH_RISK".into()
        } else if transition_detected {
            "HIGH_RISK".into()
        } else if avg_score >= 0.5 || market_regime == "neutral" {
            "ELEVATED_RISK".into()
        } else {
            "NORMAL".into()
        };

        let decision_candidate = match risk_state.as_str() {
            "HIGH_RISK" => "reduce_watch".into(),
            "ELEVATED_RISK" => "monitor".into(),
            _ => "hold".into(),
        };

        outputs.push(ShadowModeOutput {
            date,
            market_regime,
            holding_risk_score: avg_score,
            risk_state,
            transition_detected,
            decision_candidate,
            evidence_details,
        });
    }

    let summary = compute_summary(&outputs);

    ShadowModeReport {
        generated_at: Utc::now(),
        scope: scope.to_string(),
        outputs,
        summary,
    }
}

fn compute_summary(outputs: &[ShadowModeOutput]) -> ShadowModeSummary {
    let total_days = outputs.len();
    let high_risk_days = outputs.iter().filter(|o| o.risk_state == "HIGH_RISK").count();
    let elevated_risk_days = outputs
        .iter()
        .filter(|o| o.risk_state == "ELEVATED_RISK")
        .count();
    let normal_days = outputs.iter().filter(|o| o.risk_state == "NORMAL").count();
    let transition_detected_days = outputs.iter().filter(|o| o.transition_detected).count();
    let avg_score = if total_days == 0 {
        0.0
    } else {
        outputs.iter().map(|o| o.holding_risk_score).sum::<f64>() / total_days as f64
    };

    ShadowModeSummary {
        total_days,
        high_risk_days,
        elevated_risk_days,
        normal_days,
        transition_detected_days,
        avg_holding_risk_score: avg_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_on_empty_outputs() {
        let summary = compute_summary(&[]);
        assert_eq!(summary.total_days, 0);
        assert_eq!(summary.high_risk_days, 0);
    }
}

use execution_engine::v2::evidence::{Evidence, EvidencePayload};

use crate::ExecutionResearchRecord;

/// Formatter for validation output of `ExecutionResearchRecord`.
///
/// This is not a presentation formatter for end users. It is a debugging and
/// validation tool for engineers reviewing the Execution Platform. It formats
/// the complete record (Event + Outcome + Evaluation) into several readable
/// forms without leaking CLI-specific formatting into the domain crate.
#[derive(Debug, Clone, Default)]
pub struct ValidationFormatter;

impl ValidationFormatter {
    pub fn format_json(&self, record: &ExecutionResearchRecord) -> String {
        serde_json::to_string_pretty(record).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }

    pub fn format_explain(&self, record: &ExecutionResearchRecord) -> String {
        let event = &record.event;
        let decision = &event.decision;
        let outcome = &record.outcome;

        let mut lines = Vec::new();
        lines.push("=".repeat(50));
        lines.push("Execution Replay Validation".to_string());
        lines.push("=".repeat(50));
        lines.push(String::new());

        lines.push(format!("Symbol:          {}", event.symbol()));
        lines.push(format!("Date:            {}", event.date()));
        lines.push(format!("Execution ID:    {}", event.execution_id));
        lines.push(format!("Schema Version:  {}", event.versions.schema_version));
        lines.push(format!("Engine Version:  {}", event.versions.engine_version));
        lines.push(format!("Policy Version:  {}", event.versions.policy_version));
        lines.push(format!("Research Version: {}", event.versions.research_version));
        lines.push(String::new());

        lines.push("--- Decision Context ---".to_string());
        lines.push(format!("Market Regime:   {}", event.request.market_view.market_regime_label));
        lines.push(format!("Strategy State:  {:?}", event.request.strategy_state.state));
        lines.push(format!("Signal:          {:?} ({:.1})", event.request.signal.signal_label, event.request.signal.final_score));
        lines.push(format!("Decision:        {:?}", decision.state));
        lines.push(format!("Confidence:      {:.2}", decision.confidence));
        lines.push(format!("Risk:            {:?}", decision.risk));
        lines.push(String::new());

        lines.push("--- Supporting Evidence ---".to_string());
        for ev in &decision.evidences {
            if ev.direction > 0.0 {
                lines.push(format!("+ {} ({:.2})", evidence_summary(ev), ev.confidence));
            }
        }

        lines.push(String::new());
        lines.push("--- Conflicting Evidence ---".to_string());
        for ev in &decision.evidences {
            if ev.direction < 0.0 {
                lines.push(format!("- {} ({:.2})", evidence_summary(ev), ev.confidence));
            }
        }

        lines.push(String::new());
        lines.push("--- Outcome ---".to_string());
        if let Some(r) = outcome.t20_return {
            lines.push(format!("T+20 Return:     {:.2}%", r * 100.0));
        } else {
            lines.push("T+20 Return:     n/a".to_string());
        }
        if let Some(r) = outcome.t60_return {
            lines.push(format!("T+60 Return:     {:.2}%", r * 100.0));
        } else {
            lines.push("T+60 Return:     n/a".to_string());
        }
        if let Some(r) = outcome.t120_return {
            lines.push(format!("T+120 Return:    {:.2}%", r * 100.0));
        } else {
            lines.push("T+120 Return:    n/a".to_string());
        }
        if let Some(mfe) = outcome.mfe {
            lines.push(format!("MFE:             {:.2}%", mfe * 100.0));
        }
        if let Some(mae) = outcome.mae {
            lines.push(format!("MAE:             {:.2}%", mae * 100.0));
        }
        if let Some(dd) = outcome.max_drawdown {
            lines.push(format!("Max Drawdown:    {:.2}%", dd * 100.0));
        }

        lines.push(String::new());
        lines.push("--- Evaluation ---".to_string());
        lines.push(format!("Label:           {:?}", record.evaluation));
        lines.push(format!("Eval Version:    {}", record.evaluation_version));
        lines.push(format!("Evaluated At:    {}", record.evaluated_at));

        lines.join("\n")
    }

    pub fn format_trace(&self, record: &ExecutionResearchRecord) -> String {
        let event = &record.event;
        let mut lines = Vec::new();

        lines.push("Pipeline Trace".to_string());
        lines.push("=".repeat(50));
        lines.push(String::new());

        lines.push("Quote".to_string());
        lines.push(format!(
            "  open={:.2} high={:.2} low={:.2} close={:.2} volume={:.0}",
            event.request.quote.open,
            event.request.quote.high,
            event.request.quote.low,
            event.request.quote.close,
            event.request.quote.volume
        ));
        lines.push(String::new());

        lines.push("Feature".to_string());
        let f = &event.features;
        lines.push(format!("  today_return={:.2}%", f.today_return * 100.0));
        lines.push(format!("  gap_pct={:.2}%", f.gap_pct * 100.0));
        lines.push(format!("  close_position={:.2}", f.close_position));
        lines.push(format!("  volume_ratio={:.2}", f.volume_ratio));
        lines.push(format!("  amplitude_pct={:.2}%", f.amplitude_pct * 100.0));
        lines.push(String::new());

        lines.push("Observation".to_string());
        for obs in &event.observations {
            lines.push(format!("  {:?}", obs.kind));
        }
        lines.push(String::new());

        lines.push("Evidence".to_string());
        for ev in &event.evidences {
            lines.push(format!(
                "  {} direction={:.0} confidence={:.2}",
                evidence_summary(ev),
                ev.direction.signum(),
                ev.confidence
            ));
        }
        lines.push(String::new());

        lines.push("Assessment".to_string());
        lines.push(format!("  confidence={:.2}", event.assessment.confidence));
        lines.push(format!("  consensus={:.2}", event.assessment.consensus));
        lines.push(format!("  coverage={:.2}", event.assessment.coverage));
        lines.push(format!("  risk={:?}", event.assessment.risk));
        lines.push(format!("  dominant_direction={:.0}", event.assessment.dominant_direction.signum()));
        lines.push(String::new());

        lines.push("Decision".to_string());
        lines.push(format!("  state={:?}", event.decision.state));
        lines.push(format!("  confidence={:.2}", event.decision.confidence));
        lines.push(format!("  risk={:?}", event.decision.risk));
        lines.push(format!("  reasons={:?}", event.decision.decision_reasons));
        lines.push(String::new());

        lines.push("Outcome".to_string());
        lines.push(format!(
            "  t20={:?} t60={:?} t120={:?}",
            record.outcome.t20_return, record.outcome.t60_return, record.outcome.t120_return
        ));
        lines.push(format!("  evaluation={:?}", record.evaluation));

        lines.join("\n")
    }

    pub fn format_markdown(&self, record: &ExecutionResearchRecord) -> String {
        let mut lines = self.format_explain(record).replace("=", "#");
        lines.push('\n');
        lines.push_str(&self.format_trace(record));
        lines
    }

    pub fn format(&self, record: &ExecutionResearchRecord, mode: &str) -> String {
        match mode.to_lowercase().as_str() {
            "json" => self.format_json(record),
            "explain" => self.format_explain(record),
            "trace" => self.format_trace(record),
            "markdown" => self.format_markdown(record),
            _ => self.format_explain(record),
        }
    }
}

fn evidence_summary(ev: &Evidence) -> String {
    match &ev.payload {
        EvidencePayload::Signal { signal_label, .. } => format!("Signal({})", signal_label),
        EvidencePayload::StrategyState { state_label, .. } => format!("State({})", state_label),
        EvidencePayload::Confirmation { trend_score, participation_score, risk_score } => {
            format!("Market(trend={:.1},part={:.1},risk={:.1})", trend_score, participation_score, risk_score)
        }
        EvidencePayload::Rotation { rotation_state, .. } => format!("Rotation({})", rotation_state),
        EvidencePayload::Gap { gap_pct } => format!("Gap({:.2}%)", gap_pct * 100.0),
        EvidencePayload::Volume { volume_ratio } => format!("Volume({:.2})", volume_ratio),
        EvidencePayload::Breadth { breadth_pct, delta_5d } => format!("Breadth({:.1}%,d5={:.1}%)", breadth_pct, delta_5d),
        EvidencePayload::Close { close_position } => format!("Close({:.2})", close_position),
        EvidencePayload::Distribution { distribution_score } => format!("Distribution({:.2})", distribution_score),
        EvidencePayload::Empty => format!("{:?}", ev.kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionEvaluation, ExecutionOutcome};
    use chrono::Utc;
    use execution_engine::v2::assessment::{ExecutionAssessment, RiskLevel};
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::feature::IntradayFeatures;
    use execution_engine::v2::request::{ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot};
    use execution_engine::types::ExecutionState;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use research_context::{BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary};

    fn make_record() -> ExecutionResearchRecord {
        let request = ExecutionRequest {
            symbol: "000001".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            signal: core_domain::SignalSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                symbol: "000001".into(),
                final_score: 85.0,
                signal_label: SignalLabel::StrongBuy,
                analysis_scope: "CN".into(),
                regime_basis_scope: "CN".into(),
                reason: core_domain::SignalReason {
                    best_strategy: StrategyKind::MomentumRight,
                    strategy_score: 0.0,
                    strategy_contribution: 0.0,
                    alignment: 0,
                    aligned_strategies: vec![],
                    alignment_contribution: 0.0,
                    regime: core_domain::RegimeReason { trend_score: 0.0, risk_score: 0.0, combined_score: 0.0, contribution: 0.0 },
                    rotation: core_domain::RotationReason { momentum_score: 0.0, rank: None, combined_score: 0.0, contribution: 0.0 },
                    final_score: 85.0,
                    label: SignalLabel::StrongBuy,
                    summary: "test".into(),
                },
            },
            strategy_state: core_domain::StrategyStateSnapshot {
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                scope: "CN".into(),
                state: StrategyState::FullTrend,
                state_score: 75.0,
                transition_reason: "test".into(),
                recommended_position_pct: 100.0,
            },
            quote: QuoteSnapshot {
                symbol: "000001".into(),
                ts: Utc::now(),
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.0,
                volume: 1_000_000.0,
                prev_close: 99.0,
            },
            volume_ma20: 500_000.0,
            market_view: ExecutionMarketView {
                research_version: "1".into(),
                market_regime_label: "Bullish".into(),
                confirmation: ConfirmationSummary {
                    trend: ConfirmationDimension { score: 70.0, label: "Strong".into() },
                    participation: ConfirmationDimension { score: 60.0, label: "Moderate".into() },
                    risk: ConfirmationDimension { score: 35.0, label: "Low".into() },
                    overall: "Strong".into(),
                },
                breadth: BreadthSummary { breadth_pct: 60.0, sma5: None, delta_5d: Some(0.0), condition: "strong".into() },
                recovery: RecoverySummary { score: 60.0, drivers: vec![] },
                rotation_state: "broad".into(),
                leadership_stability: 0.7,
            },
            policy: ExecutionPolicy::default(),
        };

        let assessment = ExecutionAssessment {
            confidence: 0.85,
            consensus: 1.0,
            coverage: 1.0,
            risk: RiskLevel::Low,
            dominant_direction: 1.0,
            supporting_evidence: vec![],
            conflicting_evidence: vec![],
            neutral_evidence: vec![],
        };

        let decision = ExecutionDecision {
            symbol: "000001".into(),
            state: ExecutionState::Increase,
            confidence: 0.85,
            risk: RiskLevel::Low,
            evidences: vec![],
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };

        let event = ExecutionEvent::new(
            request,
            IntradayFeatures::default(),
            vec![],
            vec![],
            assessment,
            decision,
        );

        ExecutionResearchRecord {
            event,
            outcome: ExecutionOutcome { t20_return: Some(0.03), ..Default::default() },
            evaluation: ExecutionEvaluation::Hit,
            evaluation_version: "v1.0.0-test".into(),
            evaluated_at: Utc::now(),
        }
    }

    #[test]
    fn explain_includes_symbol_and_decision() {
        let formatter = ValidationFormatter;
        let record = make_record();
        let text = formatter.format_explain(&record);
        assert!(text.contains("000001"));
        assert!(text.contains("BuyNow"));
        assert!(text.contains("Hit"));
    }

    #[test]
    fn trace_includes_pipeline_steps() {
        let formatter = ValidationFormatter;
        let record = make_record();
        let text = formatter.format_trace(&record);
        assert!(text.contains("Quote"));
        assert!(text.contains("Feature"));
        assert!(text.contains("Observation"));
        assert!(text.contains("Evidence"));
        assert!(text.contains("Assessment"));
        assert!(text.contains("Decision"));
    }

    #[test]
    fn json_is_valid() {
        let formatter = ValidationFormatter;
        let record = make_record();
        let text = formatter.format_json(&record);
        let restored: ExecutionResearchRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(restored.evaluation, ExecutionEvaluation::Hit);
    }
}

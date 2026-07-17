use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use execution_engine::v2::event::ExecutionEvent;

use crate::{
    EvaluationEngine, ExecutionOutcome, ExecutionResearchRecord, ReplayOutcomeResolver,
    ValidationCase, ValidationSuite,
};

/// Result of running a single validation case through the Execution Platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub case: ValidationCase,
    pub event: Option<ExecutionEvent>,
    pub record: Option<ExecutionResearchRecord>,
    pub actual_decision: String,
    pub expected_decision: String,
    pub decision_match: bool,
    pub error: Option<String>,
}

/// Summary of a full validation run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub decision_accuracy: f64,
    pub results: Vec<ValidationResult>,
}

impl ValidationSummary {
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }
}

/// Runs a validation suite against the Execution Platform.
///
/// The runner is decoupled from the outcome resolver. Production runs use
/// `MarketStoreOutcomeResolver`; CI and unit tests can use `MockOutcomeResolver`
/// or any other implementation.
#[derive(Debug, Clone)]
pub struct ValidationRunner<R, E> {
    resolver: R,
    evaluator: E,
    as_of_days: i64,
}

impl<R, E> ValidationRunner<R, E>
where
    R: ReplayOutcomeResolver,
    E: EvaluationEngine,
{
    pub fn new(resolver: R, evaluator: E) -> Self {
        Self {
            resolver,
            evaluator,
            as_of_days: 180,
        }
    }

    pub fn with_as_of_days(mut self, days: i64) -> Self {
        self.as_of_days = days;
        self
    }

    /// Runs a single case.
    ///
    /// The `build_event` closure is provided by the caller (typically `app-service`)
    /// because constructing an `ExecutionEvent` requires orchestrating signal, state,
    /// quote, and market view from storage.
    pub fn run_case<F>(
        &self,
        case: &ValidationCase,
        build_event: F,
    ) -> anyhow::Result<ValidationResult>
    where
        F: FnOnce(&ValidationCase) -> anyhow::Result<ExecutionEvent>,
    {
        let event = match build_event(case) {
            Ok(e) => e,
            Err(err) => {
                return Ok(ValidationResult {
                    case: case.clone(),
                    event: None,
                    record: None,
                    actual_decision: "ERROR".into(),
                    expected_decision: case.expected_decision.clone(),
                    decision_match: false,
                    error: Some(err.to_string()),
                });
            }
        };

        let actual_decision = format!("{:?}", event.decision.state);
        let decision_match = actual_decision.eq_ignore_ascii_case(&case.expected_decision);

        let as_of = case
            .date
            .checked_add_signed(chrono::Duration::days(self.as_of_days))
            .unwrap_or(case.date);

        let record = match crate::replay_single(&self.resolver, &self.evaluator, &event, as_of) {
            Ok(r) => Some(r),
            Err(err) => {
                return Ok(ValidationResult {
                    case: case.clone(),
                    event: Some(event),
                    record: None,
                    actual_decision,
                    expected_decision: case.expected_decision.clone(),
                    decision_match,
                    error: Some(err.to_string()),
                });
            }
        };

        Ok(ValidationResult {
            case: case.clone(),
            event: Some(event),
            record,
            actual_decision,
            expected_decision: case.expected_decision.clone(),
            decision_match,
            error: None,
        })
    }

    /// Runs the entire suite and returns a summary.
    pub fn run_suite<F>(&self, suite: &ValidationSuite, build_event: F) -> ValidationSummary
    where
        F: Fn(&ValidationCase) -> anyhow::Result<ExecutionEvent>,
    {
        let mut results = Vec::with_capacity(suite.len());
        let mut passed = 0usize;

        for case in &suite.cases {
            let result = self.run_case(case, &build_event).unwrap_or_else(|err| {
                ValidationResult {
                    case: case.clone(),
                    event: None,
                    record: None,
                    actual_decision: "ERROR".into(),
                    expected_decision: case.expected_decision.clone(),
                    decision_match: false,
                    error: Some(err.to_string()),
                }
            });

            if result.decision_match && result.error.is_none() {
                passed += 1;
            }
            results.push(result);
        }

        let total = results.len();
        let decision_matches = results.iter().filter(|r| r.decision_match).count();
        let decision_accuracy = if total == 0 {
            0.0
        } else {
            decision_matches as f64 / total as f64
        };

        ValidationSummary {
            total,
            passed,
            failed: total - passed,
            decision_accuracy,
            results,
        }
    }
}

/// Mock outcome resolver for CI and unit tests.
///
/// It returns deterministic outcomes based on the case pattern type, so the suite
/// runner can be tested without a live ClickHouse instance. It is not a realistic
/// market simulation; it is only a structural test fixture.
#[derive(Debug, Clone, Default)]
pub struct MockOutcomeResolver;

impl ReplayOutcomeResolver for MockOutcomeResolver {
    fn resolve(&self, event: &ExecutionEvent, _as_of: NaiveDate) -> anyhow::Result<ExecutionOutcome> {
        let state_label = format!("{:?}", event.decision.state);
        let (t20, mfe, mae) = match state_label.as_str() {
            "BuyNow" => (0.05, 0.08, -0.02),
            "Reduce" => (-0.03, 0.02, -0.06),
            _ => (0.0, 0.01, -0.01),
        };

        Ok(ExecutionOutcome {
            t20_return: Some(t20),
            t60_return: Some(t20 * 1.5),
            t120_return: Some(t20 * 2.0),
            mfe: Some(mfe),
            mae: Some(mae),
            ..Default::default()
        })
    }
}

/// Formats a `ValidationSummary` into a human-readable report.
#[derive(Debug, Clone, Default)]
pub struct ValidationReportFormatter;

impl ValidationReportFormatter {
    pub fn format_summary(&self, summary: &ValidationSummary) -> String {
        let mut lines = Vec::new();
        lines.push("=".repeat(60));
        lines.push("Execution Validation Summary".to_string());
        lines.push("=".repeat(60));
        lines.push(format!("Total:   {}", summary.total));
        lines.push(format!("Passed:  {}", summary.passed));
        lines.push(format!("Failed:  {}", summary.failed));
        lines.push(format!("Pass Rate: {:.1}%", summary.pass_rate() * 100.0));
        lines.push(format!(
            "Decision Accuracy: {:.1}%",
            summary.decision_accuracy * 100.0
        ));
        lines.push(String::new());

        for result in &summary.results {
            let status = if result.decision_match && result.error.is_none() {
                "PASS"
            } else {
                "FAIL"
            };
            lines.push(format!(
                "[{}] {}  {} -> expected={}, actual={}",
                status,
                result.case.id,
                result.case.date,
                result.expected_decision,
                result.actual_decision
            ));
            if let Some(ref err) = result.error {
                lines.push(format!("      error: {}", err));
            }
        }

        lines.join("\n")
    }

    pub fn format_detail(&self, summary: &ValidationSummary) -> String {
        let mut lines = Vec::new();
        lines.push(self.format_summary(summary));
        lines.push(String::new());
        lines.push("=".repeat(60));
        lines.push("Detailed Failure Reports".to_string());
        lines.push("=".repeat(60));
        lines.push(String::new());

        for result in &summary.results {
            if result.decision_match && result.error.is_none() {
                continue;
            }
            lines.push(format!("Case: {}", result.case.id));
            lines.push(format!("Symbol: {}", result.case.symbol));
            lines.push(format!("Date: {}", result.case.date));
            lines.push(format!("Expected: {}", result.expected_decision));
            lines.push(format!("Actual:   {}", result.actual_decision));
            if let Some(ref err) = result.error {
                lines.push(format!("Error: {}", err));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }

    pub fn format_json(&self, summary: &ValidationSummary) -> String {
        serde_json::to_string_pretty(summary).unwrap_or_else(|e| format!("{{\"error\":\"{}\"}}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuleBasedEvaluationEngine;
    use chrono::Utc;
    use execution_engine::v2::assessment::{ExecutionAssessment, RiskLevel};
    use execution_engine::v2::decision::ExecutionDecision;
    use execution_engine::v2::event::ExecutionEvent;
    use execution_engine::v2::feature::IntradayFeatures;
    use execution_engine::v2::request::{ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot};
    use execution_engine::types::ExecutionState;
    use core_domain::{SignalLabel, StrategyKind, StrategyState};
    use research_context::{BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary};

    fn make_event(state: ExecutionState) -> ExecutionEvent {
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
            state,
            confidence: 0.85,
            risk: RiskLevel::Low,
            evidences: vec![],
            assessment: assessment.clone(),
            decision_reasons: vec![],
        };

        ExecutionEvent::new(
            request,
            IntradayFeatures::default(),
            vec![],
            vec![],
            assessment,
            decision,
        )
    }

    #[test]
    fn mock_resolver_returns_outcome() {
        let event = make_event(ExecutionState::BuyNow);
        let resolver = MockOutcomeResolver;
        let outcome = resolver.resolve(&event, chrono::NaiveDate::from_ymd_opt(2026, 1, 2).unwrap()).unwrap();
        assert!(outcome.t20_return.unwrap() > 0.0);
    }

    #[test]
    fn runner_compares_expected_decision() {
        let case = ValidationCase {
            id: "TEST-001".into(),
            symbol: "000001".into(),
            date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            scope: "cn".into(),
            market_regime: "Bullish".into(),
            pattern_type: "Test".into(),
            expected_decision: "BuyNow".into(),
            reason: "test".into(),
            validated_by: "Manual".into(),
            notes: "test".into(),
        };

        let runner = ValidationRunner::new(MockOutcomeResolver, RuleBasedEvaluationEngine);
        let result = runner.run_case(&case, |_| Ok(make_event(ExecutionState::BuyNow))).unwrap();

        assert!(result.decision_match);
        assert_eq!(result.actual_decision, "BuyNow");
    }

    #[test]
    fn summary_counts_pass_and_fail() {
        let suite = ValidationSuite {
            suite: "test".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            schema: crate::validation_suite::ValidationSchema {
                id: "".into(), symbol: "".into(), date: "".into(), scope: "".into(),
                market_regime: "".into(), pattern_type: "".into(), expected_decision: "".into(),
                reason: "".into(), validated_by: "".into(), notes: "".into(),
            },
            cases: vec![
                ValidationCase {
                    id: "PASS-001".into(),
                    symbol: "000001".into(),
                    date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                    scope: "cn".into(),
                    market_regime: "Bullish".into(),
                    pattern_type: "Test".into(),
                    expected_decision: "BuyNow".into(),
                    reason: "test".into(),
                    validated_by: "Manual".into(),
                    notes: "test".into(),
                },
                ValidationCase {
                    id: "FAIL-001".into(),
                    symbol: "000001".into(),
                    date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                    scope: "cn".into(),
                    market_regime: "Bullish".into(),
                    pattern_type: "Test".into(),
                    expected_decision: "Wait".into(),
                    reason: "test".into(),
                    validated_by: "Manual".into(),
                    notes: "test".into(),
                },
            ],
        };

        let runner = ValidationRunner::new(MockOutcomeResolver, RuleBasedEvaluationEngine);
        let summary = runner.run_suite(&suite, |_| Ok(make_event(ExecutionState::BuyNow)));

        assert_eq!(summary.total, 2);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn summary_formatter_produces_report() {
        let suite = ValidationSuite {
            suite: "test".into(),
            version: "1.0.0".into(),
            description: "test".into(),
            schema: crate::validation_suite::ValidationSchema {
                id: "".into(), symbol: "".into(), date: "".into(), scope: "".into(),
                market_regime: "".into(), pattern_type: "".into(), expected_decision: "".into(),
                reason: "".into(), validated_by: "".into(), notes: "".into(),
            },
            cases: vec![
                ValidationCase {
                    id: "PASS-001".into(),
                    symbol: "000001".into(),
                    date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                    scope: "cn".into(),
                    market_regime: "Bullish".into(),
                    pattern_type: "Test".into(),
                    expected_decision: "BuyNow".into(),
                    reason: "test".into(),
                    validated_by: "Manual".into(),
                    notes: "test".into(),
                },
            ],
        };

        let runner = ValidationRunner::new(MockOutcomeResolver, RuleBasedEvaluationEngine);
        let summary = runner.run_suite(&suite, |_| Ok(make_event(ExecutionState::BuyNow)));
        let formatter = ValidationReportFormatter;
        let text = formatter.format_summary(&summary);

        assert!(text.contains("PASS"));
        assert!(text.contains("Decision Accuracy"));
    }
}

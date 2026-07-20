use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::AnalysisScope;
use data_ingestion::load_universe;
use execution_engine::v2::event::ExecutionEvent;
use execution_engine::v2::request::{ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot};
use execution_engine::v2::{DefaultExecutionPipeline, ExecutionPipeline};
use execution_replay::{
    replay_single, compute_execution_statistics, ExecutionResearchRecord, MarketStoreOutcomeResolver,
    RuleBasedEvaluationEngine, ValidationCandidate, ValidationRunner, ValidationSuite,
    ValidationSummary,
};
use market_store::{
    fetch_daily_bars_for_symbols_in_range, fetch_latest_strategy_state_on_or_before,
    fetch_signal_snapshots_for_range_with_scope, fetch_signal_snapshot_for_symbol,
};
use std::collections::BTreeMap;

use crate::core::instrument_in_scope;
use crate::AppContext;
use crate::ReportScope;

/// Builds an `ExecutionEvent` from persisted market data for a historical case.
///
/// This is a validation helper. It intentionally fails loud if any required input
/// is missing. The caller must supply a pre-built `ResearchContext` for the
/// date/scope so that the `ExecutionMarketView` carries live computed values
/// rather than placeholders.
fn build_execution_event(
    app: &AppContext,
    ctx: &research_context::ResearchContext,
    symbol: &str,
    date: NaiveDate,
    scope: AnalysisScope,
    strategy_state: Option<core_domain::StrategyStateSnapshot>,
) -> Result<ExecutionEvent> {
    let signal = fetch_signal_snapshot_for_symbol(&app.storage, date, symbol, scope)
        .context("failed to fetch signal snapshot")?
        .with_context(|| format!("no signal snapshot for {} on {:?}", symbol, date))?;

    let strategy_state = match strategy_state {
        Some(s) => s,
        None => fetch_latest_strategy_state_on_or_before(&app.storage, date, scope)
            .context("failed to fetch strategy state")?
            .with_context(|| format!("no strategy state for {:?} on or before {:?}", scope, date))?,
    };

    let bars = fetch_daily_bars_for_symbols_in_range(&app.storage, &[symbol.to_string()], date, date)
        .context("failed to fetch daily bar")?;
    let bar = bars
        .into_iter()
        .next()
        .with_context(|| format!("no daily bar for {} on {:?}", symbol, date))?;

    // Fetch a short lookback to find the previous close and compute the real
    // 20-day volume moving average. Both need bars before the current date.
    let (prev_close, volume_ma20) = if let Some(lookback_start) = date.checked_sub_signed(chrono::Duration::days(40)) {
        let prev_bars = fetch_daily_bars_for_symbols_in_range(
            &app.storage,
            &[symbol.to_string()],
            lookback_start,
            date.checked_sub_signed(chrono::Duration::days(1)).unwrap_or(lookback_start),
        )
        .context("failed to fetch previous daily bars")?;

        let prev_close = prev_bars
            .iter()
            .last()
            .map(|b| b.close)
            .unwrap_or(bar.close);

        let volumes: Vec<f64> = prev_bars.iter().map(|b| b.volume).collect();
        let volume_ma20 = if volumes.len() >= 20 {
            volumes.iter().rev().take(20).sum::<f64>() / 20.0
        } else if !volumes.is_empty() {
            volumes.iter().sum::<f64>() / volumes.len() as f64
        } else {
            bar.volume
        };

        (prev_close, volume_ma20)
    } else {
        (bar.close, bar.volume)
    };

    let quote = QuoteSnapshot {
        symbol: bar.symbol.clone(),
        ts: chrono::Utc::now(), // historical replay uses current timestamp for artifact provenance
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
        volume: bar.volume,
        prev_close,
    };

    // Project the pre-built ResearchContext into the ExecutionMarketView. This
    // fixes the placeholder values that previously degraded all
    // ResearchContext-derived evidence.
    let market_view = ExecutionMarketView::from_research_context(ctx);

    let request = ExecutionRequest {
        symbol: symbol.to_string(),
        date,
        signal,
        strategy_state,
        quote,
        volume_ma20,
        market_view,
        policy: ExecutionPolicy::default(),
    };

    Ok(DefaultExecutionPipeline.execute(request))
}

/// Orchestrates a single historical replay for a symbol/date.
pub fn run_single_execution_replay(
    app: &AppContext,
    symbol: &str,
    date: NaiveDate,
    scope: AnalysisScope,
) -> Result<ExecutionResearchRecord> {
    let ctx = app
        .build_research_context_for_date(date, scope)
        .context("failed to build ResearchContext for execution replay")?;
    let event = build_execution_event(app, &ctx, symbol, date, scope, None)?;
    let resolver = MarketStoreOutcomeResolver::new(app.storage.clone());
    let evaluator = RuleBasedEvaluationEngine;
    let as_of = date
        .checked_add_signed(chrono::Duration::days(180))
        .context("failed to compute as-of date")?;

    replay_single(&resolver, &evaluator, &event, as_of)
        .context("replay failed")
}

/// Loads a suite of cases and replays each into an `ExecutionResearchRecord`.
fn load_records_from_suite(
    app: &AppContext,
    suite_path: &std::path::Path,
) -> Result<Vec<ExecutionResearchRecord>> {
    let suite = ValidationSuite::from_file(suite_path)
        .with_context(|| format!("failed to load suite from {:?}", suite_path))?;

    let resolver = MarketStoreOutcomeResolver::new(app.storage.clone());
    let evaluator = RuleBasedEvaluationEngine;
    let as_of_days = 180;

    let mut records = Vec::new();
    for case in &suite.cases {
        let scope = match case.scope.to_lowercase().as_str() {
            "cn" => AnalysisScope::Cn,
            "hk" => AnalysisScope::Hk,
            "global" => AnalysisScope::Global,
            other => anyhow::bail!("unknown scope: {}", other),
        };
        let ctx = app
            .build_research_context_for_date(case.date, scope)
            .with_context(|| format!("failed to build ResearchContext for {:?} on {:?}", scope, case.date))?;
        let event = build_execution_event(app, &ctx, &case.symbol, case.date, scope, None)?;
        let as_of = case
            .date
            .checked_add_signed(chrono::Duration::days(as_of_days))
            .context("failed to compute as-of date")?;
        let record = replay_single(&resolver, &evaluator, &event, as_of)?;
        records.push(record);
    }

    Ok(records)
}

/// Loads all validation candidates in a date range and replays each into an
/// `ExecutionResearchRecord`.
fn load_records_from_range(
    app: &AppContext,
    from: NaiveDate,
    to: NaiveDate,
    scope: ReportScope,
    decision_filter: Option<&str>,
) -> Result<Vec<ExecutionResearchRecord>> {
    let scope_value = match scope {
        ReportScope::Global => AnalysisScope::Global,
        ReportScope::Cn => AnalysisScope::Cn,
        ReportScope::Hk => AnalysisScope::Hk,
    };

    let universe = load_universe(&app.storage.universe_abspath()?)?;
    let symbols: Vec<String> = universe
        .into_iter()
        .filter(|instrument| instrument.enabled && instrument_in_scope(instrument, scope))
        .map(|instrument| instrument.symbol)
        .collect();

    let signals = fetch_signal_snapshots_for_range_with_scope(&app.storage, scope_value, from, to)?;
    let mut records = Vec::new();
    let mut state_cache: BTreeMap<NaiveDate, core_domain::StrategyStateSnapshot> = BTreeMap::new();
    let mut ctx_cache: BTreeMap<NaiveDate, research_context::ResearchContext> = BTreeMap::new();

    let resolver = MarketStoreOutcomeResolver::new(app.storage.clone());
    let evaluator = RuleBasedEvaluationEngine;
    let as_of_days = 180;

    for signal in signals {
        if !symbols.contains(&signal.symbol) {
            continue;
        }

        let date = signal.date;
        let symbol = signal.symbol.clone();

        let state = match state_cache.get(&date) {
            Some(s) => s.clone(),
            None => match fetch_latest_strategy_state_on_or_before(&app.storage, date, scope_value)? {
                Some(s) => {
                    state_cache.insert(date, s.clone());
                    s
                }
                None => continue,
            },
        };

        let ctx = match ctx_cache.get(&date) {
            Some(c) => c,
            None => match app.build_research_context_for_date(date, scope_value) {
                Ok(c) => {
                    ctx_cache.insert(date, c);
                    ctx_cache.get(&date).unwrap()
                }
                Err(_) => continue,
            },
        };

        let event = match build_execution_event(app, ctx, &symbol, date, scope_value, Some(state)) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if let Some(filter) = decision_filter {
            let decision_state = format!("{:?}", event.decision.state);
            if !decision_state.eq_ignore_ascii_case(filter) {
                continue;
            }
        }

        let as_of = date
            .checked_add_signed(chrono::Duration::days(as_of_days))
            .context("failed to compute as-of date")?;
        let record = replay_single(&resolver, &evaluator, &event, as_of)?;
        records.push(record);
    }

    Ok(records)
}

impl AppContext {
    /// Public entry point for running a single execution replay validation.
    pub fn validate_execution_replay(
        &self,
        symbol: &str,
        date: NaiveDate,
        scope: AnalysisScope,
    ) -> Result<ExecutionResearchRecord> {
        run_single_execution_replay(&self, symbol, date, scope)
    }

    /// Discovers historical validation candidates from persisted data.
    ///
    /// Scans `from` to `to` for all symbols in the given scope, and returns
    /// every symbol/date that has a complete input set (signal, strategy state,
    /// daily bar). The candidate also includes the resulting Execution Decision
    /// so engineers can quickly select golden cases by pattern.
    pub fn find_validation_candidates(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<Vec<ValidationCandidate>> {
        let universe = load_universe(&self.storage.universe_abspath()?)?;
        let symbols: Vec<String> = universe
            .into_iter()
            .filter(|instrument| instrument.enabled && instrument_in_scope(instrument, scope))
            .map(|instrument| instrument.symbol)
            .collect();

        let scope_value = match scope {
            ReportScope::Global => AnalysisScope::Global,
            ReportScope::Cn => AnalysisScope::Cn,
            ReportScope::Hk => AnalysisScope::Hk,
        };

        let signals = fetch_signal_snapshots_for_range_with_scope(&self.storage, scope_value, from, to)?;
        let mut candidates = Vec::new();
        let mut state_cache: BTreeMap<NaiveDate, core_domain::StrategyStateSnapshot> = BTreeMap::new();
        let mut ctx_cache: BTreeMap<NaiveDate, research_context::ResearchContext> = BTreeMap::new();

        for signal in signals {
            if !symbols.contains(&signal.symbol) {
                continue;
            }

            let date = signal.date;
            let symbol = signal.symbol.clone();

            let state = match state_cache.get(&date) {
                Some(s) => s.clone(),
                None => {
                    match fetch_latest_strategy_state_on_or_before(&self.storage, date, scope_value)? {
                        Some(s) => {
                            state_cache.insert(date, s.clone());
                            s
                        }
                        None => continue,
                    }
                }
            };

            let ctx = match ctx_cache.get(&date) {
                Some(c) => c,
                None => match self.build_research_context_for_date(date, scope_value) {
                    Ok(c) => {
                        ctx_cache.insert(date, c);
                        ctx_cache.get(&date).unwrap()
                    }
                    Err(_) => continue,
                },
            };

            match build_execution_event(&self, ctx, &symbol, date, scope_value, Some(state.clone())) {
                Ok(event) => {
                    let decision_state = format!("{:?}", event.decision.state);
                    if let Some(filter) = decision_filter {
                        if !decision_state.eq_ignore_ascii_case(filter) {
                            continue;
                        }
                    }
                    candidates.push(ValidationCandidate {
                        symbol,
                        date,
                        scope: format!("{:?}", scope),
                        signal_label: format!("{:?}", signal.signal_label),
                        signal_score: signal.final_score,
                        strategy_state: format!("{:?}", state.state),
                        market_regime_label: event.request.market_view.market_regime_label.clone(),
                        decision_state,
                        confidence: event.decision.confidence,
                        risk: format!("{:?}", event.decision.risk),
                        evidence_count: event.evidences.len(),
                    });
                }
                Err(_) => continue,
            }
        }

        Ok(candidates)
    }

    /// Public entry point for running a validation suite against the platform.
    pub fn validate_execution_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<ValidationSummary> {
        let suite = ValidationSuite::from_file(suite_path)
            .with_context(|| format!("failed to load suite from {:?}", suite_path))?;

        let resolver = MarketStoreOutcomeResolver::new(self.storage.clone());
        let evaluator = RuleBasedEvaluationEngine;
        let runner = ValidationRunner::new(resolver, evaluator);

        let summary = runner.run_suite(&suite, |case| {
            let scope = match case.scope.to_lowercase().as_str() {
                "cn" => AnalysisScope::Cn,
                "hk" => AnalysisScope::Hk,
                "global" => AnalysisScope::Global,
                other => anyhow::bail!("unknown scope: {}", other),
            };
            let ctx = self
                .build_research_context_for_date(case.date, scope)
                .with_context(|| format!("failed to build ResearchContext for {:?} on {:?}", scope, case.date))?;
            build_execution_event(&self, &ctx, &case.symbol, case.date, scope, None)
        });

        Ok(summary)
    }

    /// Computes Execution Statistics from a Golden Suite (representative sample).
    ///
    /// Runs every case, replays it, and feeds the resulting records into the
    /// frozen Execution Statistics contract.
    pub fn execution_statistics_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::ExecutionStatistics> {
        let suite = ValidationSuite::from_file(suite_path)
            .with_context(|| format!("failed to load suite from {:?}", suite_path))?;

        let resolver = MarketStoreOutcomeResolver::new(self.storage.clone());
        let evaluator = RuleBasedEvaluationEngine;
        let as_of_days = 180;

        let mut records = Vec::new();
        let mut ctx_cache: BTreeMap<NaiveDate, research_context::ResearchContext> = BTreeMap::new();
        for case in &suite.cases {
            let scope = match case.scope.to_lowercase().as_str() {
                "cn" => AnalysisScope::Cn,
                "hk" => AnalysisScope::Hk,
                "global" => AnalysisScope::Global,
                other => {
                    anyhow::bail!("unknown scope: {}", other);
                }
            };
            let ctx = match ctx_cache.get(&case.date) {
                Some(c) => c,
                None => {
                    let c = self
                        .build_research_context_for_date(case.date, scope)
                        .with_context(|| format!("failed to build ResearchContext for {:?} on {:?}", scope, case.date))?;
                    ctx_cache.insert(case.date, c);
                    ctx_cache.get(&case.date).unwrap()
                }
            };
            let event = build_execution_event(&self, ctx, &case.symbol, case.date, scope, None)?;
            let as_of = case
                .date
                .checked_add_signed(chrono::Duration::days(as_of_days))
                .context("failed to compute as-of date")?;
            let record = replay_single(&resolver, &evaluator, &event, as_of)?;
            records.push(record);
        }

        Ok(compute_execution_statistics(&records))
    }

    /// Computes Execution Statistics over a historical date range (full population).
    ///
    /// Discovers all validation candidates in the range, replays each one, and
    /// feeds the resulting records into the frozen Execution Statistics contract.
    pub fn execution_statistics_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::ExecutionStatistics> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        let mut stats = compute_execution_statistics(&records);
        stats.meta.scope = Some(format!("{:?}", scope));
        stats.meta.from_date = Some(from);
        stats.meta.to_date = Some(to);
        Ok(stats)
    }

    /// Computes an Evidence Trace from a Golden Suite (representative sample).
    ///
    /// Traces every EvidenceKind through Observation → Evidence → Assessment →
    /// Decision so engineers can see where a particular evidence signal dies.
    pub fn execution_evidence_trace_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::EvidenceTrace> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_evidence_trace(&records))
    }

    /// Computes an Evidence Trace over a historical date range (full population).
    pub fn execution_evidence_trace_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::EvidenceTrace> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        let mut trace = execution_replay::compute_evidence_trace(&records);
        trace.meta.scope = Some(format!("{:?}", scope));
        trace.meta.from_date = Some(from);
        trace.meta.to_date = Some(to);
        Ok(trace)
    }

    /// 2A-4A: Distribution Coverage Review.
    ///
    /// Analyzes intraday features to understand whether the current Distribution
    /// observation condition is too strict or too loose. No pipeline code is modified.
    pub fn execution_distribution_coverage_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::DistributionCoverageReview> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_distribution_coverage_review(&records))
    }

    /// 2A-4A: Distribution Coverage Review over a historical date range.
    pub fn execution_distribution_coverage_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::DistributionCoverageReview> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_distribution_coverage_review(&records))
    }

    /// 2A-4B: Decision Margin Review.
    ///
    /// Analyzes how `assessment.dominant_direction` maps to final decisions for
    /// each EvidenceKind. No pipeline code is modified.
    pub fn execution_decision_margin_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::DecisionMarginReview> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_decision_margin_review(&records))
    }

    /// 2A-4B: Decision Margin Review over a historical date range.
    pub fn execution_decision_margin_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::DecisionMarginReview> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_decision_margin_review(&records))
    }

    /// 2A-4C/2A-4.5: Decision Gate Analysis.
    ///
    /// Enumerates every Reduce candidate (dominant_direction < reduce_threshold)
    /// and reports which DecisionEngine gate blocked it: RiskCritical, RiskHigh,
    /// ConfidenceTooLow, or ConsensusTooLow. No pipeline code is modified.
    pub fn execution_decision_gate_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::DecisionGateAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_decision_gate_analysis(&records))
    }

    /// 2A-4C/2A-4.5: Decision Gate Analysis over a historical date range.
    pub fn execution_decision_gate_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::DecisionGateAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_decision_gate_analysis(&records))
    }

    /// 2A-4C: Risk Semantics Review.
    ///
    /// Analyzes RiskLevel::High records to determine whether risk is being used
    /// as Entry Risk (suppress BuyNow) or Holding Risk (drive Reduce), and whether
    /// the current semantics suppress necessary Reduce actions. No pipeline code is modified.
    pub fn execution_risk_semantics_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::RiskSemanticsReview> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_risk_semantics_review(&records))
    }

    /// 2A-4C: Risk Semantics Review over a historical date range.
    pub fn execution_risk_semantics_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::RiskSemanticsReview> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_risk_semantics_review(&records))
    }

    /// 2A-5: Directional Confidence Calibration Experiment.
    ///
    /// Replays the same set of records under a set of alternative confidence
    /// thresholds, measuring coverage, precision, and opportunity cost without
    /// modifying any engine defaults.
    pub fn execution_calibration_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::CalibrationReview> {
        let records = load_records_from_suite(&self, suite_path)?;
        let experiments = default_calibration_experiments();
        Ok(execution_replay::compute_calibration_review(&records, &experiments))
    }

    /// 2A-5: Directional Confidence Calibration Experiment over a historical date range.
    pub fn execution_calibration_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::CalibrationReview> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        let experiments = default_calibration_experiments();
        Ok(execution_replay::compute_calibration_review(&records, &experiments))
    }

    /// 2B-1: Bearish Evidence Analysis.
    ///
    /// Analyzes existing bearish candidates (dominant_direction < reduce_threshold)
    /// and their Evidence composition against historical outcomes. This is a research
    /// tool: it does not modify any Evidence, Assessment, Decision, or Policy code.
    /// The goal is to discover Exit-specific patterns before designing new evidence.
    pub fn execution_bearish_analysis_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::BearishAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_bearish_analysis(&records))
    }

    /// 2B-1: Bearish Evidence Analysis over a historical date range.
    pub fn execution_bearish_analysis_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::BearishAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_bearish_analysis(&records))
    }

    /// 2B-2: Transition Evidence Modeling over a validation suite.
    ///
    /// Research-only analysis of change/deterioration signals. Does not modify any
    /// Observation, Evidence, Assessment, Decision, or Policy code.
    pub fn execution_transition_analysis_from_suite(
        &self,
        suite_path: &std::path::Path,
        candidate: execution_replay::TransitionCandidate,
    ) -> Result<execution_replay::TransitionAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_transition_analysis(&records, candidate))
    }

    /// 2B-2: Transition Evidence Modeling over a historical date range.
    pub fn execution_transition_analysis_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
        candidate: execution_replay::TransitionCandidate,
    ) -> Result<execution_replay::TransitionAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_transition_analysis(&records, candidate))
    }

    /// 2B-2.4: LeadershipDecay Horizon Analysis over a historical date range.
    ///
    /// Research-only multi-horizon profile of the LeadershipDecay signal.
    /// Does not modify any Observation, Evidence, Assessment, Decision, or Policy code.
    pub fn execution_leadership_decay_horizon_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::LeadershipDecayHorizonAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_leadership_decay_horizon_analysis(&records))
    }

    /// 2B-2.4: LeadershipDecay Horizon Analysis over a validation suite.
    pub fn execution_leadership_decay_horizon_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::LeadershipDecayHorizonAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_leadership_decay_horizon_analysis(&records))
    }

    /// 2B-3: Holding Risk Evidence Bundle Analysis over a historical date range.
    ///
    /// Research-only combination of LeadershipDecay, BreadthDeterioration, and
    /// LiquidityDeterioration into a medium-term (T+60) Holding Risk Score.
    /// Does not modify any Observation, Evidence, Assessment, Decision, or Policy code.
    pub fn execution_holding_risk_bundle_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::HoldingRiskBundleAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_holding_risk_bundle_analysis(&records))
    }

    /// 2B-3: Holding Risk Evidence Bundle Analysis over a validation suite.
    pub fn execution_holding_risk_bundle_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::HoldingRiskBundleAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_holding_risk_bundle_analysis(&records))
    }

    /// 2B-0: ResearchContext Fact Integrity Audit over a historical date range.
    ///
    /// Read-only audit of all ResearchContext-derived fields in ExecutionMarketView.
    /// Does not modify any Observation, Evidence, Assessment, Decision, or Policy code.
    pub fn execution_context_integrity_audit_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::ContextIntegrityReport> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_context_integrity_report(&records))
    }

    /// 2B-0: ResearchContext Fact Integrity Audit over a validation suite.
    pub fn execution_context_integrity_audit_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::ContextIntegrityReport> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_context_integrity_report(&records))
    }

    /// TASK-159: Context Integrity Gate — strict pass/fail firewall for the
    /// ResearchContext → ExecutionEvent fact lineage.
    ///
    /// Returns `Ok(true)` when all audited fields satisfy the V8 contract. Returns
    /// `Ok(false)` when any field is constant, placeholder-valued, low-variance, or
    /// dominated by a single value. Evidence Modeling must remain blocked until
    /// this gate passes.
    pub fn execution_context_integrity_gate_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::ContextIntegrityValidation> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::validate_execution_context(&records))
    }

    /// TASK-159: Context Integrity Gate over a validation suite.
    pub fn execution_context_integrity_gate_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::ContextIntegrityValidation> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::validate_execution_context(&records))
    }

    /// TASK-160.1: Holding Risk Persistence Analysis over a historical date range.
    ///
    /// Research-only analysis of LeadershipDecay persistence and velocity. Does not
    /// modify any Observation, Evidence, Assessment, Decision, or Policy code.
    pub fn holding_risk_persistence_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::HoldingRiskPersistenceAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_holding_risk_persistence_analysis(&records))
    }

    /// TASK-160.1: Holding Risk Persistence Analysis over a validation suite.
    pub fn holding_risk_persistence_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::HoldingRiskPersistenceAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_holding_risk_persistence_analysis(&records))
    }

    /// TASK-160.1: Holding Risk Bundle V2 over a historical date range.
    ///
    /// Combines LeadershipDecay persistence, BreadthDeterioration, and
    /// LiquidityDeterioration into a medium-term (T+60) holding risk score.
    /// Research-only; does not modify the Execution Pipeline.
    pub fn holding_risk_bundle_v2_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
        min_leadership_persistence_days: usize,
    ) -> Result<execution_replay::HoldingRiskBundleAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_holding_risk_bundle_v2_analysis(
            &records,
            min_leadership_persistence_days,
        ))
    }

    /// TASK-160.1: Holding Risk Bundle V2 over a validation suite.
    pub fn holding_risk_bundle_v2_from_suite(
        &self,
        suite_path: &std::path::Path,
        min_leadership_persistence_days: usize,
    ) -> Result<execution_replay::HoldingRiskBundleAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_holding_risk_bundle_v2_analysis(
            &records,
            min_leadership_persistence_days,
        ))
    }

    /// TASK-160.2A: LiquidityPressure Research Asset over a historical date range.
    ///
    /// Sustained capital pressure analysis (turnover decay + price weakness + breadth
    /// not recovering + persistence). Research-only; does not modify the Execution
    /// Pipeline.
    pub fn liquidity_pressure_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
        consecutive_pressure_days: usize,
        volume_ratio_delta_threshold: f64,
        require_price_weakness: bool,
        require_breadth_weakness: bool,
        volume_level_threshold: Option<f64>,
    ) -> Result<execution_replay::LiquidityPressureAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_liquidity_pressure_analysis_with_params(
            &records,
            consecutive_pressure_days,
            volume_ratio_delta_threshold,
            require_price_weakness,
            require_breadth_weakness,
            volume_level_threshold,
        ))
    }

    /// TASK-160.2A: LiquidityPressure Research Asset over a validation suite.
    pub fn liquidity_pressure_from_suite(
        &self,
        suite_path: &std::path::Path,
        consecutive_pressure_days: usize,
        volume_ratio_delta_threshold: f64,
        require_price_weakness: bool,
        require_breadth_weakness: bool,
        volume_level_threshold: Option<f64>,
    ) -> Result<execution_replay::LiquidityPressureAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_liquidity_pressure_analysis_with_params(
            &records,
            consecutive_pressure_days,
            volume_ratio_delta_threshold,
            require_price_weakness,
            require_breadth_weakness,
            volume_level_threshold,
        ))
    }

    /// TASK-160.2A: Holding Risk Bundle V3 over a historical date range.
    ///
    /// Combines LeadershipDecay persistence (>=5 days), LiquidityPressure (any volume
    /// decline, >=3 days), and BreadthDeterioration. Research-only.
    pub fn holding_risk_bundle_v3_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::HoldingRiskBundleAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_holding_risk_bundle_v3_analysis(&records))
    }

    /// TASK-160.2A: Holding Risk Bundle V3 over a validation suite.
    pub fn holding_risk_bundle_v3_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::HoldingRiskBundleAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_holding_risk_bundle_v3_analysis(&records))
    }

    /// TASK-160.2B: ConfirmationDecay Research Asset over a historical date range.
    ///
    /// Change-based confirmation analysis (delta/velocity/persistence + optional
    /// price weakness). Research-only; does not modify the Execution Pipeline.
    pub fn confirmation_decay_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
        delta_5d_threshold: f64,
        slope_10d_threshold: f64,
        min_consecutive_days: usize,
        require_price_weakness: bool,
    ) -> Result<execution_replay::ConfirmationDecayAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_confirmation_decay_analysis_with_params(
            &records,
            delta_5d_threshold,
            slope_10d_threshold,
            min_consecutive_days,
            require_price_weakness,
        ))
    }

    /// TASK-160.2B: ConfirmationDecay Research Asset over a validation suite.
    pub fn confirmation_decay_from_suite(
        &self,
        suite_path: &std::path::Path,
        delta_5d_threshold: f64,
        slope_10d_threshold: f64,
        min_consecutive_days: usize,
        require_price_weakness: bool,
    ) -> Result<execution_replay::ConfirmationDecayAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_confirmation_decay_analysis_with_params(
            &records,
            delta_5d_threshold,
            slope_10d_threshold,
            min_consecutive_days,
            require_price_weakness,
        ))
    }

    /// TASK-160.2B: Holding Risk Bundle V4 over a historical date range.
    ///
    /// Adds ConfirmationDecay as a Confirmatory Dimension to the V3 bundle.
    pub fn holding_risk_bundle_v4_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::HoldingRiskBundleAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_holding_risk_bundle_v4_analysis(&records))
    }

    /// TASK-160.2B: Holding Risk Bundle V4 over a validation suite.
    pub fn holding_risk_bundle_v4_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::HoldingRiskBundleAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_holding_risk_bundle_v4_analysis(&records))
    }

    /// TASK-161: Holding Risk Calibration v2 over a historical date range.
    ///
    /// Computes HoldingRiskScore and validates it with score buckets, regime split,
    /// and walk-forward validation. Research-only.
    pub fn holding_risk_calibration_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::HoldingRiskCalibrationAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_holding_risk_calibration(&records))
    }

    /// TASK-161: Holding Risk Calibration v2 over a validation suite.
    pub fn holding_risk_calibration_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::HoldingRiskCalibrationAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_holding_risk_calibration(&records))
    }

    /// TASK-163: Holding Risk Lifecycle Analysis over a historical date range.
    ///
    /// Builds a risk state machine around HoldingRiskScore: entry, peak, recovery,
    /// duration, and false alarm analysis. Research-only.
    pub fn risk_lifecycle_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::RiskLifecycleAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_risk_lifecycle_analysis(&records))
    }

    /// TASK-163: Holding Risk Lifecycle Analysis over a validation suite.
    pub fn risk_lifecycle_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::RiskLifecycleAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_risk_lifecycle_analysis(&records))
    }

    /// TASK-166: Regime-Aware State Risk Model over a historical date range.
    ///
    /// Identifies when the market is ALREADY in a dangerous state (State Detector),
    /// not 'deteriorating' transitions. Research-only.
    pub fn regime_risk_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::RegimeRiskAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_regime_risk_analysis(&records))
    }

    /// TASK-166: Regime-Aware State Risk Model over a validation suite.
    pub fn regime_risk_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::RegimeRiskAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_regime_risk_analysis(&records))
    }

    /// TASK-168: State Risk Acceleration Model over a historical date range.
    ///
    /// Identifies accelerating-decline conditions (not oversold/mean-reversion).
    /// Research-only.
    pub fn state_risk_acceleration_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::StateRiskAccelerationAnalysis> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        Ok(execution_replay::compute_state_risk_acceleration_analysis(&records))
    }

    /// TASK-168: State Risk Acceleration Model over a validation suite.
    pub fn state_risk_acceleration_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::StateRiskAccelerationAnalysis> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_state_risk_acceleration_analysis(&records))
    }

    /// TASK-167: Shadow Mode Runtime Wiring over a historical date range.
    ///
    /// Generates daily shadow-mode output using market_regime_label as State Context
    /// and HoldingRiskScore as Transition Evidence. Read-only bypass.
    pub fn shadow_mode_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::ShadowModeReport> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        let scope_str = match scope {
            ReportScope::Global => "global",
            ReportScope::Cn => "cn",
            ReportScope::Hk => "hk",
        };
        Ok(execution_replay::compute_shadow_mode_report(&records, scope_str))
    }

    /// TASK-167: Shadow Mode Runtime Wiring over a validation suite.
    pub fn shadow_mode_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::ShadowModeReport> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_shadow_mode_report(&records, "suite"))
    }

    /// TASK-169: Shadow Deployment Contract over a historical date range.
    ///
    /// Generates daily ShadowRiskAssessment using market_regime_label as State Context
    /// and HoldingRiskScore as Transition Evidence. Explicitly prohibited for
    /// DecisionEngine consumption.
    pub fn shadow_deployment_from_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        scope: ReportScope,
        decision_filter: Option<&str>,
    ) -> Result<execution_replay::ShadowDeploymentReport> {
        let records = load_records_from_range(&self, from, to, scope, decision_filter)?;
        let scope_str = match scope {
            ReportScope::Global => "global",
            ReportScope::Cn => "cn",
            ReportScope::Hk => "hk",
        };
        Ok(execution_replay::compute_shadow_deployment_report(&records, scope_str))
    }

    /// TASK-169: Shadow Deployment Contract over a validation suite.
    pub fn shadow_deployment_from_suite(
        &self,
        suite_path: &std::path::Path,
    ) -> Result<execution_replay::ShadowDeploymentReport> {
        let records = load_records_from_suite(&self, suite_path)?;
        Ok(execution_replay::compute_shadow_deployment_report(&records, "suite"))
    }

    /// TASK-170: Live Context Integrity Gate over the current day's ResearchContext.
    ///
    /// Validates that the current day's `ResearchContext` → `ExecutionMarketView`
    /// projection is not polluted by placeholder values. This is a lightweight check
    /// that only verifies known placeholder values are absent; it does NOT require
    /// variance or unique ratios (which are meaningless for a single day).
    ///
    /// TASK-173: Uses the `ExecutionContextIntegrityContract` for known placeholders
    /// instead of hardcoded values, unifying Live and Replay integrity checks.
    pub fn execution_context_live_integrity_check(
        &self,
        scope: ReportScope,
    ) -> Result<execution_replay::ContextIntegrityValidation> {
        let dates = self.dashboard_available_dates()?;
        let latest_date_str = dates
            .first()
            .context("no available trading dates for live integrity gate")?;
        let latest_date = NaiveDate::parse_from_str(latest_date_str, "%Y-%m-%d")
            .context("failed to parse latest trading date")?;

        let records = load_records_from_range(&self, latest_date, latest_date, scope, None)?;

        let contract = execution_replay::ExecutionContextIntegrityContract::v8_default();
        let mut failed = Vec::new();
        for record in &records {
            let view = &record.event.request.market_view;
            for rule in &contract.rules {
                let value = match rule.field_name.as_str() {
                    "confirmation.trend.score" => view.confirmation.trend.score,
                    "confirmation.participation.score" => view.confirmation.participation.score,
                    "confirmation.risk.score" => view.confirmation.risk.score,
                    "breadth.breadth_pct" => view.breadth.breadth_pct,
                    "breadth.delta_5d" => view.breadth.delta_5d.unwrap_or(0.0),
                    "breadth.sma5" => view.breadth.sma5.unwrap_or(0.0),
                    "recovery.score" => view.recovery.score,
                    "leadership_stability" => view.leadership_stability,
                    _ => continue,
                };
                for ph in &rule.known_placeholders {
                    if (value - ph).abs() < 1e-9 {
                        failed.push(format!("{} is placeholder {:.2}", rule.field_name, ph));
                    }
                }
            }
        }

        let passed = failed.is_empty();
        let verdict = if passed {
            format!(
                "Live Context Integrity Gate PASS: {} records checked, no placeholder values detected.",
                records.len()
            )
        } else {
            format!(
                "Live Context Integrity Gate FAIL: {} records checked, placeholder values detected:\n{}",
                records.len(),
                failed.join("\n")
            )
        };

        Ok(execution_replay::ContextIntegrityValidation {
            total_records: records.len(),
            fields: vec![],
            passed,
            verdict,
        })
    }
}

fn default_calibration_experiments() -> Vec<execution_replay::CalibrationExperiment> {
    use execution_replay::{CalibrationExperiment, CalibrationPolicy, CalibrationPolicyKind};
    vec![
        CalibrationExperiment {
            id: "baseline".into(),
            policy: CalibrationPolicy {
                name: "Baseline 0.60".into(),
                description: "Current default confidence threshold.".into(),
                kind: CalibrationPolicyKind::Uniform { confidence_threshold: 0.6 },
            },
        },
        CalibrationExperiment {
            id: "c1".into(),
            policy: CalibrationPolicy {
                name: "C1: Uniform 0.55".into(),
                description: "Slightly lower confidence threshold for both sides.".into(),
                kind: CalibrationPolicyKind::Uniform { confidence_threshold: 0.55 },
            },
        },
        CalibrationExperiment {
            id: "c2".into(),
            policy: CalibrationPolicy {
                name: "C2: Uniform 0.50".into(),
                description: "Moderately lower confidence threshold.".into(),
                kind: CalibrationPolicyKind::Uniform { confidence_threshold: 0.50 },
            },
        },
        CalibrationExperiment {
            id: "c3".into(),
            policy: CalibrationPolicy {
                name: "C3: Uniform 0.45".into(),
                description: "Aggressively lower confidence threshold.".into(),
                kind: CalibrationPolicyKind::Uniform { confidence_threshold: 0.45 },
            },
        },
        CalibrationExperiment {
            id: "asymmetric".into(),
            policy: CalibrationPolicy {
                name: "Asymmetric 0.60/0.50".into(),
                description: "Buy-side confidence stays at 0.6, reduce-side confidence drops to 0.5.".into(),
                kind: CalibrationPolicyKind::Directional {
                    buy_confidence_threshold: 0.6,
                    reduce_confidence_threshold: 0.5,
                },
            },
        },
    ]
}


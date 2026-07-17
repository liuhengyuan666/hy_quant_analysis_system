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
    fetch_daily_bars_for_symbols_in_range, fetch_latest_market_regime_on_or_before,
    fetch_latest_strategy_state_on_or_before, fetch_signal_snapshots_for_range_with_scope,
    fetch_signal_snapshot_for_symbol, StorageConfig,
};
use research_context::{
    BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
};
use std::collections::BTreeMap;

use crate::core::instrument_in_scope;
use crate::AppContext;
use crate::ReportScope;

/// Builds an `ExecutionEvent` from persisted market data for a historical case.
///
/// This is a validation helper. It intentionally fails loud if any required input
/// is missing.
fn build_execution_event(
    storage: &StorageConfig,
    symbol: &str,
    date: NaiveDate,
    scope: AnalysisScope,
    strategy_state: Option<core_domain::StrategyStateSnapshot>,
) -> Result<ExecutionEvent> {
    let signal = fetch_signal_snapshot_for_symbol(storage, date, symbol, scope)
        .context("failed to fetch signal snapshot")?
        .with_context(|| format!("no signal snapshot for {} on {:?}", symbol, date))?;

    let strategy_state = match strategy_state {
        Some(s) => s,
        None => fetch_latest_strategy_state_on_or_before(storage, date, scope)
            .context("failed to fetch strategy state")?
            .with_context(|| format!("no strategy state for {:?} on or before {:?}", scope, date))?,
    };

    let bars = fetch_daily_bars_for_symbols_in_range(storage, &[symbol.to_string()], date, date)
        .context("failed to fetch daily bar")?;
    let bar = bars
        .into_iter()
        .next()
        .with_context(|| format!("no daily bar for {} on {:?}", symbol, date))?;

    let quote = QuoteSnapshot {
        symbol: bar.symbol.clone(),
        ts: chrono::Utc::now(), // historical replay uses current timestamp for artifact provenance
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
        volume: bar.volume,
        prev_close: bar.close, // TODO: fetch previous close
    };

    // Fetch the real market regime label from the Research/Regime layer.
    // Execution Platform does not generate or interpret regime; it preserves
    // whatever ResearchContext.market_state.label provides. If no regime is
    // available, fall back to "Unknown" so the lineage remains explicit.
    let market_regime_label = fetch_latest_market_regime_on_or_before(storage, date, scope)
        .context("failed to fetch market regime")?
        .map(|r| r.regime_label)
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| "Unknown".to_string());

    let market_view = ExecutionMarketView {
        research_version: "1".into(),
        market_regime_label,
        confirmation: ConfirmationSummary {
            trend: ConfirmationDimension {
                score: 50.0,
                label: "Moderate".into(),
            },
            participation: ConfirmationDimension {
                score: 50.0,
                label: "Moderate".into(),
            },
            risk: ConfirmationDimension {
                score: 50.0,
                label: "Moderate".into(),
            },
            overall: "Moderate".into(),
        },
        breadth: BreadthSummary {
            breadth_pct: 50.0,
            sma5: None,
            delta_5d: None,
            condition: "moderate".into(),
        },
        recovery: RecoverySummary {
            score: 50.0,
            drivers: vec![],
        },
        rotation_state: "mixed".into(),
        leadership_stability: 0.5,
    };

    let request = ExecutionRequest {
        symbol: symbol.to_string(),
        date,
        signal,
        strategy_state,
        quote,
        volume_ma20: 1.0, // TODO: fetch real volume MA20
        market_view,
        policy: ExecutionPolicy::default(),
    };

    Ok(DefaultExecutionPipeline.execute(request))
}

/// Orchestrates a single historical replay for a symbol/date.
pub fn run_single_execution_replay(
    storage: &StorageConfig,
    symbol: &str,
    date: NaiveDate,
    scope: AnalysisScope,
) -> Result<ExecutionResearchRecord> {
    let event = build_execution_event(storage, symbol, date, scope, None)?;
    let resolver = MarketStoreOutcomeResolver::new(storage.clone());
    let evaluator = RuleBasedEvaluationEngine;
    let as_of = date
        .checked_add_signed(chrono::Duration::days(180))
        .context("failed to compute as-of date")?;

    replay_single(&resolver, &evaluator, &event, as_of)
        .context("replay failed")
}

impl AppContext {
    /// Public entry point for running a single execution replay validation.
    pub fn validate_execution_replay(
        &self,
        symbol: &str,
        date: NaiveDate,
        scope: AnalysisScope,
    ) -> Result<ExecutionResearchRecord> {
        run_single_execution_replay(&self.storage, symbol, date, scope)
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

            match build_execution_event(&self.storage, &symbol, date, scope_value, Some(state.clone())) {
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
            build_execution_event(&self.storage, &case.symbol, case.date, scope, None)
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
        for case in &suite.cases {
            let scope = match case.scope.to_lowercase().as_str() {
                "cn" => AnalysisScope::Cn,
                "hk" => AnalysisScope::Hk,
                "global" => AnalysisScope::Global,
                other => {
                    anyhow::bail!("unknown scope: {}", other);
                }
            };
            let event = build_execution_event(&self.storage, &case.symbol, case.date, scope, None)?;
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
        let scope_value = match scope {
            ReportScope::Global => AnalysisScope::Global,
            ReportScope::Cn => AnalysisScope::Cn,
            ReportScope::Hk => AnalysisScope::Hk,
        };

        let universe = load_universe(&self.storage.universe_abspath()?)?;
        let symbols: Vec<String> = universe
            .into_iter()
            .filter(|instrument| instrument.enabled && instrument_in_scope(instrument, scope))
            .map(|instrument| instrument.symbol)
            .collect();

        let signals = fetch_signal_snapshots_for_range_with_scope(&self.storage, scope_value, from, to)?;
        let mut records = Vec::new();
        let mut state_cache: BTreeMap<NaiveDate, core_domain::StrategyStateSnapshot> = BTreeMap::new();

        let resolver = MarketStoreOutcomeResolver::new(self.storage.clone());
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
                None => match fetch_latest_strategy_state_on_or_before(&self.storage, date, scope_value)? {
                    Some(s) => {
                        state_cache.insert(date, s.clone());
                        s
                    }
                    None => continue,
                },
            };

            let event = match build_execution_event(&self.storage, &symbol, date, scope_value, Some(state)) {
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

        let mut stats = compute_execution_statistics(&records);
        stats.meta.scope = Some(format!("{:?}", scope));
        stats.meta.from_date = Some(from);
        stats.meta.to_date = Some(to);
        Ok(stats)
    }
}

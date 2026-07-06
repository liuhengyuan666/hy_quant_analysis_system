use chrono::{Duration, NaiveDate};
use core_domain::{Instrument, InstrumentType};
use core_domain::AnalysisScope as ReportScope;
use market_store::StorageConfig;
use report_engine::{DashboardSnapshot, DataHealthSummary, TrustSummary};

use crate::{
    LatestGateStageExplanation, PipelineDateDiagnostics, PipelineStageDateStatus,
    RefreshLatestDateStatus, ScopedPipelineDiagnostics,
    CALENDAR_GAP_REVIEW_THRESHOLD_DAYS, REFRESH_BOOTSTRAP_LOOKBACK_DAYS,
    REFRESH_GATE_REPAIR_WINDOW_DAYS, REFRESH_SOURCE_LOOKBACK_DAYS,
};

pub(crate) fn build_trust_summary(
    scoped_instruments: &[Instrument],
    snapshot: &DashboardSnapshot,
    pipeline_dates: &PipelineDateDiagnostics,
    data_health: Option<&DataHealthSummary>,
    calendar: &core_domain::calendar::TradingCalendar,
    storage: &StorageConfig,
) -> TrustSummary {
    let freshest_market_date = data_health
        .and_then(|dh| dh.freshest_market_date)
        .or_else(|| {
            pipeline_dates
                .freshest_market_date
                .as_deref()
                .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
        });
    let trading_instruments: Vec<_> = match freshest_market_date {
        Some(date) => scoped_instruments
            .iter()
            .filter(|i| calendar.is_trading_day(&i.market, date))
            .collect(),
        None => Vec::new(),
    };
    let non_trading_count = scoped_instruments
        .len()
        .saturating_sub(trading_instruments.len());
    let scoped_symbols_expected = trading_instruments.len();
    let scoped_symbols_on_freshest_market_date = match freshest_market_date {
        Some(date) => {
            let symbols: Vec<String> = trading_instruments
                .iter()
                .map(|i| i.symbol.clone())
                .collect();
            market_store::fetch_distinct_entity_count_for_date_in_symbols(
                storage,
                "daily_bar",
                "symbol",
                &symbols,
                date,
            )
            .unwrap_or(0)
        }
        None => 0,
    };
    let latest_day_complete = scoped_symbols_expected > 0
        && scoped_symbols_on_freshest_market_date == scoped_symbols_expected;
    let macro_status = match data_health {
        Some(dh) if dh.critical_macro_sources > 0 => "critical".to_string(),
        Some(dh) if dh.review_macro_sources > 0 => "review".to_string(),
        Some(_) => "healthy".to_string(),
        None => "unknown".to_string(),
    };

    let signal_analysis_scope = snapshot
        .top_signals
        .first()
        .map(|row| row.analysis_scope.clone());
    let signal_regime_basis_scope = snapshot
        .top_signals
        .first()
        .map(|row| row.regime_basis_scope.clone());
    let backtest_matches_snapshot = snapshot.latest_backtest.as_ref().map(|backtest| {
        backtest
            .analysis_scope
            .eq_ignore_ascii_case(&snapshot.scope)
            && backtest.signal_scope.eq_ignore_ascii_case(&snapshot.scope)
            && backtest
                .signal_end_date
                .map(|date| date.to_string())
                .as_deref()
                == Some(snapshot.report_date.as_str())
    });

    let pipeline_partial_latest = pipeline_dates
        .stages
        .iter()
        .any(|stage| stage.is_latest && stage.is_complete == Some(false));
    let pipeline_partial_latest_stage_count = pipeline_dates
        .stages
        .iter()
        .filter(|stage| stage.is_latest && stage.is_complete == Some(false))
        .count();
    let pipeline_stale = pipeline_dates.stages.iter().any(|stage| {
        matches!(stage.lag_days, Some(lag) if lag > 0)
            && matches!(
                stage.stage.as_str(),
                "market_regime"
                    | "environment_snapshot"
                    | "strategy_state"
                    | "strategy_preference"
                    | "signal_snapshot"
            )
    });
    let pipeline_stale_stage_count = pipeline_dates
        .stages
        .iter()
        .filter(|stage| {
            matches!(stage.lag_days, Some(lag) if lag > 0)
                && matches!(
                    stage.stage.as_str(),
                    "market_regime"
                        | "environment_snapshot"
                        | "strategy_state"
                        | "strategy_preference"
                        | "signal_snapshot"
                )
        })
        .count();

    let mut notes = Vec::new();
    if data_health.is_none() {
        notes.push(
            "Full data health check (provider probes, macro source status) is deferred for dashboard performance; coverage is computed from ClickHouse. Run `check-data-health` for complete health report."
                .to_string(),
        );
    }
    if non_trading_count > 0 {
        notes.push(format!(
            "{} symbol(s) were on non-trading markets on the freshest market date and were excluded from coverage checks.",
            non_trading_count
        ));
    }
    if !latest_day_complete {
        notes.push(
            "Latest market date is not fully covered across the active trading universe."
                .to_string(),
        );
    }
    if matches!(data_health, Some(dh) if dh.review_macro_sources > 0) {
        notes.push(
            "One or more macro sources are currently using review/fallback transport.".to_string(),
        );
    }
    if matches!(data_health, Some(dh) if dh.critical_macro_sources > 0) {
        notes.push("One or more macro sources are currently unavailable.".to_string());
    }
    if pipeline_partial_latest {
        notes.push("At least one pipeline stage is only partially complete on the freshest available date.".to_string());
    }
    if pipeline_stale {
        notes.push(
            "One or more decision stages are lagging behind the freshest market date.".to_string(),
        );
    }
    notes.extend(pipeline_dates.alerts.iter().cloned());
    if let (Some(signal_scope), Some(regime_scope)) = (
        signal_analysis_scope.as_ref(),
        signal_regime_basis_scope.as_ref(),
    ) {
        if !signal_scope.eq_ignore_ascii_case(&snapshot.scope)
            || !regime_scope.eq_ignore_ascii_case(&snapshot.scope)
        {
            notes.push(format!(
                "Signal analysis/regime basis still points to {} / {} while dashboard scope is {}.",
                signal_scope, regime_scope, snapshot.scope
            ));
        }
    }
    if backtest_matches_snapshot == Some(false) {
        notes.push(
            "Latest backtest does not match the current dashboard snapshot scope/date.".to_string(),
        );
    }

    // Signal-State Divergence observability (ADR-065 Shadow Production)
    if let Some(strategy_state) = &snapshot.strategy_state {
        let conservative_states = [
            core_domain::StrategyState::DeRisk,
            core_domain::StrategyState::NoTrade,
            core_domain::StrategyState::LeftProbe,
        ];
        if conservative_states.contains(&strategy_state.state) {
            let bullish_count = snapshot
                .top_signals
                .iter()
                .filter(|s| {
                    matches!(
                        s.signal_label,
                        core_domain::SignalLabel::StrongBuy | core_domain::SignalLabel::Buy
                    )
                })
                .count();
            if bullish_count > 0 {
                notes.push(format!(
                    "Signal-State Divergence: {} bullish signal(s) (StrongBuy/Buy) detected but StrategyState is {} ({:.0}%). This combination is being tracked during Shadow Production (see ADR-065).",
                    bullish_count,
                    strategy_state.state,
                    strategy_state.recommended_position_pct
                ));
            }
        }
    }

    if let Some(strategy_state) = &snapshot.strategy_state {
        notes.push(format!(
            "Strategy state {} recommends {:.2}% position as of {} ({}).",
            strategy_state.state,
            strategy_state.recommended_position_pct,
            strategy_state.date,
            strategy_state.transition_reason
        ));
    }

    let critical_macro = matches!(data_health, Some(dh) if dh.critical_macro_sources > 0);
    let review_macro = matches!(data_health, Some(dh) if dh.review_macro_sources > 0);

    let (level, headline, message) = if critical_macro || pipeline_stale {
        (
            "degraded",
            "Use with caution",
            "The current research view is usable, but freshness or macro availability issues reduce trust in the latest outputs.",
        )
    } else if !latest_day_complete
        || review_macro
        || pipeline_partial_latest
        || backtest_matches_snapshot == Some(false)
    {
        (
            "review",
            "Review before acting",
            "The pipeline completed, but coverage/provenance caveats should be reviewed before treating this as a clean research snapshot.",
        )
    } else {
        (
            "trusted",
            "Ready for analysis",
            "Freshness, provenance, and macro transport checks currently look healthy for this snapshot.",
        )
    };

    TrustSummary {
        level: level.to_string(),
        headline: headline.to_string(),
        message: message.to_string(),
        pipeline_has_partial_latest: pipeline_partial_latest,
        pipeline_has_stale_stage: pipeline_stale,
        pipeline_partial_latest_stage_count,
        pipeline_stale_stage_count,
        freshest_market_date: freshest_market_date.map(|date| date.to_string()),
        latest_available_date: Some(snapshot.latest_available_date.clone()),
        latest_day_complete,
        scoped_symbols_expected,
        scoped_symbols_on_freshest_market_date,
        macro_status,
        data_health_generated_at: data_health.map(|dh| dh.generated_at.clone()),
        data_health_review_symbols: data_health.map(|dh| dh.review_symbols),
        data_health_critical_symbols: data_health.map(|dh| dh.critical_symbols),
        data_health_review_macro_sources: data_health.map(|dh| dh.review_macro_sources),
        data_health_critical_macro_sources: data_health.map(|dh| dh.critical_macro_sources),
        signal_analysis_scope,
        signal_regime_basis_scope,
        strategy_state: snapshot
            .strategy_state
            .as_ref()
            .map(|row| row.state.to_string()),
        strategy_recommended_position_pct: snapshot
            .strategy_state
            .as_ref()
            .map(|row| row.recommended_position_pct),
        backtest_matches_snapshot,
        notes,
    }
}

pub(crate) fn pipeline_date_alerts(
    scope: ReportScope,
    stages: &[PipelineStageDateStatus],
) -> Vec<String> {
    let strategy_latest_date = stages
        .iter()
        .find(|stage| stage.stage == "strategy_preference")
        .and_then(|stage| stage.latest_date.clone());
    let signal_latest_date = stages
        .iter()
        .find(|stage| stage.stage == "signal_snapshot")
        .and_then(|stage| stage.latest_date.clone());

    let mut alerts = Vec::new();
    if let Some(issue) = build_signal_alignment_issue_for_dates(
        scope,
        strategy_latest_date
            .as_ref()
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        signal_latest_date
            .as_ref()
            .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
    ) {
        alerts.push(issue);
    }
    if let Some(issue) = build_signal_completeness_issue(stages) {
        alerts.push(issue);
    }
    alerts
}

pub(crate) fn latest_gate_alerts_for_scope(
    scope: ReportScope,
    before: &PipelineDateDiagnostics,
    after: &PipelineDateDiagnostics,
) -> Vec<String> {
    use crate::scope_label;

    let mut alerts = Vec::new();
    let scope_name = scope_label(scope);
    let before_latest = before.dashboard_latest_date.as_deref();
    let after_latest = after.dashboard_latest_date.as_deref();
    let freshest_market_date = after.freshest_market_date.as_deref();

    if before_latest == after_latest {
        match (after_latest, freshest_market_date) {
            (Some(latest), Some(freshest)) if freshest > latest => alerts.push(format!(
                "Latest available dashboard date for scope {} did not advance (still {}, freshest market date is {}).",
                scope_name, latest, freshest
            )),
            (None, Some(freshest)) => alerts.push(format!(
                "Scope {} still has no qualified dashboard date even though freshest market date is {}.",
                scope_name, freshest
            )),
            _ => {}
        }
    }

    for stage in after.stages.iter().filter(|stage| {
        stage.stage != "daily_bar"
            && stage.stage != "dashboard_available"
            && stage.is_latest
            && stage.is_complete == Some(false)
    }) {
        alerts.push(format!(
            "Stage {} is incomplete on the freshest available date for scope {} (actual {:?} / expected {:?}).",
            stage.stage, scope_name, stage.latest_entities, stage.expected_entities
        ));
    }

    for stage in after.stages.iter().filter(|stage| {
        stage.stage != "daily_bar"
            && stage.stage != "dashboard_available"
            && matches!(stage.lag_days, Some(lag) if lag > 0)
    }) {
        alerts.push(format!(
            "Stage {} is lagging by {} day(s) for scope {} (latest {}).",
            stage.stage,
            stage.lag_days.unwrap_or_default(),
            scope_name,
            stage.latest_date.as_deref().unwrap_or("N/A")
        ));
    }

    alerts
}

pub(crate) fn latest_gate_stage_explanations(
    diagnostics: &PipelineDateDiagnostics,
) -> Vec<LatestGateStageExplanation> {
    diagnostics
        .stages
        .iter()
        .map(|stage| {
            let reason = if stage.stage == "dashboard_available" {
                None
            } else if stage.is_latest && stage.is_complete == Some(false) {
                Some(format!(
                    "{} is incomplete on the freshest market date.",
                    stage.stage
                ))
            } else if matches!(stage.lag_days, Some(lag) if lag > 0) {
                Some(format!(
                    "{} is lagging behind the freshest market date by {} day(s).",
                    stage.stage,
                    stage.lag_days.unwrap_or_default()
                ))
            } else if stage.latest_date.is_none() {
                Some(format!("{} has no available rows yet.", stage.stage))
            } else {
                None
            };

            LatestGateStageExplanation {
                stage: stage.stage.clone(),
                latest_date: stage.latest_date.clone(),
                lag_days: stage.lag_days,
                is_latest: stage.is_latest,
                latest_entities: stage.latest_entities,
                expected_entities: stage.expected_entities,
                is_complete: stage.is_complete,
                blocking: reason.is_some() && stage.stage != "daily_bar",
                reason,
            }
        })
        .collect()
}

pub(crate) fn derive_refresh_window(
    to: NaiveDate,
    latest_daily_date: Option<NaiveDate>,
    latest_gated_dashboard_date: Option<NaiveDate>,
    has_missing_gated_scope: bool,
) -> (NaiveDate, String, i64) {
    let bootstrap_from = to - Duration::days(REFRESH_BOOTSTRAP_LOOKBACK_DAYS);

    match latest_daily_date {
        None => (
            bootstrap_from,
            "bootstrap".to_string(),
            REFRESH_GATE_REPAIR_WINDOW_DAYS,
        ),
        Some(latest_daily) => {
            let effective_to = std::cmp::max(to, latest_daily);
            let source_from = latest_daily - Duration::days(REFRESH_SOURCE_LOOKBACK_DAYS);
            let gated_repair_from = if has_missing_gated_scope {
                Some(effective_to - Duration::days(REFRESH_GATE_REPAIR_WINDOW_DAYS))
            } else {
                latest_gated_dashboard_date
                    .filter(|gated_latest| *gated_latest < latest_daily)
                    .map(|gated_latest| {
                        gated_latest - Duration::days(REFRESH_GATE_REPAIR_WINDOW_DAYS)
                    })
            };

            match gated_repair_from {
                Some(repair_from) => (
                    std::cmp::min(source_from, repair_from).max(bootstrap_from),
                    if has_missing_gated_scope {
                        "missing-gated-scope-repair".to_string()
                    } else {
                        "latest-gate-repair".to_string()
                    },
                    REFRESH_GATE_REPAIR_WINDOW_DAYS,
                ),
                None => (
                    source_from.max(bootstrap_from),
                    "source-lookback".to_string(),
                    REFRESH_GATE_REPAIR_WINDOW_DAYS,
                ),
            }
        }
    }
}

pub(crate) fn build_signal_alignment_issue_for_dates(
    scope: ReportScope,
    strategy_latest: Option<NaiveDate>,
    signal_latest: Option<NaiveDate>,
) -> Option<String> {
    use crate::scope_label;

    match (strategy_latest, signal_latest) {
        (Some(strategy_latest), Some(signal_latest)) if strategy_latest > signal_latest => Some(
            format!(
                "Signal snapshot for scope {} is lagging behind strategy preferences (signal={}, strategy={}). Rerun `compute-signals` before trusting dashboard/export defaults.",
                scope_label(scope), signal_latest, strategy_latest
            ),
        ),
        (Some(strategy_latest), None) => Some(format!(
            "Signal snapshot for scope {} is missing while strategy preferences already exist through {}. Rerun `compute-signals` before trusting dashboard/export defaults.",
            scope_label(scope), strategy_latest
        )),
        _ => None,
    }
}

pub(crate) fn build_signal_completeness_issue(
    stages: &[PipelineStageDateStatus],
) -> Option<String> {
    let signal_stage = stages
        .iter()
        .find(|stage| stage.stage == "signal_snapshot")?;
    match (
        signal_stage.latest_date.as_ref(),
        signal_stage.latest_entities,
        signal_stage.expected_entities,
        signal_stage.is_complete,
    ) {
        (Some(latest_date), Some(actual), Some(expected), Some(false)) if expected > 0 => Some(
            format!(
                "Signal snapshot is incomplete on its latest date {} ({}/{} symbols). Rerun `compute-signals` before trusting dashboard/export defaults.",
                latest_date, actual, expected
            ),
        ),
        _ => None,
    }
}

pub(crate) fn analyze_gap_metrics(
    bars: &[core_domain::DailyBar],
    instrument: &Instrument,
    calendar: &core_domain::calendar::TradingCalendar,
) -> (usize, i64) {
    let mut gap_count = 0usize;
    let mut max_gap_days = 0i64;
    for window in bars.windows(2) {
        let gap = (window[1].date - window[0].date).num_days();
        if gap <= 1 {
            continue;
        }
        let all_holidays = (1..gap).all(|offset| {
            let date = window[0].date + chrono::Duration::days(offset);
            !calendar.is_trading_day(&instrument.market, date)
        });
        if !all_holidays && gap > CALENDAR_GAP_REVIEW_THRESHOLD_DAYS {
            gap_count += 1;
            max_gap_days = max_gap_days.max(gap);
        }
    }
    (gap_count, max_gap_days)
}

pub(crate) fn analyze_jump_metrics(
    instrument: &Instrument,
    bars: &[core_domain::DailyBar],
) -> (usize, f64) {
    const REGISTRATION_BOARD_INDICES: &[&str] = &["000688", "000698", "399006", "399673"];
    let threshold = match instrument.instrument_type {
        InstrumentType::Index
            if REGISTRATION_BOARD_INDICES.contains(&instrument.symbol.as_str()) =>
        {
            0.22
        }
        InstrumentType::Index => 0.12,
        InstrumentType::Etf => 0.15,
    };
    let mut suspicious = 0usize;
    let mut max_abs_return = 0.0f64;
    for window in bars.windows(2) {
        let previous = window[0].close;
        let current = window[1].close;
        if previous <= 0.0 {
            continue;
        }
        let abs_return = ((current / previous) - 1.0).abs();
        max_abs_return = max_abs_return.max(abs_return * 100.0);
        if abs_return > threshold {
            suspicious += 1;
        }
    }
    (suspicious, max_abs_return)
}

pub(crate) fn classify_health(
    rows: usize,
    last_date: Option<NaiveDate>,
    now: NaiveDate,
    primary_provider_ok: bool,
    fallback_provider_ok: Option<bool>,
    gap_count: usize,
    suspicious_jump_count: usize,
) -> String {
    let freshness_days = last_date
        .map(|date| (now - date).num_days())
        .unwrap_or(i64::MAX);
    let has_recent_data = freshness_days <= 3;

    if rows == 0 {
        "critical".to_string()
    } else if !has_recent_data {
        "critical".to_string()
    } else if !primary_provider_ok && fallback_provider_ok != Some(true) {
        "review".to_string()
    } else if !primary_provider_ok || gap_count > 0 || suspicious_jump_count > 0 {
        "review".to_string()
    } else {
        "healthy".to_string()
    }
}

pub(crate) fn summarize_latest_dates(
    diagnostics: &[ScopedPipelineDiagnostics],
) -> Vec<RefreshLatestDateStatus> {
    diagnostics
        .iter()
        .map(|item| RefreshLatestDateStatus {
            scope: item.scope.clone(),
            freshest_market_date: item.diagnostics.freshest_market_date.clone(),
            dashboard_latest_date: item.diagnostics.dashboard_latest_date.clone(),
        })
        .collect()
}

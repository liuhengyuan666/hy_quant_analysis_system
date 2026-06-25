use anyhow::{Context, Result};
use chrono::NaiveDate;
use core_domain::Instrument;
use core_domain::AnalysisScope as ReportScope;
use std::time::Instant;

pub(crate) fn load_calendar_from_config(
    dir: &std::path::Path,
) -> core_domain::calendar::TradingCalendar {
    use std::collections::HashSet;

    let mut cn_holidays = HashSet::new();
    let mut hk_holidays = HashSet::new();

    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry_result in entries {
                let entry = match entry_result {
                    Ok(entry) => entry,
                    Err(error) => {
                        eprintln!("failed to read calendar config directory entry: {error}");
                        continue;
                    }
                };
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let content = match std::fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(error) => {
                        eprintln!("failed to read calendar config {}: {error}", path.display());
                        continue;
                    }
                };
                let config = match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(config) => config,
                    Err(error) => {
                        eprintln!(
                            "failed to parse calendar config {}: {error}",
                            path.display()
                        );
                        continue;
                    }
                };
                let (market, holidays) = match (
                    config.get("market").and_then(|m| m.as_str()),
                    config.get("holidays").and_then(|h| h.as_array()),
                ) {
                    (Some(market), Some(holidays)) => (market, holidays),
                    _ => {
                        eprintln!(
                            "calendar config {} is missing market or holidays",
                            path.display()
                        );
                        continue;
                    }
                };
                for holiday in holidays {
                    let Some(date_str) = holiday.as_str() else {
                        eprintln!(
                            "calendar config {} contains a non-string holiday",
                            path.display()
                        );
                        continue;
                    };
                    let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        Ok(date) => date,
                        Err(error) => {
                            eprintln!(
                                "calendar config {} contains invalid holiday {date_str}: {error}",
                                path.display()
                            );
                            continue;
                        }
                    };
                    match market {
                        "CN" => {
                            cn_holidays.insert(date);
                        }
                        "HK" => {
                            hk_holidays.insert(date);
                        }
                        other => eprintln!(
                            "calendar config {} uses unsupported market {other}",
                            path.display()
                        ),
                    }
                }
            }
        }
        Err(error) => eprintln!(
            "failed to read calendar config directory {}: {error}",
            dir.display()
        ),
    }

    core_domain::calendar::TradingCalendar::new(cn_holidays, hk_holidays)
}

pub(crate) fn format_error_chain(error: &anyhow::Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut current = error.source();
    while let Some(source) = current {
        parts.push(source.to_string());
        current = source.source();
    }
    parts.join(" | caused by: ")
}

pub(crate) fn validate_user_preference(key: &str, value: &str) -> Result<()> {
    const MAX_PREFERENCE_VALUE_LEN: usize = 32;
    if value.len() > MAX_PREFERENCE_VALUE_LEN {
        anyhow::bail!("user preference value is too long: {key}");
    }

    match key {
        "default_scope" => match value {
            "global" | "cn" | "hk" => Ok(()),
            _ => anyhow::bail!("unsupported default_scope preference value: {value}"),
        },
        "last_analysis_date" => {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .with_context(|| format!("invalid last_analysis_date preference value: {value}"))?;
            Ok(())
        }
        _ => anyhow::bail!("unsupported user preference key: {key}"),
    }
}

pub(crate) fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis() as u64
}

pub(crate) fn new_refresh_job_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub(crate) fn last_successful_stage(stages: &[crate::RefreshStageExecution]) -> Option<String> {
    stages
        .iter()
        .rev()
        .find(|stage| stage.status == "success")
        .map(|stage| stage.name.clone())
}

pub(crate) fn refresh_stage_order(stage: &str) -> Option<u8> {
    match stage {
        "ingest" => Some(0),
        "indicators" => Some(1),
        "macro" => Some(2),
        "rotation" => Some(3),
        "strategy" => Some(4),
        "signals" => Some(5),
        "backtests" => Some(6),
        _ => None,
    }
}

pub(crate) fn scope_label(scope: ReportScope) -> &'static str {
    scope.as_str()
}

pub(crate) fn instrument_in_scope(instrument: &Instrument, scope: ReportScope) -> bool {
    scope.matches_market(&instrument.market)
}

pub(crate) fn instrument_in_latest_gate_scope(instrument: &Instrument, scope: ReportScope) -> bool {
    instrument.enabled && instrument.latest_gate_required && instrument_in_scope(instrument, scope)
}

pub(crate) fn scope_universe_label(scope: ReportScope) -> &'static str {
    match scope {
        ReportScope::Global => "Global tracked universe",
        ReportScope::Cn => "CN tracked universe",
        ReportScope::Hk => "HK tracked universe",
    }
}

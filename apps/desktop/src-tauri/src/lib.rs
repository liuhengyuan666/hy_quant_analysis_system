use app_service::{pipeline_stages, AppContext};
use chrono::{Local, NaiveDate};
use core_domain::{LlmStatus};
use market_store::StorageConfig;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
struct DashboardRefreshStatus {
    running: bool,
    status: String,
    progress_pct: u8,
    stage: String,
    current_stage: Option<String>,
    start_stage: String,
    retry_from_stage: Option<String>,
    refresh_from: Option<String>,
    refresh_to: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    error: Option<String>,
    cancelling: bool,
    job_id: Option<String>,
    last_successful_stage: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DashboardBundlePayload {
    status: app_service::AppStatus,
    available_dates: Vec<String>,
    snapshot: Option<report_engine::DashboardSnapshot>,
    insight: Option<app_service::ResearchInsight>,
    recent_reports: Vec<app_service::RecentReportItem>,
    pipeline_dates: app_service::PipelineDateDiagnostics,
    refresh_status: DashboardRefreshStatus,
}

impl Default for DashboardRefreshStatus {
    fn default() -> Self {
        Self {
            running: false,
            status: "idle".to_string(),
            progress_pct: 0,
            stage: "Idle".to_string(),
            current_stage: None,
            start_stage: "full".to_string(),
            retry_from_stage: None,
            refresh_from: None,
            refresh_to: None,
            started_at: None,
            finished_at: None,
            error: None,
            cancelling: false,
            job_id: None,
            last_successful_stage: None,
        }
    }
}

#[derive(Clone, Default)]
struct RefreshCoordinator {
    status: Arc<Mutex<DashboardRefreshStatus>>,
    cancel_flag: Arc<AtomicBool>,
    current_job_id: Arc<Mutex<Option<String>>>,
}

fn set_refresh_status<F>(coordinator: &RefreshCoordinator, update: F)
where
    F: FnOnce(&mut DashboardRefreshStatus),
{
    if let Ok(mut status) = coordinator.status.lock() {
        update(&mut status);
    }
}

fn validate_report_artifact_path(artifact_path: &str) -> Result<PathBuf, String> {
    let requested = PathBuf::from(artifact_path);
    if requested.as_os_str().is_empty() {
        return Err("artifact path is empty".to_string());
    }

    let artifact = fs::canonicalize(&requested)
        .map_err(|error| format!("failed to resolve artifact path: {error}"))?;
    if !artifact.is_file() {
        return Err("artifact path does not point to a file".to_string());
    }

    let report_dir = StorageConfig::project_root()
        .map_err(|error| error.to_string())?
        .join("reports");
    let report_dir = fs::canonicalize(&report_dir)
        .map_err(|error| format!("failed to resolve reports directory: {error}"))?;

    if !artifact.starts_with(&report_dir) {
        return Err(format!(
            "artifact path is outside the managed reports directory: {}",
            report_dir.display()
        ));
    }

    if artifact
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("md")
    {
        return Err("only markdown report artifacts can be opened".to_string());
    }

    Ok(artifact)
}

fn validate_registered_report_artifact(artifact: &Path) -> Result<(), String> {
    let context = AppContext::new(StorageConfig::default());
    let reports = context
        .recent_reports(1000)
        .map_err(|error| error.to_string())?;
    for report in reports {
        if let Ok(registered) = fs::canonicalize(&report.artifact_path) {
            if registered == artifact {
                return Ok(());
            }
        }
    }
    Err("artifact is not registered in recent reports".to_string())
}

fn open_file_in_os(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .status()
            .map_err(|error| format!("failed to launch artifact: {error}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("artifact opener exited with status: {status}"));
    }

    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(path)
            .status()
            .map_err(|error| format!("failed to launch artifact: {error}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("artifact opener exited with status: {status}"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let status = Command::new("xdg-open")
            .arg(path)
            .status()
            .map_err(|error| format!("failed to launch artifact: {error}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("artifact opener exited with status: {status}"));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshStartStage {
    Ingest,
    Indicators,
    Macro,
    Rotation,
    Strategy,
    Signals,
    Backtests,
}

impl RefreshStartStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ingest => "ingest",
            Self::Indicators => "indicators",
            Self::Macro => "macro",
            Self::Rotation => "rotation",
            Self::Strategy => "strategy",
            Self::Signals => "signals",
            Self::Backtests => "backtests",
        }
    }

    fn display_label(self) -> &'static str {
        match self {
            Self::Ingest => "Daily bars",
            Self::Indicators => "Indicators",
            Self::Macro => "Macro regime",
            Self::Rotation => "Rotation",
            Self::Strategy => "Strategy preferences",
            Self::Signals => "Signals",
            Self::Backtests => "Backtests",
        }
    }

    fn progress_after(self) -> u8 {
        pipeline_stages::progress_after(self.as_str())
    }
}

fn parse_refresh_start_stage(stage: Option<&str>) -> Result<Option<RefreshStartStage>, String> {
    match stage.unwrap_or("full") {
        "full" => Ok(None),
        "ingest" => Ok(Some(RefreshStartStage::Ingest)),
        "indicators" => Ok(Some(RefreshStartStage::Indicators)),
        "macro" => Ok(Some(RefreshStartStage::Macro)),
        "rotation" => Ok(Some(RefreshStartStage::Rotation)),
        "strategy" => Ok(Some(RefreshStartStage::Strategy)),
        "signals" => Ok(Some(RefreshStartStage::Signals)),
        "backtests" => Ok(Some(RefreshStartStage::Backtests)),
        other => Err(format!("unsupported refresh start stage: {other}")),
    }
}

fn start_stage_value(start_stage: Option<RefreshStartStage>) -> String {
    start_stage
        .map(RefreshStartStage::as_str)
        .unwrap_or("full")
        .to_string()
}

fn stage_label_from_key(stage_key: &str) -> &'static str {
    match stage_key {
        "ingest" => "Daily bars",
        "indicators" => "Indicators",
        "macro" => "Macro regime",
        "rotation" => "Rotation",
        "strategy" => "Strategy preferences",
        "signals" => "Signals",
        "backtests" => "Backtests",
        _ => "Refresh",
    }
}

fn next_stage_after(stage_key: &str) -> Option<RefreshStartStage> {
    match stage_key {
        "ingest" => Some(RefreshStartStage::Indicators),
        "indicators" => Some(RefreshStartStage::Macro),
        "macro" => Some(RefreshStartStage::Rotation),
        "rotation" => Some(RefreshStartStage::Strategy),
        "strategy" => Some(RefreshStartStage::Signals),
        "signals" => Some(RefreshStartStage::Backtests),
        "backtests" => None,
        _ => Some(RefreshStartStage::Ingest),
    }
}

fn retry_stage_from_last_successful(stage_key: Option<&str>) -> Option<RefreshStartStage> {
    match stage_key {
        Some(stage) => next_stage_after(stage),
        None => Some(RefreshStartStage::Ingest),
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PersistedRefreshStage {
    name: String,
    status: String,
}

fn parse_refresh_stages(stages_json: &str) -> Result<Vec<PersistedRefreshStage>, String> {
    serde_json::from_str(stages_json)
        .map_err(|error| format!("failed to parse persisted refresh stages: {error}"))
}

fn last_successful_from_stages(stages: &[PersistedRefreshStage]) -> Option<String> {
    stages
        .iter()
        .rev()
        .find(|stage| stage.status == "success")
        .map(|stage| stage.name.clone())
}

fn failed_stage_from_stages(stages: &[PersistedRefreshStage]) -> Option<String> {
    stages
        .iter()
        .find(|stage| stage.status == "error")
        .map(|stage| stage.name.clone())
}

fn recoverable_persisted_refresh_status() -> Option<DashboardRefreshStatus> {
    let context = AppContext::new(StorageConfig::default());
    let job = context.latest_refresh_job().ok().flatten()?;
    if job.status != "cancelled" && job.status != "error" {
        return None;
    }

    let persisted_stages = parse_refresh_stages(&job.stages_json).ok()?;
    let last_successful_stage = job
        .last_successful_stage
        .clone()
        .or_else(|| last_successful_from_stages(&persisted_stages));
    let retry_from_stage = if job.status == "error" {
        failed_stage_from_stages(&persisted_stages)
            .and_then(|stage| parse_refresh_start_stage(Some(&stage)).ok().flatten())
    } else {
        retry_stage_from_last_successful(last_successful_stage.as_deref())
    }
    .map(|stage| stage.as_str().to_string());
    let progress_pct = last_successful_stage
        .as_deref()
        .and_then(|stage| parse_refresh_start_stage(Some(stage)).ok().flatten())
        .map(RefreshStartStage::progress_after)
        .unwrap_or(0);
    let stage = if job.status == "cancelled" {
        last_successful_stage
            .as_deref()
            .map(|stage| format!("Refresh cancelled after {}", stage_label_from_key(stage)))
            .unwrap_or_else(|| "Refresh cancelled before any stage completed".to_string())
    } else {
        retry_from_stage
            .as_deref()
            .map(|stage| format!("{} failed", stage_label_from_key(stage)))
            .unwrap_or_else(|| "Refresh failed".to_string())
    };

    Some(DashboardRefreshStatus {
        running: false,
        status: job.status,
        progress_pct,
        stage,
        current_stage: None,
        start_stage: "full".to_string(),
        retry_from_stage,
        refresh_from: job.refresh_from,
        refresh_to: job.refresh_to,
        started_at: Some(job.started_at),
        finished_at: job.finished_at,
        error: job.error,
        cancelling: false,
        job_id: Some(job.id),
        last_successful_stage,
    })
}

fn visible_refresh_status(
    coordinator: &RefreshCoordinator,
) -> Result<DashboardRefreshStatus, String> {
    let status = coordinator
        .status
        .lock()
        .map(|status| status.clone())
        .map_err(|error| error.to_string())?;
    if status.status == "idle" {
        Ok(recoverable_persisted_refresh_status().unwrap_or(status))
    } else {
        Ok(status)
    }
}

fn spawn_dashboard_refresh(
    coordinator: RefreshCoordinator,
    start_stage: Option<RefreshStartStage>,
) -> Result<DashboardRefreshStatus, String> {
    let started_at = Local::now().to_rfc3339();
    let start_stage_value = start_stage_value(start_stage);
    let prep_label = start_stage
        .map(|stage| format!("Preparing rerun from {}", stage.display_label()))
        .unwrap_or_else(|| "Preparing refresh window".to_string());
    {
        let mut current = coordinator
            .status
            .lock()
            .map_err(|error| error.to_string())?;
        if current.running {
            return Ok(current.clone());
        }
        coordinator.cancel_flag.store(false, Ordering::Relaxed);
        *current = DashboardRefreshStatus {
            running: true,
            status: "running".to_string(),
            progress_pct: 0,
            stage: prep_label,
            current_stage: None,
            start_stage: start_stage_value.clone(),
            retry_from_stage: None,
            refresh_from: None,
            refresh_to: None,
            started_at: Some(started_at.clone()),
            finished_at: None,
            error: None,
            cancelling: false,
            job_id: None,
            last_successful_stage: None,
        };
    }

    if let Ok(mut current_job_id) = coordinator.current_job_id.lock() {
        *current_job_id = None;
    }

    let worker = coordinator.clone();
    std::thread::spawn(move || {
        let context = AppContext::new(StorageConfig::default());
        let today = Local::now().date_naive();

        let progress_worker = worker.clone();
        let progress_callback: Option<Box<dyn Fn(&str) + Send>> =
            Some(Box::new(move |msg: &str| {
                if let Some(start) = msg.find("Starting ") {
                    let rest = &msg[start + "Starting ".len()..];
                    let stage_name = rest.trim_end_matches("...").trim_end_matches(".");
                    let pct = pipeline_stages::progress_after(stage_name);
                    if pct > 0 {
                        set_refresh_status(&progress_worker, |status| {
                            status.progress_pct = pct;
                            status.stage = stage_name.to_string();
                            status.current_stage = Some(stage_name.to_string());
                        });
                    }
                }
            }));

        let result = context.refresh_pipeline(
            today,
            app_service::ReportScope::Global,
            true,
            Some(worker.cancel_flag.as_ref()),
            start_stage.map(RefreshStartStage::as_str),
            progress_callback,
        );

        match result {
            Ok(summary) if summary.cancelled => {
                let finished_at = Local::now().to_rfc3339();
                let last_successful_stage = summary
                    .stages
                    .iter()
                    .rev()
                    .find(|stage| stage.status == "success")
                    .map(|stage| stage.name.clone());
                let retry_from_stage =
                    retry_stage_from_last_successful(last_successful_stage.as_deref())
                        .map(|stage| stage.as_str().to_string());
                if let Ok(mut current_job_id) = worker.current_job_id.lock() {
                    *current_job_id = Some(summary.job_id.clone());
                }
                set_refresh_status(&worker, |status| {
                    status.running = false;
                    status.status = "cancelled".to_string();
                    status.finished_at = Some(finished_at);
                    status.error = summary.alerts.blocking.first().cloned();
                    status.current_stage = None;
                    status.stage = last_successful_stage
                        .as_deref()
                        .map(|stage| {
                            format!("Refresh cancelled after {}", stage_label_from_key(stage))
                        })
                        .unwrap_or_else(|| {
                            "Refresh cancelled before any stage completed".to_string()
                        });
                    status.retry_from_stage = retry_from_stage;
                    status.job_id = Some(summary.job_id.clone());
                    status.last_successful_stage = last_successful_stage;
                    status.cancelling = false;
                    status.refresh_from = Some(summary.refresh_window.refresh_from.clone());
                    status.refresh_to = Some(summary.refresh_window.refresh_to.clone());
                });
            }
            Ok(summary) if summary.success => {
                let finished_at = Local::now().to_rfc3339();
                if let Ok(mut current_job_id) = worker.current_job_id.lock() {
                    *current_job_id = Some(summary.job_id.clone());
                }
                set_refresh_status(&worker, |status| {
                    status.running = false;
                    status.status = "success".to_string();
                    status.progress_pct = 100;
                    status.stage = "Refresh consistency verified".to_string();
                    status.finished_at = Some(finished_at);
                    status.error = None;
                    status.current_stage = None;
                    status.retry_from_stage = None;
                    status.job_id = Some(summary.job_id.clone());
                    status.last_successful_stage = summary
                        .stages
                        .iter()
                        .rev()
                        .find(|stage| stage.status == "success")
                        .map(|stage| stage.name.clone());
                    status.cancelling = false;
                    status.refresh_from = Some(summary.refresh_window.refresh_from.clone());
                    status.refresh_to = Some(summary.refresh_window.refresh_to.clone());
                });
            }
            Ok(summary) => {
                let finished_at = Local::now().to_rfc3339();
                let retry_from_stage = summary
                    .stages
                    .iter()
                    .find(|stage| stage.status == "error")
                    .map(|stage| stage.name.clone())
                    .or_else(|| {
                        retry_stage_from_last_successful(
                            summary
                                .stages
                                .iter()
                                .rev()
                                .find(|stage| stage.status == "success")
                                .map(|stage| stage.name.as_str()),
                        )
                        .map(|stage| stage.as_str().to_string())
                    });
                let last_successful_stage = summary
                    .stages
                    .iter()
                    .rev()
                    .find(|stage| stage.status == "success")
                    .map(|stage| stage.name.clone());
                let error = summary.alerts.blocking.join(" | ");
                if let Ok(mut current_job_id) = worker.current_job_id.lock() {
                    *current_job_id = Some(summary.job_id.clone());
                }
                set_refresh_status(&worker, |status| {
                    let stage_label = retry_from_stage
                        .as_deref()
                        .map(stage_label_from_key)
                        .unwrap_or("Refresh consistency validation");
                    status.running = false;
                    status.status = "error".to_string();
                    status.stage = format!("{} failed", stage_label);
                    status.finished_at = Some(finished_at);
                    status.error = Some(error);
                    status.retry_from_stage = retry_from_stage;
                    status.job_id = Some(summary.job_id.clone());
                    status.last_successful_stage = last_successful_stage;
                    status.cancelling = false;
                    status.refresh_from = Some(summary.refresh_window.refresh_from.clone());
                    status.refresh_to = Some(summary.refresh_window.refresh_to.clone());
                });
            }
            Err(error) => {
                let finished_at = Local::now().to_rfc3339();
                set_refresh_status(&worker, |status| {
                    let retry_from_stage = status.retry_from_stage.clone();
                    let stage_label = retry_from_stage
                        .as_deref()
                        .map(stage_label_from_key)
                        .unwrap_or("Refresh");
                    status.running = false;
                    status.status = "error".to_string();
                    status.stage = format!("{} failed", stage_label);
                    status.finished_at = Some(finished_at);
                    status.error = Some(error.to_string());
                    status.retry_from_stage = retry_from_stage;
                    status.cancelling = false;
                });
            }
        }
    });

    coordinator
        .status
        .lock()
        .map(|status| status.clone())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn app_status() -> Result<app_service::AppStatus, String> {
    let context = AppContext::new(StorageConfig::default());
    context.status().map_err(|error| error.to_string())
}

#[tauri::command]
async fn dashboard_bundle(
    refresh: tauri::State<'_, RefreshCoordinator>,
    report_date: Option<String>,
    scope: Option<String>,
    recent_report_limit: Option<usize>,
) -> Result<DashboardBundlePayload, String> {
    let parsed_date = report_date
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .transpose()
        .map_err(|error| error.to_string())?;
    let limit = recent_report_limit.unwrap_or(10);
    let parsed_scope = match scope.as_deref().unwrap_or("global") {
        "global" => app_service::ReportScope::Global,
        "cn" => app_service::ReportScope::Cn,
        "hk" => app_service::ReportScope::Hk,
        other => return Err(format!("unsupported scope: {other}")),
    };
    let bundle = tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.dashboard_bundle_with_scope(parsed_date, parsed_scope, limit)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())?;

    let refresh_status = visible_refresh_status(refresh.inner())?;

    Ok(DashboardBundlePayload {
        status: bundle.status,
        available_dates: bundle.available_dates,
        snapshot: bundle.snapshot,
        insight: bundle.insight,
        recent_reports: bundle.recent_reports,
        pipeline_dates: bundle.pipeline_dates,
        refresh_status,
    })
}

#[tauri::command]
async fn dashboard_snapshot(
    report_date: Option<String>,
    scope: Option<String>,
) -> Result<Option<report_engine::DashboardSnapshot>, String> {
    let parsed_date = report_date
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .transpose()
        .map_err(|error| error.to_string())?;
    let parsed_scope = match scope.as_deref().unwrap_or("global") {
        "global" => app_service::ReportScope::Global,
        "cn" => app_service::ReportScope::Cn,
        "hk" => app_service::ReportScope::Hk,
        other => return Err(format!("unsupported scope: {other}")),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.dashboard_snapshot_with_scope(parsed_date, parsed_scope)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn dashboard_available_dates(scope: Option<String>) -> Result<Vec<String>, String> {
    let parsed_scope = match scope.as_deref().unwrap_or("global") {
        "global" => app_service::ReportScope::Global,
        "cn" => app_service::ReportScope::Cn,
        "hk" => app_service::ReportScope::Hk,
        other => return Err(format!("unsupported scope: {other}")),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.dashboard_available_dates_with_scope(parsed_scope)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_report(
    report_date: Option<String>,
    scope: Option<String>,
) -> Result<app_service::ReportSummary, String> {
    let parsed_date = report_date
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .transpose()
        .map_err(|error| error.to_string())?;
    let parsed_scope = match scope.as_deref().unwrap_or("global") {
        "global" => app_service::ReportScope::Global,
        "cn" => app_service::ReportScope::Cn,
        "hk" => app_service::ReportScope::Hk,
        other => return Err(format!("unsupported scope: {other}")),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.export_report_with_scope(parsed_date, parsed_scope)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_signal_detail(
    symbol: String,
    scope: Option<String>,
    date: Option<String>,
) -> Result<Option<core_domain::SignalSnapshot>, String> {
    let parsed_date = date
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .transpose()
        .map_err(|error| error.to_string())?;
    let parsed_scope = match scope.as_deref().unwrap_or("global") {
        "global" => app_service::ReportScope::Global,
        "cn" => app_service::ReportScope::Cn,
        "hk" => app_service::ReportScope::Hk,
        other => return Err(format!("unsupported scope: {other}")),
    };
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.get_signal_detail(
            parsed_scope,
            &symbol,
            parsed_date.unwrap_or_else(|| chrono::Utc::now().date_naive()),
        )
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn data_health_summary() -> Result<report_engine::DataHealthSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.check_data_health()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_data_health_report() -> Result<app_service::ReportSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.export_data_health_report()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn recent_reports(
    limit: Option<usize>,
) -> Result<Vec<app_service::RecentReportItem>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.recent_reports(limit.unwrap_or(10))
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_report_artifact(artifact_path: String) -> Result<(), String> {
    let artifact = validate_report_artifact_path(&artifact_path)?;
    validate_registered_report_artifact(&artifact)?;
    open_file_in_os(&artifact)
}

#[tauri::command]
async fn usage_guides() -> Result<Vec<app_service::UsageGuide>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.usage_guides()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn dashboard_refresh_status(
    refresh: tauri::State<RefreshCoordinator>,
) -> Result<DashboardRefreshStatus, String> {
    visible_refresh_status(refresh.inner())
}

#[tauri::command]
fn start_dashboard_refresh(
    refresh: tauri::State<RefreshCoordinator>,
    start_stage: Option<String>,
) -> Result<DashboardRefreshStatus, String> {
    let parsed_start_stage = parse_refresh_start_stage(start_stage.as_deref())?;
    spawn_dashboard_refresh(refresh.inner().clone(), parsed_start_stage)
}

#[tauri::command]
fn cancel_dashboard_refresh(refresh: tauri::State<RefreshCoordinator>) -> Result<(), String> {
    refresh.cancel_flag.store(true, Ordering::Relaxed);
    set_refresh_status(refresh.inner(), |status| {
        if status.running {
            status.cancelling = true;
            status.stage = "Cancelling after current stage completes...".to_string();
        }
    });
    Ok(())
}

#[tauri::command]
fn get_user_preferences() -> Result<BTreeMap<String, String>, String> {
    let context = AppContext::new(StorageConfig::default());
    context
        .get_all_user_preferences()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn check_startup_freshness() -> Result<serde_json::Value, String> {
    let context = AppContext::new(StorageConfig::default());
    let check = context.check_startup_freshness().map_err(|e| e.to_string())?;
    serde_json::to_value(check).map_err(|e| e.to_string())
}

#[tauri::command]
async fn auto_ingest_on_startup(
    refresh: tauri::State<'_, RefreshCoordinator>,
) -> Result<DashboardRefreshStatus, String> {
    let context = AppContext::new(StorageConfig::default());
    let check = context.check_startup_freshness().map_err(|e| e.to_string())?;
    
    if !check.auto_ingest_eligible {
        return Ok(DashboardRefreshStatus {
            status: "skipped".to_string(),
            ..Default::default()
        });
    }
    
    // Compute date range for the gap
    let now = chrono::Local::now();
    let expected_date = context.calendar.expected_latest_tradable_date(now)
        .ok_or_else(|| "无法确定期望最新日期".to_string())?;
    let latest_db_date = market_store::fetch_latest_daily_bar_date(&context.storage)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "数据库中无数据".to_string())?;
    let from = latest_db_date + chrono::Duration::days(1);
    let to = expected_date;
    
    // Set initial status
    set_refresh_status(refresh.inner(), |status| {
        status.running = true;
        status.status = "running".to_string();
        status.stage = "auto_ingest".to_string();
        status.progress_pct = 0;
        status.started_at = Some(chrono::Local::now().to_rfc3339());
    });
    
    let coordinator = refresh.inner().clone();
    let coordinator_for_success = coordinator.clone();
    let coordinator_for_error = coordinator.clone();
    
    // Spawn the parallel ingest task in background
    tauri::async_runtime::spawn(async move {
        let progress = Box::new(move |msg: &str| {
            set_refresh_status(&coordinator, |status| {
                status.status = msg.to_string();
                if msg.contains("ingest progress") {
                    if let Some(pct_start) = msg.rfind('(') {
                        if let Some(pct_end) = msg.rfind('%') {
                            if let Ok(pct) = msg[pct_start+1..pct_end].parse::<u8>() {
                                status.progress_pct = pct;
                            }
                        }
                    }
                }
            });
        }) as Box<dyn Fn(&str) + Send>;
        
        let context = AppContext::new(StorageConfig::default());
        match context.ingest_daily_parallel(from, to, Some(progress)).await {
            Ok(_summary) => {
                set_refresh_status(&coordinator_for_success, |status| {
                    status.running = false;
                    status.status = "success".to_string();
                    status.progress_pct = 100;
                    status.finished_at = Some(chrono::Local::now().to_rfc3339());
                });
            }
            Err(error) => {
                set_refresh_status(&coordinator_for_error, |status| {
                    status.running = false;
                    status.status = "error".to_string();
                    status.error = Some(error.to_string());
                    status.finished_at = Some(chrono::Local::now().to_rfc3339());
                });
            }
        }
    });
    
    // Return current status immediately
    let status = refresh.status.lock().map_err(|e| e.to_string())?;
    Ok(status.clone())
}

#[tauri::command]
fn set_user_preference(key: String, value: String) -> Result<(), String> {
    let context = AppContext::new(StorageConfig::default());
    context
        .set_user_preference(&key, &value)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn retry_dashboard_refresh(
    refresh: tauri::State<RefreshCoordinator>,
) -> Result<DashboardRefreshStatus, String> {
    let retry_stage = if let Some(stage) = {
        let status = refresh
            .status
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        parse_refresh_start_stage(status.retry_from_stage.as_deref())?
    } {
        stage
    } else {
        let context = AppContext::new(StorageConfig::default());
        let job = context
            .latest_refresh_job()
            .map_err(|error| error.to_string())?
            .filter(|job| job.status == "cancelled" || job.status == "error")
            .ok_or_else(|| {
                "no failed or cancelled refresh job is available to resume".to_string()
            })?;
        let persisted_stages = parse_refresh_stages(&job.stages_json)?;
        let retry_stage = if job.status == "error" {
            failed_stage_from_stages(&persisted_stages)
                .and_then(|stage| parse_refresh_start_stage(Some(&stage)).ok().flatten())
        } else {
            let persisted_last_successful = last_successful_from_stages(&persisted_stages);
            retry_stage_from_last_successful(
                job.last_successful_stage
                    .as_deref()
                    .or(persisted_last_successful.as_deref()),
            )
        };
        retry_stage.ok_or_else(|| {
            "no resumable stage is available for the latest refresh job".to_string()
        })?
    };
    spawn_dashboard_refresh(refresh.inner().clone(), Some(retry_stage))
}

#[tauri::command]
fn get_llm_status() -> Result<LlmStatus, String> {
    let context = AppContext::new(StorageConfig::default());
    context.get_llm_status().map_err(|e| e.to_string())
}

#[tauri::command]
async fn analyze_with_llm(
    scope: Option<String>,
    action: String,
) -> Result<serde_json::Value, String> {
    let scope = scope.unwrap_or_else(|| "global".to_string());
    let report_scope = match scope.as_str() {
        "cn" => app_service::ReportScope::Cn,
        "hk" => app_service::ReportScope::Hk,
        _ => app_service::ReportScope::Global,
    };

    let context = AppContext::new(StorageConfig::default());
    context
        .analyze_with_action(&action, report_scope)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_llm_config(
    base_url: String,
    model: String,
    timeout_secs: u64,
) -> Result<(), String> {
    let context = AppContext::new(StorageConfig::default());
    context
        .set_llm_config(&base_url, &model, timeout_secs)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_llm_api_key(key: String) -> Result<(), String> {
    let context = AppContext::new(StorageConfig::default());
    context
        .set_llm_api_key(&key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_llm_analysis(
    scope: Option<String>,
    date: String,
    analysis: serde_json::Value,
) -> Result<app_service::ReportSummary, String> {
    let parsed_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let parsed_scope = match scope.as_deref().unwrap_or("global") {
        "cn" => app_service::ReportScope::Cn,
        "hk" => app_service::ReportScope::Hk,
        _ => app_service::ReportScope::Global,
    };
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.export_llm_analysis(parsed_scope, parsed_date, &analysis)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn run_preclose_analysis(
    scope: String,
) -> Result<Vec<app_service::ExecutionDecision>, String> {
    let report_scope = match scope.as_str() {
        "cn" => app_service::ReportScope::Cn,
        "hk" => app_service::ReportScope::Hk,
        _ => app_service::ReportScope::Global,
    };

    let decisions = tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.analyze_preclose(report_scope)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| e.to_string())?;

    Ok(decisions)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RefreshCoordinator::default())
        .invoke_handler(tauri::generate_handler![
            app_status,
            dashboard_bundle,
            dashboard_snapshot,
            dashboard_available_dates,
            export_report,
            get_signal_detail,
            data_health_summary,
            export_data_health_report,
            recent_reports,
            open_report_artifact,
            usage_guides,
            dashboard_refresh_status,
            start_dashboard_refresh,
            cancel_dashboard_refresh,
            retry_dashboard_refresh,
            get_user_preferences,
            set_user_preference,
            get_llm_status,
            set_llm_config,
            set_llm_api_key,
            export_llm_analysis,
            analyze_with_llm,
            run_preclose_analysis,
            check_startup_freshness,
            auto_ingest_on_startup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

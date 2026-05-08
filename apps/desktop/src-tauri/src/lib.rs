use app_service::AppContext;
use chrono::{Local, NaiveDate};
use market_store::StorageConfig;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
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
}

#[derive(Debug, Clone, Serialize)]
struct DashboardBundlePayload {
    status: app_service::AppStatus,
    available_dates: Vec<String>,
    snapshot: Option<report_engine::DashboardSnapshot>,
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
        }
    }
}

#[derive(Clone, Default)]
struct RefreshCoordinator {
    status: Arc<Mutex<DashboardRefreshStatus>>,
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

    Ok(artifact)
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
        match self {
            Self::Ingest => 20,
            Self::Indicators => 40,
            Self::Macro => 60,
            Self::Rotation => 75,
            Self::Strategy => 88,
            Self::Signals => 92,
            Self::Backtests => 96,
        }
    }

    fn order(self) -> u8 {
        match self {
            Self::Ingest => 0,
            Self::Indicators => 1,
            Self::Macro => 2,
            Self::Rotation => 3,
            Self::Strategy => 4,
            Self::Signals => 5,
            Self::Backtests => 6,
        }
    }

    fn should_run(self, start_stage: Option<Self>) -> bool {
        start_stage
            .map(|start| self.order() >= start.order())
            .unwrap_or(true)
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

fn spawn_dashboard_refresh(
    coordinator: RefreshCoordinator,
    start_stage: Option<RefreshStartStage>,
) -> Result<DashboardRefreshStatus, String> {
    {
        let current = coordinator
            .status
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        if current.running {
            return Ok(current);
        }
    }

    let started_at = Local::now().to_rfc3339();
    let start_stage_value = start_stage_value(start_stage);
    let prep_label = start_stage
        .map(|stage| format!("Preparing rerun from {}", stage.display_label()))
        .unwrap_or_else(|| "Preparing refresh window".to_string());
    set_refresh_status(&coordinator, |status| {
        *status = DashboardRefreshStatus {
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
        };
    });

    let worker = coordinator.clone();
    std::thread::spawn(move || {
        let context = AppContext::new(StorageConfig::default());
        let today = Local::now().date_naive();

        let result = (|| -> Result<(), anyhow::Error> {
            let plan = context.build_refresh_plan(today)?;
            let refresh_from = NaiveDate::parse_from_str(&plan.refresh_from, "%Y-%m-%d")?;
            let refresh_to = NaiveDate::parse_from_str(&plan.refresh_to, "%Y-%m-%d")?;
            let macro_from = NaiveDate::parse_from_str(&plan.macro_from, "%Y-%m-%d")?;
            let macro_to = NaiveDate::parse_from_str(&plan.macro_to, "%Y-%m-%d")?;

            set_refresh_status(&worker, |status| {
                status.progress_pct = 5;
                status.stage = "Prepared refresh window".to_string();
                status.refresh_from = Some(plan.refresh_from.clone());
                status.refresh_to = Some(plan.refresh_to.clone());
            });

            let run_stage = |worker: &RefreshCoordinator,
                             stage: RefreshStartStage,
                             action: &mut dyn FnMut() -> Result<(), anyhow::Error>| -> Result<(), anyhow::Error> {
                if !stage.should_run(start_stage) {
                    return Ok(());
                }
                set_refresh_status(worker, |status| {
                    status.current_stage = Some(stage.as_str().to_string());
                    status.stage = format!("Running {}", stage.display_label());
                });
                action()?;
                set_refresh_status(worker, |status| {
                    status.progress_pct = stage.progress_after();
                    status.stage = format!("{} refreshed", stage.display_label());
                });
                Ok(())
            };

            run_stage(&worker, RefreshStartStage::Ingest, &mut || {
                context.ingest_daily(refresh_from, refresh_to).map(|_| ())
            })?;
            run_stage(&worker, RefreshStartStage::Indicators, &mut || {
                context.compute_indicators().map(|_| ())
            })?;
            run_stage(&worker, RefreshStartStage::Macro, &mut || {
                context.compute_macro_regime(macro_from, macro_to).map(|_| ())
            })?;
            run_stage(&worker, RefreshStartStage::Rotation, &mut || {
                context.compute_rotation().map(|_| ())
            })?;
            run_stage(&worker, RefreshStartStage::Strategy, &mut || {
                context.compute_strategy_preferences().map(|_| ())
            })?;
            run_stage(&worker, RefreshStartStage::Signals, &mut || {
                context.compute_signals().map(|_| ())
            })?;
            run_stage(&worker, RefreshStartStage::Backtests, &mut || {
                context.refresh_backtests_for_standard_scopes().map(|_| ())
            })?;

            let alerts = context.refresh_consistency_alerts()?;
            if !alerts.is_empty() {
                set_refresh_status(&worker, |status| {
                    status.current_stage = None;
                    status.retry_from_stage = None;
                    status.stage = "Refresh consistency validation failed".to_string();
                });
                anyhow::bail!(alerts.join(" | "));
            }
            set_refresh_status(&worker, |status| {
                status.progress_pct = 100;
                status.current_stage = None;
                status.stage = "Refresh consistency verified".to_string();
                status.retry_from_stage = None;
            });

            Ok(())
        })();

        match result {
            Ok(()) => {
                let finished_at = Local::now().to_rfc3339();
                set_refresh_status(&worker, |status| {
                    status.running = false;
                    status.status = "success".to_string();
                    status.finished_at = Some(finished_at);
                    status.error = None;
                    status.current_stage = None;
                    status.retry_from_stage = None;
                });
            }
            Err(error) => {
                let finished_at = Local::now().to_rfc3339();
                set_refresh_status(&worker, |status| {
                    let retry_from_stage = status.retry_from_stage.clone();
                    let stage_label = retry_from_stage
                        .as_deref()
                        .map(stage_label_from_key)
                        .unwrap_or("Refresh consistency validation");
                    status.running = false;
                    status.status = "error".to_string();
                    status.stage = format!("{} failed", stage_label);
                    status.finished_at = Some(finished_at);
                    status.error = Some(error.to_string());
                    status.retry_from_stage = retry_from_stage;
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

    let refresh_status = refresh
        .status
        .lock()
        .map(|status| status.clone())
        .map_err(|error| error.to_string())?;

    Ok(DashboardBundlePayload {
        status: bundle.status,
        available_dates: bundle.available_dates,
        snapshot: bundle.snapshot,
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
        context.get_signal_detail(parsed_scope, &symbol, parsed_date.unwrap_or_else(|| chrono::Utc::now().date_naive()))
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
async fn recent_reports(limit: Option<usize>) -> Result<Vec<app_service::RecentReportItem>, String> {
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
    refresh
        .status
        .lock()
        .map(|status| status.clone())
        .map_err(|error| error.to_string())
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
fn retry_dashboard_refresh(
    refresh: tauri::State<RefreshCoordinator>,
) -> Result<DashboardRefreshStatus, String> {
    let retry_stage = {
        let status = refresh
            .status
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        parse_refresh_start_stage(status.retry_from_stage.as_deref())?
            .ok_or_else(|| "no failed stage is available to retry".to_string())?
    };
    spawn_dashboard_refresh(refresh.inner().clone(), Some(retry_stage))
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
            retry_dashboard_refresh
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

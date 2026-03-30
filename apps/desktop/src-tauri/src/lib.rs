use app_service::AppContext;
use chrono::{Local, NaiveDate};
use market_store::StorageConfig;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
struct DashboardRefreshStatus {
    running: bool,
    status: String,
    progress_pct: u8,
    stage: String,
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

#[tauri::command]
fn app_status() -> Result<app_service::AppStatus, String> {
    let context = AppContext::new(StorageConfig::default());
    context.status().map_err(|error| error.to_string())
}

#[tauri::command]
async fn dashboard_bundle(
    refresh: tauri::State<'_, RefreshCoordinator>,
    report_date: Option<String>,
    recent_report_limit: Option<usize>,
) -> Result<DashboardBundlePayload, String> {
    let parsed_date = report_date
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .transpose()
        .map_err(|error| error.to_string())?;
    let limit = recent_report_limit.unwrap_or(10);
    let bundle = tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.dashboard_bundle(parsed_date, limit)
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
) -> Result<Option<report_engine::DashboardSnapshot>, String> {
    let parsed_date = report_date
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .transpose()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.dashboard_snapshot(parsed_date)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn dashboard_available_dates() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.dashboard_available_dates()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_report(report_date: Option<String>) -> Result<app_service::ReportSummary, String> {
    let parsed_date = report_date
        .as_deref()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d"))
        .transpose()
        .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let context = AppContext::new(StorageConfig::default());
        context.export_report(parsed_date)
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
) -> Result<DashboardRefreshStatus, String> {
    let coordinator = refresh.inner().clone();
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
    set_refresh_status(&coordinator, |status| {
        *status = DashboardRefreshStatus {
            running: true,
            status: "running".to_string(),
            progress_pct: 0,
            stage: "Preparing refresh window".to_string(),
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

            context.ingest_daily(refresh_from, refresh_to)?;
            set_refresh_status(&worker, |status| {
                status.progress_pct = 20;
                status.stage = "Daily bars updated".to_string();
            });

            context.compute_indicators()?;
            set_refresh_status(&worker, |status| {
                status.progress_pct = 40;
                status.stage = "Indicators recomputed".to_string();
            });

            context.compute_macro_regime(macro_from, macro_to)?;
            set_refresh_status(&worker, |status| {
                status.progress_pct = 60;
                status.stage = "Macro regime recomputed".to_string();
            });

            context.compute_rotation()?;
            set_refresh_status(&worker, |status| {
                status.progress_pct = 75;
                status.stage = "Rotation updated".to_string();
            });

            context.compute_strategy_preferences()?;
            set_refresh_status(&worker, |status| {
                status.progress_pct = 88;
                status.stage = "Strategy preferences updated".to_string();
            });

            context.compute_signals()?;
            set_refresh_status(&worker, |status| {
                status.progress_pct = 100;
                status.stage = "Signals refreshed".to_string();
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
                });
            }
            Err(error) => {
                let finished_at = Local::now().to_rfc3339();
                set_refresh_status(&worker, |status| {
                    status.running = false;
                    status.status = "error".to_string();
                    status.finished_at = Some(finished_at);
                    status.error = Some(error.to_string());
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
            data_health_summary,
            export_data_health_report,
            recent_reports,
            usage_guides,
            dashboard_refresh_status,
            start_dashboard_refresh
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

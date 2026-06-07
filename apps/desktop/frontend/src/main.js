import { invoke } from '@tauri-apps/api/core';
import {
  getErrorMessage,
  normalizeAvailableDates,
  normalizeRecentReports,
  normalizeRefreshStatus,
  normalizeScope,
  resolveSelectedReportDate,
} from './lib/dashboard-utils.js';
import {
  dashboardStore,
  updateSnapshot as syncSnapshotToStore,
  updateInsight as syncInsightToStore,
  updateStatus as syncStatusToStore,
  updateScope as syncScopeToStore,
  updateReportDate as syncReportDateToStore,
  updateAvailableDates as syncAvailableDatesToStore,
  updateLoading as syncLoadingToStore,
  updateError as syncErrorToStore,
  updateExporting as syncExportingToStore,
  updateExportResult as syncExportResultToStore,
  updateRefreshStatus as syncRefreshStatusToStore,
  updateRefreshing as syncRefreshingToStore,
  updateRecentReports as syncRecentReportsToStore,
  initEventBridge,
} from './store.js';
import { setLocale, setPersistCallback, i18n } from './i18n.js';
import { llmApi } from './api/tauri.js';

// Helper to translate
function t(key, params) {
  return i18n.global.t(key, params);
}
import './styles.css';

const RECENT_REPORT_LIMIT = 8;

const COMMANDS = {
  status: 'app_status',
  dashboardBundle: 'dashboard_bundle',
  availableDates: 'dashboard_available_dates',
  snapshot: 'dashboard_snapshot',
  exportReport: 'export_report',
  dataHealthSummary: 'data_health_summary',
  exportDataHealthReport: 'export_data_health_report',
  recentReports: 'recent_reports',
  usageGuides: 'usage_guides',
  startRefresh: 'start_dashboard_refresh',
  cancelRefresh: 'cancel_dashboard_refresh',
  retryRefresh: 'retry_dashboard_refresh',
  refreshStatus: 'dashboard_refresh_status',
  openReportArtifact: 'open_report_artifact',
  getUserPreferences: 'get_user_preferences',
  setUserPreference: 'set_user_preference',
};

let refreshPollTimer = null;

function savePreference(key, value) {
  invoke(COMMANDS.setUserPreference, { key, value }).catch(() => {});
}

// Set up locale persistence callback
setPersistCallback((locale) => savePreference('locale', locale));

async function loadAndApplyPreferences() {
  try {
    const prefs = await invoke(COMMANDS.getUserPreferences);
    if (prefs.default_scope && ['global', 'cn', 'hk'].includes(prefs.default_scope)) {
      dashboardStore.selectedScope = prefs.default_scope;
    }
    if (prefs.last_analysis_date) {
      dashboardStore.selectedReportDate = prefs.last_analysis_date;
    }
    // Load and apply locale preference
    if (prefs.locale && ['zh', 'en'].includes(prefs.locale)) {
      setLocale(prefs.locale);
    }
  } catch (_) {
    // Silently ignore preference load errors
  }
}

function getActiveReportDate() {
  return dashboardStore.selectedReportDate || dashboardStore.snapshot?.report_date || '';
}

function pushRecentReport(reportType, reportDate, artifactPath) {
  const updated = normalizeRecentReports([
    {
      report_type: reportType,
      report_date: reportDate,
      artifact_path: artifactPath,
    },
    ...dashboardStore.recentReports,
  ], RECENT_REPORT_LIMIT);
  syncRecentReportsToStore(updated);
}

// ── Refresh orchestration ──────────────────────────────────────────────

function stopRefreshPolling() {
  if (refreshPollTimer) {
    clearTimeout(refreshPollTimer);
    refreshPollTimer = null;
  }
}

function scheduleRefreshPoll(delay = 1000) {
  stopRefreshPolling();
  if (!dashboardStore.refreshing && !dashboardStore.refreshStatus.running) return;
  refreshPollTimer = setTimeout(() => {
    pollRefreshStatus();
  }, delay);
}

async function pollRefreshStatus() {
  try {
    dashboardStore.refreshStatus = normalizeRefreshStatus(await invoke(COMMANDS.refreshStatus));
    dashboardStore.refreshing = dashboardStore.refreshStatus.running;
    syncRefreshStatusToStore(dashboardStore.refreshStatus);
    syncRefreshingToStore(dashboardStore.refreshing);

    if (dashboardStore.refreshStatus.running) {
      scheduleRefreshPoll(1000);
      return;
    }

    stopRefreshPolling();
    if (dashboardStore.refreshStatus.status === 'success') {
      await loadDashboard();
    }
  } catch (error) {
    stopRefreshPolling();
    dashboardStore.refreshing = false;
    dashboardStore.refreshStatus = {
      ...dashboardStore.refreshStatus,
      running: false,
      status: 'error',
      error: getErrorMessage(error),
    };
    syncRefreshStatusToStore(dashboardStore.refreshStatus);
    syncRefreshingToStore(false);
  }
}

async function startRefreshJob(startStage = 'full') {
  if (dashboardStore.refreshing || dashboardStore.refreshStatus.running) return;

  dashboardStore.error = '';
  dashboardStore.refreshing = true;
  syncRefreshingToStore(true);

  try {
    dashboardStore.refreshStatus = normalizeRefreshStatus(await invoke(COMMANDS.startRefresh, {
      startStage: startStage === 'full' ? null : startStage,
    }));
    dashboardStore.refreshing = dashboardStore.refreshStatus.running;
    syncRefreshStatusToStore(dashboardStore.refreshStatus);
    syncRefreshingToStore(dashboardStore.refreshing);
    scheduleRefreshPoll(500);
  } catch (error) {
    dashboardStore.refreshing = false;
    dashboardStore.refreshStatus = {
      ...dashboardStore.refreshStatus,
      running: false,
      status: 'error',
      error: getErrorMessage(error),
    };
    syncRefreshStatusToStore(dashboardStore.refreshStatus);
    syncRefreshingToStore(false);
  }
}

async function retryFailedRefresh() {
  if (dashboardStore.refreshing || dashboardStore.refreshStatus.running) return;

  dashboardStore.error = '';
  dashboardStore.refreshing = true;
  syncRefreshingToStore(true);

  try {
    dashboardStore.refreshStatus = normalizeRefreshStatus(await invoke(COMMANDS.retryRefresh));
    dashboardStore.refreshing = dashboardStore.refreshStatus.running;
    syncRefreshStatusToStore(dashboardStore.refreshStatus);
    syncRefreshingToStore(dashboardStore.refreshing);
    scheduleRefreshPoll(500);
  } catch (error) {
    dashboardStore.refreshing = false;
    dashboardStore.refreshStatus = {
      ...dashboardStore.refreshStatus,
      running: false,
      status: 'error',
      error: getErrorMessage(error),
    };
    syncRefreshStatusToStore(dashboardStore.refreshStatus);
    syncRefreshingToStore(false);
  }
}

async function cancelRefreshJob() {
  if (!dashboardStore.refreshStatus.running || dashboardStore.refreshStatus.cancelling) return;

  dashboardStore.refreshStatus = {
    ...dashboardStore.refreshStatus,
    cancelling: true,
    stage: t('refresh.cancellingAfterStage'),
  };
  syncRefreshStatusToStore(dashboardStore.refreshStatus);

  try {
    await invoke(COMMANDS.cancelRefresh);
    dashboardStore.refreshStatus = normalizeRefreshStatus(await invoke(COMMANDS.refreshStatus));
    dashboardStore.refreshing = dashboardStore.refreshStatus.running;
    syncRefreshStatusToStore(dashboardStore.refreshStatus);
    syncRefreshingToStore(dashboardStore.refreshing);
    scheduleRefreshPoll(500);
  } catch (error) {
    dashboardStore.refreshStatus = {
      ...dashboardStore.refreshStatus,
      cancelling: false,
      error: getErrorMessage(error),
    };
    syncRefreshStatusToStore(dashboardStore.refreshStatus);
  }
}

// ── Data loading ────────────────────────────────────────────────────────

async function loadDashboard() {
  dashboardStore.loading = true;
  dashboardStore.error = '';
  syncLoadingToStore(true);
  syncErrorToStore('');

  try {
    const previousSelectedReportDate = dashboardStore.selectedReportDate;
    const activeScope = normalizeScope(dashboardStore.selectedScope);
    const bundleResult = await invoke(COMMANDS.dashboardBundle, {
      reportDate: previousSelectedReportDate || null,
      scope: activeScope,
      recentReportLimit: RECENT_REPORT_LIMIT,
    });

    if (bundleResult) {
      dashboardStore.status = bundleResult.status || null;
      dashboardStore.availableDates = normalizeAvailableDates(bundleResult.available_dates);
      dashboardStore.recentReports = normalizeRecentReports(bundleResult.recent_reports, RECENT_REPORT_LIMIT);
      dashboardStore.snapshot = bundleResult.snapshot || null;
      dashboardStore.insight = bundleResult.insight || null;
      dashboardStore.selectedScope = normalizeScope(dashboardStore.snapshot?.scope || activeScope);
      dashboardStore.selectedReportDate = dashboardStore.snapshot?.report_date
        || resolveSelectedReportDate(dashboardStore.availableDates, previousSelectedReportDate);
      dashboardStore.refreshStatus = normalizeRefreshStatus(bundleResult.refresh_status);
      dashboardStore.refreshing = dashboardStore.refreshStatus.running;
      if (dashboardStore.refreshStatus.running) {
        scheduleRefreshPoll(1000);
      }

      // Sync to shared store for Vue components
      syncSnapshotToStore(dashboardStore.snapshot);
      syncInsightToStore(dashboardStore.insight);
      syncStatusToStore(dashboardStore.status);
      syncScopeToStore(dashboardStore.selectedScope);
      syncReportDateToStore(dashboardStore.selectedReportDate);
      syncAvailableDatesToStore(dashboardStore.availableDates);
      syncRefreshStatusToStore(dashboardStore.refreshStatus);
      syncRefreshingToStore(dashboardStore.refreshing);
      syncRecentReportsToStore(dashboardStore.recentReports);
    }

    if (previousSelectedReportDate !== dashboardStore.selectedReportDate) {
      dashboardStore.exportResult = null;
    }

    if (bundleResult || dashboardStore.snapshot) {
      dashboardStore.lastUpdatedAt = new Date().toISOString();
    }
  } catch (error) {
    dashboardStore.snapshot = null;
    dashboardStore.error = getErrorMessage(error);
    syncErrorToStore(dashboardStore.error);
  } finally {
    dashboardStore.loading = false;
    syncLoadingToStore(false);
  }
}

async function loadSelectedSnapshot() {
  dashboardStore.loading = true;
  dashboardStore.error = '';
  syncLoadingToStore(true);
  syncErrorToStore('');

  try {
    const activeReportDate = getActiveReportDate();
    const activeScope = normalizeScope(dashboardStore.selectedScope);
    dashboardStore.snapshot = await invoke(
      COMMANDS.snapshot,
      activeReportDate ? { reportDate: activeReportDate, scope: activeScope } : { scope: activeScope },
    );

    if (dashboardStore.snapshot?.report_date) {
      dashboardStore.selectedScope = normalizeScope(dashboardStore.snapshot.scope || activeScope);
      dashboardStore.selectedReportDate = dashboardStore.snapshot.report_date;
    }

    dashboardStore.lastUpdatedAt = new Date().toISOString();

    // Sync to shared store
    syncSnapshotToStore(dashboardStore.snapshot);
    syncScopeToStore(dashboardStore.selectedScope);
    syncReportDateToStore(dashboardStore.selectedReportDate);
  } catch (error) {
    dashboardStore.snapshot = null;
    dashboardStore.error = `${t('common.dashboardSnapshot')}: ${getErrorMessage(error)}`;

    syncSnapshotToStore(null);
    syncErrorToStore(dashboardStore.error);
  } finally {
    dashboardStore.loading = false;
    syncLoadingToStore(false);
  }
}

// ── Export ──────────────────────────────────────────────────────────────

async function exportReport() {
  const activeReportDate = getActiveReportDate();
  if (!dashboardStore.snapshot || !activeReportDate || dashboardStore.exporting) return;
  const activeScope = normalizeScope(dashboardStore.selectedScope);

  dashboardStore.exporting = true;
  dashboardStore.exportResult = null;
  syncExportingToStore(true);
  syncExportResultToStore(null);

  try {
    const result = await invoke(COMMANDS.exportReport, { reportDate: activeReportDate, scope: activeScope });
    const exportedReportDate = result.report_date || activeReportDate;
    dashboardStore.exportResult = {
      kind: 'success',
      title: t('export.reportExported'),
      message: t('export.savedMessage', { date: exportedReportDate }),
      output_path: result.output_path,
      failed_items: Array.isArray(result.failed_items) ? result.failed_items : [],
    };
    syncExportResultToStore(dashboardStore.exportResult);
    if (result.output_path) {
      const reportType = activeScope === 'cn'
        ? 'DAILY_REPORT_CN'
        : activeScope === 'hk'
          ? 'DAILY_REPORT_HK'
          : 'DAILY_REPORT';
      pushRecentReport(reportType, exportedReportDate, result.output_path);
    }
  } catch (error) {
    dashboardStore.exportResult = {
      kind: 'error',
      title: t('export.exportFailed'),
      message: getErrorMessage(error),
    };
    syncExportResultToStore(dashboardStore.exportResult);
  } finally {
    dashboardStore.exporting = false;
    syncExportingToStore(false);
  }
}

// ── Event bridge ────────────────────────────────────────────────────────

initEventBridge({
  loadDashboard: () => loadDashboard(),
  loadSelectedSnapshot: () => loadSelectedSnapshot(),
  startRefresh: (stage) => startRefreshJob(stage),
  retryRefresh: () => retryFailedRefresh(),
  cancelRefresh: () => cancelRefreshJob(),
  exportReport: () => exportReport(),
  analyzeWithLlm: (scope, skill, agent) => llmApi.analyzeWithSkill(scope, skill, agent),
});

// ── Bootstrap ───────────────────────────────────────────────────────────

loadAndApplyPreferences().then(() => loadDashboard());

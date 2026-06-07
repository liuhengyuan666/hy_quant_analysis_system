import { tauriInvoke, COMMANDS } from './tauri.js';

/**
 * Dashboard data loading API.
 * Encapsulates all dashboard-related Tauri commands.
 */
export const dashboardApi = {
  /**
   * Load full dashboard bundle (status + dates + snapshot + recent reports).
   * Used for initial load and scope changes.
   */
  async loadBundle(scope, reportDate) {
    return tauriInvoke(COMMANDS.dashboardBundle, {
      scope,
      reportDate: reportDate || null,
    });
  },

  /**
   * Load snapshot for a specific date.
   * Used for historical date changes.
   */
  async loadSnapshot(scope, reportDate) {
    return tauriInvoke(COMMANDS.snapshot, {
      scope,
      reportDate: reportDate || null,
    });
  },

  /**
   * Load available analysis dates.
   */
  async loadAvailableDates(scope) {
    return tauriInvoke(COMMANDS.availableDates, { scope });
  },

  /**
   * Load app status.
   */
  async loadStatus() {
    return tauriInvoke(COMMANDS.status);
  },

  /**
   * Export report for a given date and scope.
   */
  async exportReport(reportDate, scope) {
    return tauriInvoke(COMMANDS.exportReport, { reportDate, scope });
  },
};

/**
 * Data health API.
 */
export const dataHealthApi = {
  /**
   * Load data health summary.
   */
  async loadSummary() {
    return tauriInvoke(COMMANDS.dataHealthSummary);
  },

  /**
   * Export data health report.
   */
  async exportReport() {
    return tauriInvoke(COMMANDS.exportDataHealthReport);
  },
};

/**
 * Recent reports API.
 */
export const recentReportsApi = {
  /**
   * Load recent reports list.
   */
  async load() {
    return tauriInvoke(COMMANDS.recentReports);
  },

  /**
   * Open a report artifact in the system viewer.
   */
  async openArtifact(path) {
    return tauriInvoke(COMMANDS.openReportArtifact, { path });
  },
};

/**
 * Usage guides API.
 */
export const usageGuidesApi = {
  /**
   * Load usage guides.
   */
  async load() {
    return tauriInvoke(COMMANDS.usageGuides);
  },
};

import { invoke } from '@tauri-apps/api/core';

/**
 * Tauri command names.
 */
export const COMMANDS = {
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
  getLlmStatus: 'get_llm_status',
  setLlmConfig: 'set_llm_config',
  setLlmApiKey: 'set_llm_api_key',
  analyzeWithLlm: 'analyze_with_llm',
  runPrecloseAnalysis: 'run_preclose_analysis',
  strategyScoreboard: 'strategy_scoreboard',
  strategyAttribution: 'strategy_attribution',
};

export async function tauriInvoke(command, args) {
  try {
    return await invoke(command, args);
  } catch (error) {
    console.error(`[Tauri API] ${command} failed:`, error);
    throw error;
  }
}

export const preferencesApi = {
  async get() { return tauriInvoke(COMMANDS.getUserPreferences); },
  async set(key, value) { return tauriInvoke(COMMANDS.setUserPreference, { key, value }).catch(() => {}); },
};

export const refreshApi = {
  async getStatus() { return tauriInvoke(COMMANDS.refreshStatus); },
  async start(options) { return tauriInvoke(COMMANDS.startRefresh, options); },
  async retry() { return tauriInvoke(COMMANDS.retryRefresh); },
  async cancel() { return tauriInvoke(COMMANDS.cancelRefresh); },
};

export const llmApi = {
  async getStatus() { return tauriInvoke(COMMANDS.getLlmStatus); },

  async analyzeWithLlm(scope, action, adversarial) {
    const args = { scope, action };
    if (adversarial !== undefined) {
      args.adversarial = adversarial;
    }
    return tauriInvoke(COMMANDS.analyzeWithLlm, args);
  },

  async setLlmConfig(baseUrl, model, timeoutSecs) {
    return tauriInvoke(COMMANDS.setLlmConfig, { baseUrl, model, timeoutSecs });
  },

  async setLlmApiKey(key) {
    return tauriInvoke(COMMANDS.setLlmApiKey, { key });
  },
};

export const strategyApi = {
  async strategyScoreboard(scope, date) {
    return tauriInvoke(COMMANDS.strategyScoreboard, { scope, date });
  },

  async strategyAttribution(symbol, scope, date) {
    return tauriInvoke(COMMANDS.strategyAttribution, { symbol, scope, date });
  },
};

import { invoke } from '@tauri-apps/api/core';

/**
 * Tauri command names.
 * Centralizes all invoke targets to avoid magic strings.
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
  listAgentProfiles: 'list_agent_profiles',
  readAgentProfile: 'read_agent_profile',
  saveAgentProfile: 'save_agent_profile',
  listSkills: 'list_skills',
  setLlmConfig: 'set_llm_config',
  setLlmApiKey: 'set_llm_api_key',
  exportLlmAnalysis: 'export_llm_analysis',
  analyzeWithSkill: 'analyze_with_skill',
  analyzeWithLlm: 'analyze_with_llm',
  evaluateSkillTriggers: 'evaluate_skill_triggers',
  runPrecloseAnalysis: 'run_preclose_analysis',
};

/**
 * Generic Tauri invoke wrapper.
 * Provides consistent error handling across all API calls.
 */
export async function tauriInvoke(command, args) {
  try {
    return await invoke(command, args);
  } catch (error) {
    console.error(`[Tauri API] ${command} failed:`, error);
    throw error;
  }
}

/**
 * User preferences API.
 */
export const preferencesApi = {
  async get() {
    return tauriInvoke(COMMANDS.getUserPreferences);
  },

  async set(key, value) {
    return tauriInvoke(COMMANDS.setUserPreference, { key, value }).catch(() => {});
  },
};

/**
 * Dashboard refresh API.
 */
export const refreshApi = {
  async getStatus() {
    return tauriInvoke(COMMANDS.refreshStatus);
  },

  async start(options) {
    return tauriInvoke(COMMANDS.startRefresh, options);
  },

  async retry() {
    return tauriInvoke(COMMANDS.retryRefresh);
  },

  async cancel() {
    return tauriInvoke(COMMANDS.cancelRefresh);
  },
};

/**
 * LLM analysis API.
 */
export const llmApi = {
  async getStatus() {
    return tauriInvoke(COMMANDS.getLlmStatus);
  },

  async listAgentProfiles() {
    return tauriInvoke(COMMANDS.listAgentProfiles);
  },

  async listSkills() {
    return tauriInvoke(COMMANDS.listSkills);
  },

  async analyzeWithSkill(scope, skillName, agentName) {
    return tauriInvoke(COMMANDS.analyzeWithSkill, {
      scope,
      skillName,
      agentName,
    });
  },

  async analyzeWithLlm(scope, agentName) {
    return tauriInvoke(COMMANDS.analyzeWithLlm, {
      scope,
      agentName,
    });
  },

  async readAgentProfile(name) {
    return tauriInvoke(COMMANDS.readAgentProfile, { name });
  },

  async saveAgentProfile(name, content) {
    return tauriInvoke(COMMANDS.saveAgentProfile, {
      name,
      content,
    });
  },

  async setLlmConfig(baseUrl, model, timeoutSecs) {
    return tauriInvoke(COMMANDS.setLlmConfig, {
      baseUrl,
      model,
      timeoutSecs,
    });
  },

  async setLlmApiKey(key) {
    return tauriInvoke(COMMANDS.setLlmApiKey, { key });
  },

  async exportLlmAnalysis(scope, date, analysis) {
    return tauriInvoke(COMMANDS.exportLlmAnalysis, {
      scope,
      date,
      analysis,
    });
  },

  async evaluateSkillTriggers(scope) {
    return tauriInvoke(COMMANDS.evaluateSkillTriggers, { scope });
  },
};

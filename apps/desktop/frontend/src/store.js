/**
 * Shared reactive state module.
 *
 * This module provides a single source of truth for dashboard state
 * that both the Plain JS runtime and Vue components can access.
 *
 * Usage:
 *   import { dashboardStore, updateSnapshot, updateScope } from './store.js';
 *
 * Plain JS: call update functions directly
 * Vue: import dashboardStore and use with computed()/watch()
 */

import { reactive } from 'vue';

/**
 * Shared dashboard state.
 * Both main.js and Vue components read from this store.
 */
export const dashboardStore = reactive({
  /** Current dashboard snapshot data */
  snapshot: null,

  /** App status (profile, clickhouse_url, etc.) */
  status: null,

  /** Selected analysis scope (global, cn, hk) */
  selectedScope: 'global',

  /** Selected report date (YYYY-MM-DD) */
  selectedReportDate: '',

  /** Available analysis dates */
  availableDates: [],

  /** Loading state */
  loading: true,

  /** Error message */
  error: '',

  /** Export in progress */
  exporting: false,

  /** Export result (success/error) */
  exportResult: null,

  /** Refresh status object */
  refreshStatus: {
    running: false,
    status: 'idle',
    progress_pct: 0,
    stage: 'Idle',
    current_stage: null,
    start_stage: 'full',
    retry_from_stage: null,
    refresh_from: null,
    refresh_to: null,
    started_at: null,
    finished_at: null,
    error: null,
    cancelling: false,
    job_id: null,
    last_successful_stage: null,
  },

  /** Last successful update timestamp */
  lastUpdatedAt: null,

  /** Whether a refresh job is currently in progress */
  refreshing: false,

  /** Selected refresh start stage */
  selectedRefreshStartStage: 'full',

  /** Recent report artifacts */
  recentReports: [],

  /** LLM analysis result */
  llmAnalysis: null,

  /** LLM analysis loading state */
  llmLoading: false,

  /** LLM analysis error message */
  llmError: '',

  /** LLM configuration status */
  llmConfig: null,

  /** Selected agent profile for LLM analysis */
  selectedAgent: 'macro-strategist',

  /** Available agent profiles */
  availableAgents: [],

  /** Selected skill for LLM analysis */
  selectedSkill: 'market-regime-reasoning',

  /** Available research skills */
  availableSkills: [],

  /** Whether LLM analysis panel is visible */
  showLlmPanel: false,
});

/**
 * Update the dashboard snapshot.
 * Called by main.js when new data is loaded.
 */
export function updateSnapshot(snapshot) {
  dashboardStore.snapshot = snapshot;
  dashboardStore.lastUpdatedAt = new Date().toISOString();
}

/**
 * Update app status.
 */
export function updateStatus(status) {
  dashboardStore.status = status;
}

/**
 * Update the selected scope.
 * Called by main.js when user changes scope.
 */
export function updateScope(scope) {
  dashboardStore.selectedScope = scope;
}

/**
 * Update the selected report date.
 * Called by main.js when user changes date.
 */
export function updateReportDate(date) {
  dashboardStore.selectedReportDate = date;
}

/**
 * Update available dates.
 */
export function updateAvailableDates(dates) {
  dashboardStore.availableDates = dates || [];
}

/**
 * Update loading state.
 */
export function updateLoading(loading) {
  dashboardStore.loading = loading;
}

/**
 * Update error state.
 */
export function updateError(error) {
  dashboardStore.error = error || '';
}

/**
 * Update exporting state.
 */
export function updateExporting(exporting) {
  dashboardStore.exporting = exporting;
}

/**
 * Update export result.
 */
export function updateExportResult(result) {
  dashboardStore.exportResult = result;
}

/**
 * Update refresh status.
 */
export function updateRefreshStatus(status) {
  dashboardStore.refreshStatus = status;
}

/**
 * Update refreshing state.
 */
export function updateRefreshing(refreshing) {
  dashboardStore.refreshing = refreshing;
}

/**
 * Update selected refresh start stage.
 */
export function updateSelectedRefreshStartStage(stage) {
  dashboardStore.selectedRefreshStartStage = stage || 'full';
}

/**
 * Update recent reports list.
 */
export function updateRecentReports(reports) {
  dashboardStore.recentReports = reports || [];
}

/**
 * Update LLM analysis state.
 */
export function updateLlmAnalysis(analysis) {
  dashboardStore.llmAnalysis = analysis;
}

/**
 * Update LLM loading state.
 */
export function updateLlmLoading(loading) {
  dashboardStore.llmLoading = loading;
}

/**
 * Update LLM error state.
 */
export function updateLlmError(error) {
  dashboardStore.llmError = error || '';
}

/**
 * Update LLM config status.
 */
export function updateLlmConfig(config) {
  dashboardStore.llmConfig = config;
}

/**
 * Update selected agent profile.
 */
export function updateSelectedAgent(agent) {
  dashboardStore.selectedAgent = agent || 'macro-strategist';
}

/**
 * Update available agent profiles.
 */
export function updateAvailableAgents(agents) {
  dashboardStore.availableAgents = agents || [];
}

/**
 * Update selected skill.
 */
export function updateSelectedSkill(skill) {
  dashboardStore.selectedSkill = skill || 'market-regime-reasoning';
}

/**
 * Update available skills.
 */
export function updateAvailableSkills(skills) {
  dashboardStore.availableSkills = skills || [];
}

/**
 * Toggle LLM analysis panel visibility.
 */
export function toggleLlmPanel(show) {
  dashboardStore.showLlmPanel = show !== undefined ? show : !dashboardStore.showLlmPanel;
}

/**
 * Reset store to initial state.
 */
export function resetStore() {
  dashboardStore.snapshot = null;
  dashboardStore.status = null;
  dashboardStore.selectedScope = 'global';
  dashboardStore.selectedReportDate = '';
  dashboardStore.availableDates = [];
  dashboardStore.loading = true;
  dashboardStore.error = '';
  dashboardStore.exporting = false;
  dashboardStore.exportResult = null;
  dashboardStore.refreshStatus = {
    running: false,
    status: 'idle',
    progress_pct: 0,
    stage: 'Idle',
    current_stage: null,
    start_stage: 'full',
    retry_from_stage: null,
    refresh_from: null,
    refresh_to: null,
    started_at: null,
    finished_at: null,
    error: null,
    cancelling: false,
    job_id: null,
    last_successful_stage: null,
  };
  dashboardStore.lastUpdatedAt = null;
  dashboardStore.refreshing = false;
  dashboardStore.selectedRefreshStartStage = 'full';
  dashboardStore.recentReports = [];
  dashboardStore.llmAnalysis = null;
  dashboardStore.llmLoading = false;
  dashboardStore.llmError = '';
  dashboardStore.llmConfig = null;
  dashboardStore.selectedAgent = 'macro-strategist';
  dashboardStore.availableAgents = [];
  dashboardStore.selectedSkill = 'market-regime-reasoning';
  dashboardStore.availableSkills = [];
  dashboardStore.showLlmPanel = false;
}

/**
 * Event bridge functions.
 * These are set by main.js and called by Vue components
 * to trigger data loads without direct coupling.
 */
export let loadDashboard = () => console.warn('[Store] loadDashboard not yet initialized');
export let loadSelectedSnapshot = () => console.warn('[Store] loadSelectedSnapshot not yet initialized');
export let startRefresh = (stage) => console.warn('[Store] startRefresh not yet initialized', stage);
export let retryRefresh = () => console.warn('[Store] retryRefresh not yet initialized');
export let cancelRefresh = () => console.warn('[Store] cancelRefresh not yet initialized');
export let exportReport = () => console.warn('[Store] exportReport not yet initialized');

/**
 * Initialize event bridge with actual implementations from main.js.
 * Called once during main.js initialization.
 */
export function initEventBridge(handlers) {
  if (handlers.loadDashboard) loadDashboard = handlers.loadDashboard;
  if (handlers.loadSelectedSnapshot) loadSelectedSnapshot = handlers.loadSelectedSnapshot;
  if (handlers.startRefresh) startRefresh = handlers.startRefresh;
  if (handlers.retryRefresh) retryRefresh = handlers.retryRefresh;
  if (handlers.cancelRefresh) cancelRefresh = handlers.cancelRefresh;
  if (handlers.exportReport) exportReport = handlers.exportReport;
}

import { invoke } from '@tauri-apps/api/core';
import { createDataHealthSlice } from './features/data-health.js';
import { createRecentReportsSlice } from './features/recent-reports.js';
import { createUsageGuidesSlice } from './features/usage-guides.js';
import {
  escapeHtml,
  formatCanonicalAdjustment,
  formatCurrency,
  formatDate,
  formatDateRange,
  formatDateTime,
  formatFallbackState,
  formatInteger,
  formatNumber,
  formatPercent,
  formatReportType,
  formatScopeLabel,
  getDayDifference,
  getErrorMessage,
  getFiniteNumber,
  getFlaggedMacroSources,
  getFlaggedSymbols,
  getRecentReportScope,
  healthTone,
  normalizeAvailableDates,
  normalizeRecentReports,
  normalizeRefreshStatus,
  normalizeScope,
  normalizeUsageGuides,
  prettifyToken,
  regimeTone,
  renderMarkdownContent,
  resolveSelectedReportDate,
  resolveSelectedUsageGuide,
  signalTone,
  trustTone,
} from './lib/dashboard-utils.js';
import { createEnvironmentBreadthRenderers } from './renderers/environment-breadth.js';
import './styles.css';

const app = document.querySelector('#app');

const RECENT_REPORT_LIMIT = 8;
const DATA_HEALTH_CACHE_MS = 5 * 60 * 1000;
const REFRESH_START_STAGE_OPTIONS = [
  { value: 'full', label: 'Full refresh' },
  { value: 'ingest', label: 'From daily bars' },
  { value: 'indicators', label: 'From indicators' },
  { value: 'macro', label: 'From macro' },
  { value: 'rotation', label: 'From rotation' },
  { value: 'strategy', label: 'From strategy' },
  { value: 'signals', label: 'From signals' },
  { value: 'backtests', label: 'From backtests' },
];

const state = {
  loading: true,
  refreshing: false,
  dataHealthLoading: false,
  exporting: false,
  dataHealthExporting: false,
  usageGuidesLoading: false,
  usageGuidesLoaded: false,
  isUsageGuideOpen: false,
  error: '',
  dataHealthError: '',
  usageGuidesError: '',
  exportResult: null,
  dataHealthExportResult: null,
  recentReportActionResult: null,
  status: null,
  pipelineDates: null,
  snapshot: null,
  dataHealth: null,
  dataHealthFetchedAt: null,
  recentReports: [],
  usageGuides: [],
  availableDates: [],
  selectedScope: 'global',
  selectedReportDate: '',
  selectedRefreshStartStage: 'full',
  selectedUsageGuideId: '',
  selectedSignalDetail: null,
  lastUpdatedAt: null,
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
  },
};

let refreshPollTimer = null;
let renderFrame = 0;

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
  retryRefresh: 'retry_dashboard_refresh',
  refreshStatus: 'dashboard_refresh_status',
  openReportArtifact: 'open_report_artifact',
};

const usageGuides = createUsageGuidesSlice({
  state,
  render,
  invoke,
  commands: COMMANDS,
  escapeHtml,
  formatInteger,
  normalizeUsageGuides,
  resolveSelectedUsageGuide,
  renderMarkdownContent,
  getErrorMessage,
});

const dataHealth = createDataHealthSlice({
  state,
  render,
  invoke,
  commands: COMMANDS,
  cacheMs: DATA_HEALTH_CACHE_MS,
  escapeHtml,
  formatCanonicalAdjustment,
  formatDate,
  formatDateRange,
  formatDateTime,
  formatFallbackState,
  formatInteger,
  formatNumber,
  getErrorMessage,
  getFlaggedMacroSources,
  getFlaggedSymbols,
  healthTone,
  prettifyToken,
  renderMetricCard,
  renderNotice,
  pushRecentReport,
});

const environmentBreadthRenderers = createEnvironmentBreadthRenderers({
  renderMetricCard,
});

const recentReports = createRecentReportsSlice({
  state,
  render,
  invoke,
  commands: COMMANDS,
  escapeHtml,
  formatDate,
  formatInteger,
  formatReportType,
  formatScopeLabel,
  getErrorMessage,
  getRecentReportScope,
  getActiveReportDate,
  loadDashboard,
  loadSelectedSnapshot,
  normalizeScope,
  renderNotice,
});

function getActiveReportDate() {
  return state.selectedReportDate || state.snapshot?.report_date || '';
}

function getRegimeFreshness(snapshot) {
  if (!snapshot?.report_date) return null;

  const asOfDate = snapshot.regime_as_of_date || snapshot.report_date;
  const lagDays = getFiniteNumber(snapshot.regime_stale_days) ?? getDayDifference(asOfDate, snapshot.report_date);

  if (lagDays === 0) {
    return {
      stale: false,
      tone: 'positive',
      value: 'Synchronized',
      meta: `As of ${formatDate(asOfDate)}`,
      asOfDate,
      lagDays: 0,
    };
  }

  if (typeof lagDays === 'number' && lagDays > 0) {
    return {
      stale: true,
      tone: lagDays > 2 ? 'negative' : 'neutral',
      value: `${formatInteger(lagDays)}d behind`,
      meta: `Regime as of ${formatDate(asOfDate)}`,
      asOfDate,
      lagDays,
    };
  }

  return {
    stale: asOfDate !== snapshot.report_date,
    tone: 'neutral',
    value: 'Offset',
    meta: `Regime as of ${formatDate(asOfDate)}`,
    asOfDate,
    lagDays,
  };
}

function getLatestAvailableDate(snapshot) {
  return snapshot?.latest_available_date || state.availableDates[0] || '';
}

function pushRecentReport(reportType, reportDate, artifactPath) {
  state.recentReports = normalizeRecentReports([
    {
      report_type: reportType,
      report_date: reportDate,
      artifact_path: artifactPath,
    },
    ...state.recentReports,
  ], RECENT_REPORT_LIMIT);
}

function getViewModeSummary(selectedDate, latestAvailableDate) {
  if (!selectedDate || !latestAvailableDate) {
    return {
      value: 'Awaiting snapshot',
      meta: 'Load a dashboard snapshot to compare the selected analysis date with the newest available run.',
      tone: 'neutral',
    };
  }

  if (selectedDate === latestAvailableDate) {
    return {
      value: 'Latest snapshot',
      meta: 'Selected analysis date matches the newest available analysis in storage.',
      tone: 'positive',
    };
  }

  const lagDays = getDayDifference(selectedDate, latestAvailableDate);

  return {
    value: 'Historical view',
    meta: typeof lagDays === 'number' && lagDays > 0
      ? `Selected analysis is ${formatInteger(lagDays)} day${lagDays === 1 ? '' : 's'} behind the latest available analysis.`
      : 'Selected analysis differs from the newest available analysis date.',
    tone: 'neutral',
  };
}

function formatRefreshStageLabel(value) {
  const normalized = String(value ?? 'full').trim().toLowerCase();
  return REFRESH_START_STAGE_OPTIONS.find((option) => option.value === normalized)?.label || 'Full refresh';
}


function renderMetricCard(label, value, meta, tone = 'neutral') {
  return `
    <article class="metric-card metric-card--${tone}">
      <span class="metric-card__label">${escapeHtml(label)}</span>
      <strong class="metric-card__value">${escapeHtml(value)}</strong>
      <span class="metric-card__meta">${escapeHtml(meta)}</span>
    </article>
  `;
}

function renderPipelineDateDiagnostics(pipelineDates) {
  if (!pipelineDates?.stages?.length) {
    return `
      <section>
        <div class="panel__subheader">
          <p class="panel__section-title">Pipeline freshness</p>
          <span class="panel__meta">No diagnostics yet</span>
        </div>
        <div class="empty-state empty-state--compact">
          <p>No stage freshness diagnostics are available.</p>
        </div>
      </section>
    `;
  }

  return `
    <section>
      <div class="panel__subheader">
        <p class="panel__section-title">Pipeline freshness</p>
        <span class="panel__meta">Freshest market date · ${escapeHtml(formatDate(pipelineDates.freshest_market_date))}</span>
      </div>
      ${Array.isArray(pipelineDates.alerts) && pipelineDates.alerts.length ? `
        <section class="staleness-banner staleness-banner--warning" aria-label="Pipeline freshness alerts">
          <strong>Action required before trusting latest defaults</strong>
          <ul class="note-list">${pipelineDates.alerts.map((alert) => `<li>${escapeHtml(alert)}</li>`).join('')}</ul>
        </section>
      ` : ''}
      <div class="table-wrap">
        <table class="data-table data-table--compact">
          <thead>
            <tr>
              <th>Stage</th>
              <th>Latest date</th>
              <th>Coverage</th>
              <th>Lag</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            ${pipelineDates.stages.map((item) => {
              const lagDays = getFiniteNumber(item?.lag_days);
              const isLatest = Boolean(item?.is_latest);
              const isComplete = item?.is_complete;
              const coverageText = Number.isFinite(Number(item?.latest_entities)) && Number.isFinite(Number(item?.expected_entities))
                ? `${formatInteger(item.latest_entities)}/${formatInteger(item.expected_entities)}`
                : '—';
              const tone = isLatest
                ? isComplete === false
                  ? 'warning'
                  : 'positive'
                : (lagDays !== null && lagDays > 0 ? 'warning' : 'outline');
              const statusLabel = isLatest
                ? isComplete === false
                  ? 'Partial latest'
                  : 'Fresh'
                : (lagDays !== null ? `${formatInteger(lagDays)}d behind` : 'Unknown');
              return `
                <tr>
                  <td class="data-table__symbol">${escapeHtml(prettifyToken(item?.stage || 'unknown'))}</td>
                  <td>${escapeHtml(item?.latest_date ? formatDate(item.latest_date) : 'Unavailable')}</td>
                  <td>${escapeHtml(coverageText)}</td>
                  <td>${escapeHtml(lagDays === null ? '—' : `${formatInteger(lagDays)}d`)}</td>
                  <td><span class="pill pill--${tone}">${escapeHtml(statusLabel)}</span></td>
                </tr>
              `;
            }).join('')}
          </tbody>
        </table>
      </div>
    </section>
  `;
}

function renderTimeContext(snapshot) {
  const selectedDate = snapshot?.report_date || state.selectedReportDate || '';
  const latestAvailableDate = getLatestAvailableDate(snapshot);
  const regimeFreshness = getRegimeFreshness(snapshot);
  const regimeAsOfDate = snapshot?.regime_as_of_date || snapshot?.report_date || '';
  const viewMode = getViewModeSummary(selectedDate, latestAvailableDate);

  return `
    <section class="overview-grid" aria-label="Dashboard time context">
      ${renderMetricCard(
        'Selected analysis date',
        selectedDate ? formatDate(selectedDate) : 'Unavailable',
        selectedDate
          ? 'All dashboard panels below reflect this analysis snapshot.'
          : 'Load a dashboard snapshot to inspect a report date.',
      )}
      ${renderMetricCard(
        'Latest available analysis',
        latestAvailableDate ? formatDate(latestAvailableDate) : 'Unavailable',
        latestAvailableDate
          ? selectedDate === latestAvailableDate
            ? 'You are viewing the newest analysis currently available.'
            : 'Newest stored analysis date available from the selector.'
          : 'No analysis dates are available yet.',
        selectedDate && latestAvailableDate && selectedDate === latestAvailableDate ? 'positive' : 'neutral',
      )}
      ${renderMetricCard(
        'Regime as-of date',
        regimeAsOfDate ? formatDate(regimeAsOfDate) : 'Unavailable',
        regimeAsOfDate
          ? regimeFreshness?.stale
            ? 'Macro posture inputs were last refreshed before the selected analysis date.'
            : 'Macro posture inputs are aligned with the selected analysis date.'
          : 'Macro posture timestamp is unavailable.',
        regimeFreshness?.tone || 'neutral',
      )}
      ${renderMetricCard('View mode', viewMode.value, viewMode.meta, viewMode.tone)}
    </section>
  `;
}

function renderHealthStrip(status, snapshot) {
  const exportTone = state.exportResult?.kind === 'success'
    ? 'positive'
    : state.exportResult?.kind === 'error'
      ? 'negative'
      : 'neutral';

  const items = [
    {
      label: 'Runtime',
      value: status ? 'Connected' : 'Awaiting runtime',
      tone: status ? 'positive' : 'neutral',
    },
    {
      label: 'Snapshot',
      value: snapshot?.report_date ? `Loaded · ${formatDate(snapshot.report_date)}` : 'No report snapshot yet',
      tone: snapshot?.report_date ? 'positive' : 'neutral',
    },
    {
      label: 'Export',
      value: state.exporting
        ? 'Export in progress'
        : state.exportResult?.kind === 'success'
          ? 'Last export succeeded'
          : state.exportResult?.kind === 'error'
            ? 'Last export failed'
            : snapshot
              ? 'Ready to export'
              : 'Waiting for snapshot',
      tone: state.exporting ? 'neutral' : exportTone,
    },
  ];

  return `
    <section class="health-strip" aria-label="Dashboard health summary">
      ${items
        .map(
          (item) => `
            <article class="status-chip status-chip--${item.tone}">
              <span class="status-chip__label">${escapeHtml(item.label)}</span>
              <strong class="status-chip__value">${escapeHtml(item.value)}</strong>
            </article>
          `,
        )
        .join('')}
    </section>
  `;
}

function renderTrustSummaryPanel(snapshot, pipelineDates, dataHealth) {
  const trust = snapshot?.trust_summary;
  if (!trust) return '';

  const trustLevelTone = trustTone(trust.level);
  const freshnessValue = trust.pipeline_has_stale_stage
    ? `${formatInteger(trust.pipeline_stale_stage_count)} stale stage${trust.pipeline_stale_stage_count === 1 ? '' : 's'}`
    : trust.pipeline_has_partial_latest
      ? `${formatInteger(trust.pipeline_partial_latest_stage_count)} partial latest`
      : 'Decision stages fresh';
  const freshnessTone = trust.pipeline_has_stale_stage
    ? 'negative'
    : trust.pipeline_has_partial_latest || !trust.latest_day_complete
      ? 'warning'
      : 'positive';
  const dataHealthValue = trust.data_health_critical_symbols > 0 || trust.data_health_critical_macro_sources > 0
    ? `${formatInteger(trust.data_health_critical_symbols)} symbol / ${formatInteger(trust.data_health_critical_macro_sources)} macro critical`
    : trust.data_health_review_symbols > 0 || trust.data_health_review_macro_sources > 0
      ? `${formatInteger(trust.data_health_review_symbols)} symbol / ${formatInteger(trust.data_health_review_macro_sources)} macro review`
      : 'No critical health warnings';
  const dataHealthToneValue = trust.data_health_critical_symbols > 0 || trust.data_health_critical_macro_sources > 0
    ? 'negative'
    : trust.data_health_review_symbols > 0 || trust.data_health_review_macro_sources > 0
      ? 'warning'
      : 'positive';
  const dataHealthGeneratedAt = dataHealth?.generated_at || trust.data_health_generated_at || '';
  const freshestMarketDate = trust.freshest_market_date || 'N/A';
  const latestAvailableDate = trust.latest_available_date || 'N/A';
  const freshnessStageCount = pipelineDates?.stages?.length ?? 0;
  const notes = Array.isArray(trust.notes) && trust.notes.length
    ? `<ul class="note-list">${trust.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join('')}</ul>`
    : '';
  const historicalEvidenceNote = snapshot?.report_date && trust.latest_available_date && snapshot.report_date !== trust.latest_available_date
    ? `This trust summary combines the selected historical snapshot with current operational freshness/data-health evidence as of ${trust.latest_available_date}.`
    : '';

  return `
    <article class="panel panel--accent">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Trust summary</p>
          <h2>${escapeHtml(trust.headline)}</h2>
          <p class="panel__lede">Primary trust verdict for the currently selected snapshot. Use the evidence sections below before acting on environment, signal, or backtest output.</p>
        </div>
        <div class="panel__actions">
          <span class="pill pill--${trustLevelTone}">${escapeHtml(prettifyToken(trust.level))}</span>
        </div>
      </div>
      <p>${escapeHtml(trust.message)}</p>
      <div class="panel__meta-row">
        <span class="panel__meta">Dashboard scope · ${escapeHtml(snapshot.scope)}</span>
        <span class="panel__meta">Signal analysis scope · ${escapeHtml(trust.signal_analysis_scope || 'N/A')}</span>
        <span class="panel__meta">Signal regime basis · ${escapeHtml(trust.signal_regime_basis_scope || 'N/A')}</span>
        <span class="panel__meta">Backtest matches snapshot · ${escapeHtml(trust.backtest_matches_snapshot === undefined || trust.backtest_matches_snapshot === null ? 'N/A' : trust.backtest_matches_snapshot ? 'yes' : 'no')}</span>
      </div>
      ${historicalEvidenceNote ? `<p class="breadth-panel__note">${escapeHtml(historicalEvidenceNote)}</p>` : ''}
      <div class="mini-metrics">
        ${renderMetricCard('Trust level', prettifyToken(trust.level), trust.headline, trustLevelTone)}
        ${renderMetricCard('Latest-day coverage', `${formatInteger(trust.scoped_symbols_on_freshest_market_date)}/${formatInteger(trust.scoped_symbols_expected)}`, `Freshest market date · ${freshestMarketDate}`, trust.latest_day_complete ? 'positive' : 'warning')}
        ${renderMetricCard('Pipeline evidence', freshnessValue, `Latest available · ${latestAvailableDate}`, freshnessTone)}
        ${renderMetricCard('Data health evidence', dataHealthValue, dataHealthGeneratedAt ? `Generated ${formatDateTime(dataHealthGeneratedAt)}` : 'Detailed health summary not loaded yet', dataHealthToneValue)}
      </div>
      <section>
        <div class="panel__subheader">
          <p class="panel__section-title">Freshness evidence</p>
          <span class="panel__meta">${escapeHtml(formatInteger(freshnessStageCount))} tracked stages</span>
        </div>
        <p class="breadth-panel__note">
          ${escapeHtml(`Pipeline freshness remains the stage-level evidence layer. Current verdict: ${freshnessValue}. Latest-day complete: ${trust.latest_day_complete ? 'yes' : 'no'}.`)}
        </p>
      </section>
      <section>
        <div class="panel__subheader">
          <p class="panel__section-title">Data-health evidence</p>
          <span class="panel__meta">Macro status · ${escapeHtml(prettifyToken(trust.macro_status))}</span>
        </div>
        <p class="breadth-panel__note">
          ${escapeHtml(`Data health remains the symbol/provider evidence layer. Current digest: ${dataHealthValue}.`)}
        </p>
      </section>
      ${notes}
    </article>
  `;
}

function renderStatusPanel(status, pipelineDates) {
  if (!status) {
    return `
      <article class="panel panel--soft">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Runtime</p>
            <h2>App status</h2>
          </div>
        </div>
        <div class="empty-state">
          <p>Status data is unavailable.</p>
        </div>
      </article>
    `;
  }

  return `
    <article class="panel panel--soft">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Runtime</p>
          <h2>App status</h2>
          <p class="panel__lede">Storage targets and active profile wired through the desktop runtime.</p>
        </div>
        <span class="pill pill--outline">${escapeHtml(prettifyToken(status.profile))}</span>
      </div>
      <dl class="detail-grid">
        <div class="detail-item">
          <dt>Profile</dt>
          <dd>${escapeHtml(status.profile)}</dd>
        </div>
        <div class="detail-item">
          <dt>Database</dt>
          <dd>${escapeHtml(status.clickhouse_database)}</dd>
        </div>
        <div class="detail-item detail-item--full">
          <dt>ClickHouse URL</dt>
          <dd><code>${escapeHtml(status.clickhouse_url)}</code></dd>
        </div>
        <div class="detail-item detail-item--full">
          <dt>SQLite path</dt>
          <dd><code>${escapeHtml(status.sqlite_path)}</code></dd>
        </div>
        <div class="detail-item detail-item--full">
          <dt>Universe path</dt>
          <dd><code>${escapeHtml(status.universe_path)}</code></dd>
        </div>
      </dl>
      ${renderPipelineDateDiagnostics(pipelineDates)}
    </article>
  `;
}

function renderRegimePanel(snapshot) {
  if (!snapshot) {
    return `
      <article class="panel panel--accent">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Market regime</p>
            <h2>Waiting for snapshot</h2>
          </div>
        </div>
        <div class="empty-state">
          <p>Run the analysis pipeline to populate the dashboard snapshot.</p>
        </div>
      </article>
    `;
  }

  const scores = [
    ['Trend score', snapshot.trend_score],
    ['Liquidity score', snapshot.liquidity_score],
    ['Risk score', snapshot.risk_score],
  ];
  const freshness = getRegimeFreshness(snapshot);
  const latestAvailableDate = getLatestAvailableDate(snapshot);
  const freshnessPillTone = freshness?.stale ? 'warning' : 'outline';
  const freshnessMessage = freshness?.stale
    ? typeof freshness.lagDays === 'number' && freshness.lagDays > 0
      ? `Macro regime inputs lag the selected report by ${formatInteger(freshness.lagDays)} day${freshness.lagDays === 1 ? '' : 's'}. Posture is shown as of ${formatDate(freshness.asOfDate)} while rotation and signal data reflect ${formatDate(snapshot.report_date)}.`
      : `Macro regime inputs were last updated on ${formatDate(freshness.asOfDate)} while this dashboard view reflects ${formatDate(snapshot.report_date)}.`
    : '';

  return `
    <article class="panel panel--accent">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Market regime</p>
          <h2>${escapeHtml(prettifyToken(snapshot.regime_label))}</h2>
        </div>
        <div class="panel__actions">
          <span class="pill pill--${snapshot.report_date === latestAvailableDate ? 'positive' : regimeTone(snapshot.regime_label)}">Selected analysis · ${escapeHtml(formatDate(snapshot.report_date))}</span>
          ${latestAvailableDate
            ? `<span class="pill pill--outline">Latest available · ${escapeHtml(formatDate(latestAvailableDate))}</span>`
            : ''}
          <span class="pill pill--${freshnessPillTone}">Regime as-of · ${escapeHtml(formatDate(freshness?.asOfDate || snapshot.report_date))}</span>
        </div>
      </div>
      <p class="panel__lede">Latest inferred posture across macro trend, liquidity, and risk inputs.</p>
      ${freshness?.stale
        ? `
          <section class="staleness-banner staleness-banner--warning" aria-label="Regime staleness notice">
            <strong>Macro regime is lagging the selected report</strong>
            <p>${escapeHtml(freshnessMessage)}</p>
          </section>
        `
        : ''}
      <div class="score-stack">
        ${scores
          .map(
            ([label, value]) => `
              <div class="score-row">
                <div class="score-row__meta">
                  <span>${escapeHtml(label)}</span>
                  <strong>${escapeHtml(formatNumber(value, 1))}</strong>
                </div>
                <div class="score-bar">
                  <span class="score-bar__fill" style="width: ${clampScore(value)}%"></span>
                </div>
              </div>
            `,
          )
          .join('')}
      </div>
    </article>
  `;
}

function getSignalBasisInfo(snapshot) {
  const signal = snapshot?.top_signals?.[0] || snapshot?.bullish_signals?.[0] || snapshot?.defensive_signals?.[0];
  if (!signal) return null;

  const analysisScope = String(signal.analysis_scope || 'GLOBAL').toUpperCase();
  const regimeBasisScope = String(signal.regime_basis_scope || 'GLOBAL').toUpperCase();
  const snapshotScope = String(snapshot?.scope || 'GLOBAL').toUpperCase();

  return {
    analysisScope,
    regimeBasisScope,
    snapshotScope,
    mismatched: regimeBasisScope !== snapshotScope,
  };
}

function getBacktestMatchInfo(snapshot, backtest) {
  if (!backtest) return null;
  const snapshotScope = String(snapshot?.scope || 'GLOBAL').toUpperCase();
  const analysisScope = String(backtest.analysis_scope || 'GLOBAL').toUpperCase();
  const signalScope = String(backtest.signal_scope || 'GLOBAL').toUpperCase();
  const regimeBasisScope = String(backtest.regime_basis_scope || 'GLOBAL').toUpperCase();
  const signalEndDate = backtest.signal_end_date || '';
  const matchesCurrentSnapshot = analysisScope === snapshotScope
    && signalScope === snapshotScope
    && signalEndDate === String(snapshot?.report_date || '');

  return {
    analysisScope,
    signalScope,
    regimeBasisScope,
    signalEndDate,
    matchesCurrentSnapshot,
  };
}

function renderSignalReason(reason) {
  if (!reason) return '';

  const alignedStrategies = Array.isArray(reason.aligned_strategies) && reason.aligned_strategies.length
    ? reason.aligned_strategies.map((strategy) => prettifyToken(strategy)).join(', ')
    : 'None';
  const rank = reason.rotation?.rank === null || reason.rotation?.rank === undefined
    ? 'N/A'
    : `#${formatInteger(reason.rotation.rank)}`;

  return `
    <p class="signal-card__text">${escapeHtml(reason.summary || '')}</p>
    <div class="panel__meta-row">
      <span class="panel__meta">Strategy · ${escapeHtml(prettifyToken(reason.best_strategy))} ${escapeHtml(formatNumber(reason.strategy_score, 1))} / contrib ${escapeHtml(formatNumber(reason.strategy_contribution, 1))}</span>
      <span class="panel__meta">Alignment · ${escapeHtml(formatInteger(reason.alignment))} (${escapeHtml(alignedStrategies)}) / contrib ${escapeHtml(formatNumber(reason.alignment_contribution, 1))}</span>
      <span class="panel__meta">Regime · trend ${escapeHtml(formatNumber(reason.regime?.trend_score, 1))} · risk ${escapeHtml(formatNumber(reason.regime?.risk_score, 1))} · contrib ${escapeHtml(formatNumber(reason.regime?.contribution, 1))}</span>
      <span class="panel__meta">Rotation · momentum ${escapeHtml(formatNumber(reason.rotation?.momentum_score, 1))} · rank ${escapeHtml(rank)} · contrib ${escapeHtml(formatNumber(reason.rotation?.contribution, 1))}</span>
    </div>
  `;
}

function renderSignalContributionSection(title, weight, valueRows) {
  return `
    <section class="signal-detail__section">
      <div class="panel__subheader">
        <p class="panel__section-title">${escapeHtml(title)}</p>
        <span class="pill pill--outline">${escapeHtml(weight)}</span>
      </div>
      <dl class="detail-grid signal-detail__grid">
        ${valueRows
          .map(
            ([label, value]) => `
              <div class="detail-item">
                <dt>${escapeHtml(label)}</dt>
                <dd>${value}</dd>
              </div>
            `,
          )
          .join('')}
      </dl>
    </section>
  `;
}

function renderSignalDetailModal(signal) {
  if (!signal) return '';

  const reason = signal.reason || {};
  const label = signal.signal_label || reason.label || 'N/A';
  const finalScore = signal.final_score ?? reason.final_score;
  const alignedStrategies = Array.isArray(reason.aligned_strategies) && reason.aligned_strategies.length
    ? reason.aligned_strategies
      .map((strategy) => `<span class="pill pill--neutral">${escapeHtml(prettifyToken(strategy))}</span>`)
      .join('')
    : '<span class="panel__meta">No aligned strategies</span>';
  const alignmentCount = reason.alignment === null || reason.alignment === undefined
    ? 'N/A'
    : formatInteger(reason.alignment);
  const rotationRank = reason.rotation?.rank === null || reason.rotation?.rank === undefined
    ? 'N/A'
    : `#${formatInteger(reason.rotation.rank)}`;

  return `
    <div class="signal-detail" role="dialog" aria-modal="true" aria-labelledby="signalDetailTitle">
      <button class="signal-detail__backdrop" type="button" aria-label="Close signal detail"></button>
      <article class="signal-detail__panel panel">
        <div class="panel__header signal-detail__header">
          <div>
            <p class="eyebrow">Signal drilldown</p>
            <h2 id="signalDetailTitle">${escapeHtml(signal.symbol || 'Unknown symbol')}</h2>
            <p class="panel__lede">${escapeHtml(reason.summary || 'No structured summary is available for this signal.')}</p>
          </div>
          <div class="panel__actions signal-detail__actions">
            <span class="pill pill--${signalTone(label)}">${escapeHtml(prettifyToken(label))}</span>
            <span class="pill pill--outline">Score ${escapeHtml(formatNumber(finalScore, 2))}</span>
            <button id="closeSignalDetailButton" class="signal-detail__close" type="button" aria-label="Close signal detail">×</button>
          </div>
        </div>
        <div class="signal-detail__sections">
          ${renderSignalContributionSection('Strategy', '45% weight', [
            ['Best strategy', escapeHtml(prettifyToken(reason.best_strategy || 'N/A'))],
            ['Strategy score', escapeHtml(formatNumber(reason.strategy_score, 2))],
            ['Contribution', escapeHtml(formatNumber(reason.strategy_contribution, 2))],
          ])}
          ${renderSignalContributionSection('Alignment', '15% weight', [
            ['Alignment count', escapeHtml(alignmentCount)],
            ['Aligned strategies', `<div class="signal-detail__pill-row">${alignedStrategies}</div>`],
            ['Contribution', escapeHtml(formatNumber(reason.alignment_contribution, 2))],
          ])}
          ${renderSignalContributionSection('Regime', '20% weight', [
            ['Trend score', escapeHtml(formatNumber(reason.regime?.trend_score, 2))],
            ['Risk score', escapeHtml(formatNumber(reason.regime?.risk_score, 2))],
            ['Combined score', escapeHtml(formatNumber(reason.regime?.combined_score, 2))],
            ['Contribution', escapeHtml(formatNumber(reason.regime?.contribution, 2))],
          ])}
          ${renderSignalContributionSection('Rotation', '20% weight', [
            ['Momentum score', escapeHtml(formatNumber(reason.rotation?.momentum_score, 2))],
            ['Rank', escapeHtml(rotationRank)],
            ['Combined score', escapeHtml(formatNumber(reason.rotation?.combined_score, 2))],
            ['Contribution', escapeHtml(formatNumber(reason.rotation?.contribution, 2))],
          ])}
        </div>
      </article>
    </div>
  `;
}

function renderRotationPanel(snapshot) {
  if (!snapshot?.top_rotation?.length) {
    return `
      <article class="panel">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Leadership</p>
            <h2>Top rotation</h2>
          </div>
        </div>
        <div class="empty-state">
          <p>No rotation leaders are available for the latest report date.</p>
        </div>
      </article>
    `;
  }

  return `
    <article class="panel">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Leadership</p>
          <h2>Top rotation</h2>
          <p class="panel__lede">Leaders and laggards ranked by momentum score for the current report date.</p>
        </div>
        <span class="panel__meta">Momentum-ranked leaders and laggards</span>
      </div>
      <div class="rotation-dual-grid">
        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">Leaders</p>
            <span class="panel__meta">Top 5 strength</span>
          </div>
          <div class="table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th>Rank</th>
                  <th>Symbol</th>
                  <th>Momentum</th>
                  <th>RS 20</th>
                  <th>RS 60</th>
                  <th>RS 120</th>
                </tr>
              </thead>
              <tbody>
                ${snapshot.top_rotation
                  .map(
                    (item) => `
                      <tr>
                        <td>#${escapeHtml(item.rank)}</td>
                        <td class="data-table__symbol">${escapeHtml(item.symbol)}</td>
                        <td>${escapeHtml(formatNumber(item.momentum_score, 2))}</td>
                        <td>${escapeHtml(formatNumber(item.rs_20, 2))}</td>
                        <td>${escapeHtml(formatNumber(item.rs_60, 2))}</td>
                        <td>${escapeHtml(formatNumber(item.rs_120, 2))}</td>
                      </tr>
                    `,
                  )
                  .join('')}
              </tbody>
            </table>
          </div>
        </section>
        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">Laggards</p>
            <span class="panel__meta">Bottom 5 momentum</span>
          </div>
          <div class="table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th>Rank</th>
                  <th>Symbol</th>
                  <th>Momentum</th>
                  <th>RS 20</th>
                  <th>RS 60</th>
                  <th>RS 120</th>
                </tr>
              </thead>
              <tbody>
                ${(snapshot.bottom_rotation || [])
                  .map(
                    (item) => `
                      <tr>
                        <td>#${escapeHtml(item.rank)}</td>
                        <td class="data-table__symbol">${escapeHtml(item.symbol)}</td>
                        <td>${escapeHtml(formatNumber(item.momentum_score, 2))}</td>
                        <td>${escapeHtml(formatNumber(item.rs_20, 2))}</td>
                        <td>${escapeHtml(formatNumber(item.rs_60, 2))}</td>
                        <td>${escapeHtml(formatNumber(item.rs_120, 2))}</td>
                      </tr>
                    `,
                  )
                  .join('')}
              </tbody>
            </table>
          </div>
        </section>
      </div>
    </article>
  `;
}

function renderSignalsPanel(snapshot) {
  const topSignals = snapshot?.top_signals || [];
  const bullishSignals = snapshot?.bullish_signals || [];
  const defensiveSignals = snapshot?.defensive_signals || [];

  if (!topSignals.length && !bullishSignals.length && !defensiveSignals.length) {
    return `
      <article class="panel">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Signal stack</p>
            <h2>Signal groups</h2>
          </div>
        </div>
        <div class="empty-state">
          <p>No signal candidates are available for the latest report date.</p>
        </div>
      </article>
    `;
  }

  const signalBasis = getSignalBasisInfo(snapshot);

  return `
    <article class="panel">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Signal stack</p>
          <h2>Buy & defensive groups</h2>
          <p class="panel__lede">Bullish opportunities separated from defensive or sell-side signals for the selected report date.</p>
        </div>
        <div class="panel__actions">
          <span class="panel__meta">Grouped signal view for ${escapeHtml(snapshot.report_date)}</span>
        </div>
      </div>
      ${signalBasis ? `
        <div class="panel__meta-row">
          <span class="panel__meta">Dashboard scope · ${escapeHtml(signalBasis.snapshotScope)}</span>
          <span class="panel__meta">Signal analysis scope · ${escapeHtml(signalBasis.analysisScope)}</span>
          <span class="panel__meta">Signal regime basis · ${escapeHtml(signalBasis.regimeBasisScope)}</span>
        </div>
      ` : ''}
      ${signalBasis?.mismatched ? `
        <section class="staleness-banner staleness-banner--warning" aria-label="Signal provenance notice">
          <strong>Signal scoring basis differs from the selected scope</strong>
          <p>${escapeHtml(`This ${signalBasis.snapshotScope} view is currently showing signals with analysis scope ${signalBasis.analysisScope} and regime basis ${signalBasis.regimeBasisScope}.`)}</p>
        </section>
      ` : ''}
      ${topSignals.length ? `
        <section class="signal-focus-section">
          <div class="panel__subheader">
            <p class="panel__section-title">Top signals</p>
            <span class="panel__meta">Highest conviction across labels</span>
          </div>
          <div class="signal-list">
            ${topSignals
              .map(
                (item, index) => `
                  <button class="signal-card signal-card--top signal-card--interactive" type="button" data-signal-group="top" data-signal-index="${escapeHtml(index)}">
                    <div class="signal-card__header">
                      <div>
                        <strong class="signal-card__symbol">${escapeHtml(item.symbol)}</strong>
                        <p class="signal-card__score">Score ${escapeHtml(formatNumber(item.final_score, 2))}</p>
                      </div>
                      <span class="pill pill--${signalTone(item.signal_label)}">${escapeHtml(prettifyToken(item.signal_label))}</span>
                    </div>
                    ${renderSignalReason(item.reason)}
                  </button>
                `,
              )
              .join('')}
          </div>
        </section>
      ` : ''}
      <div class="signal-groups-grid">
        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">Bullish opportunities</p>
            <span class="panel__meta">StrongBuy / Buy</span>
          </div>
          ${bullishSignals.length
            ? `<div class="signal-list">
                ${bullishSignals
                  .map(
                    (item, index) => `
                      <button class="signal-card signal-card--bullish signal-card--interactive" type="button" data-signal-group="bullish" data-signal-index="${escapeHtml(index)}">
                        <div class="signal-card__header">
                          <div>
                            <strong class="signal-card__symbol">${escapeHtml(item.symbol)}</strong>
                            <p class="signal-card__score">Score ${escapeHtml(formatNumber(item.final_score, 2))}</p>
                          </div>
                          <span class="pill pill--${signalTone(item.signal_label)}">${escapeHtml(prettifyToken(item.signal_label))}</span>
                        </div>
                        ${renderSignalReason(item.reason)}
                      </button>
                    `,
                  )
                  .join('')}
              </div>`
            : `<div class="empty-state empty-state--compact"><p>No bullish signals for this date.</p></div>`}
        </section>
        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">Defensive / sell watch</p>
            <span class="panel__meta">Watch / Hold / Reduce / Sell</span>
          </div>
          ${defensiveSignals.length
            ? `<div class="signal-list">
                ${defensiveSignals
                  .map(
                    (item, index) => `
                      <button class="signal-card signal-card--defensive signal-card--interactive" type="button" data-signal-group="defensive" data-signal-index="${escapeHtml(index)}">
                        <div class="signal-card__header">
                          <div>
                            <strong class="signal-card__symbol">${escapeHtml(item.symbol)}</strong>
                            <p class="signal-card__score">Score ${escapeHtml(formatNumber(item.final_score, 2))}</p>
                          </div>
                          <span class="pill pill--${signalTone(item.signal_label)}">${escapeHtml(prettifyToken(item.signal_label))}</span>
                        </div>
                        ${renderSignalReason(item.reason)}
                      </button>
                    `,
                  )
                  .join('')}
              </div>`
            : `<div class="empty-state empty-state--compact"><p>No defensive or sell-side signals for this date.</p></div>`}
        </section>
      </div>
    </article>
  `;
}

function renderBacktestPanel(snapshot) {
  const backtest = snapshot?.latest_backtest;

  if (!backtest) {
    return `
      <article class="panel panel--soft">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Validation</p>
            <h2>Latest backtest</h2>
          </div>
        </div>
        <div class="empty-state">
          <p>No backtest result is available yet.</p>
        </div>
      </article>
    `;
  }

  const provenance = getBacktestMatchInfo(snapshot, backtest);

  return `
    <article class="panel panel--soft">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Validation</p>
          <h2>Latest backtest</h2>
          <p class="panel__lede">Recent strategy validation snapshot generated from the same pipeline.</p>
        </div>
        <div class="panel__actions">
          <span class="panel__meta">${escapeHtml(backtest.strategy_name)}</span>
          ${provenance ? `<span class="pill pill--${provenance.matchesCurrentSnapshot ? 'positive' : 'warning'}">${escapeHtml(provenance.matchesCurrentSnapshot ? 'Matches current snapshot' : 'Snapshot mismatch')}</span>` : ''}
        </div>
      </div>
      ${provenance ? `
        <div class="panel__meta-row">
          <span class="panel__meta">Analysis scope · ${escapeHtml(provenance.analysisScope)}</span>
          <span class="panel__meta">Signal scope · ${escapeHtml(provenance.signalScope)}</span>
          <span class="panel__meta">Regime basis · ${escapeHtml(provenance.regimeBasisScope)}</span>
          <span class="panel__meta">Signal end · ${escapeHtml(provenance.signalEndDate ? formatDate(provenance.signalEndDate) : 'N/A')}</span>
        </div>
      ` : ''}
      <div class="mini-metrics">
        ${renderMetricCard('CAGR', formatPercent(backtest.cagr), 'Annualized return', 'positive')}
        ${renderMetricCard('Max drawdown', formatPercent(backtest.max_drawdown), 'Peak-to-trough', 'negative')}
        ${renderMetricCard('Sharpe', formatNumber(backtest.sharpe, 2), 'Risk-adjusted', 'neutral')}
        ${renderMetricCard('Final equity', formatCurrency(backtest.final_equity), `${formatInteger(backtest.trades)} trades · ${formatInteger(backtest.trading_days)} days`, 'neutral')}
      </div>
      ${backtest.config_summary ? `<p class="breadth-panel__note">Config · ${escapeHtml(backtest.config_summary)}</p>` : ''}
    </article>
  `;
}

function renderNotice(result, className = '') {
  if (!result) return '';

  const failedItems = result.failed_items?.length
    ? `<p class="notice__detail">Warnings: ${escapeHtml(result.failed_items.join(' · '))}</p>`
    : result.kind === 'success'
      ? '<p class="notice__detail">All report artifacts completed without warnings.</p>'
      : '';

  const classes = ['notice', `notice--${escapeHtml(result.kind)}`, className].filter(Boolean).join(' ');

  return `
    <section class="${classes}">
      <div>
        <strong>${escapeHtml(result.title)}</strong>
        <p>${escapeHtml(result.message)}</p>
        ${result.output_path ? `<p class="notice__detail"><code>${escapeHtml(result.output_path)}</code></p>` : ''}
        ${failedItems}
      </div>
    </section>
  `;
}

function renderTrustSummaryNotice(snapshot, inline = false) {
  const trust = snapshot?.trust_summary;
  if (!trust) return '';

  const notes = Array.isArray(trust.notes) && trust.notes.length
    ? `<ul class="note-list">${trust.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join('')}</ul>`
    : '';
  const historicalEvidenceNote = snapshot?.report_date && trust.latest_available_date && snapshot.report_date !== trust.latest_available_date
    ? `This trust summary combines the selected historical snapshot with current operational freshness/data-health evidence as of ${trust.latest_available_date}.`
    : '';
  const provenanceSummary = [
    snapshot?.scope ? `Dashboard scope: ${snapshot.scope}` : null,
    trust.signal_analysis_scope ? `Signal analysis scope: ${trust.signal_analysis_scope}` : null,
    trust.signal_regime_basis_scope ? `Signal regime basis: ${trust.signal_regime_basis_scope}` : null,
    trust.backtest_matches_snapshot === null || trust.backtest_matches_snapshot === undefined
      ? null
      : `Backtest matches snapshot: ${trust.backtest_matches_snapshot ? 'yes' : 'no'}`,
  ]
    .filter(Boolean)
    .join(' · ');

  return `
    <section class="notice notice--${trustTone(trust.level)} ${inline ? 'notice--inline' : ''}">
      <div>
        <strong>${escapeHtml(trust.headline)}</strong>
        <p>${escapeHtml(trust.message)}</p>
        <p class="notice__detail">
          ${escapeHtml(`Trust level: ${prettifyToken(trust.level)} · Macro status: ${prettifyToken(trust.macro_status)} · Latest-day complete: ${trust.latest_day_complete ? 'yes' : 'no'}`)}
        </p>
        ${provenanceSummary ? `<p class="notice__detail">${escapeHtml(provenanceSummary)}</p>` : ''}
        ${historicalEvidenceNote ? `<p class="notice__detail">${escapeHtml(historicalEvidenceNote)}</p>` : ''}
        ${notes}
      </div>
    </section>
  `;
}

function renderRefreshProgress() {
  const refresh = state.refreshStatus;
  const isVisible = state.refreshing || refresh.running || refresh.status === 'error' || refresh.status === 'success';
  if (!isVisible) return '';

  const tone = refresh.status === 'error'
    ? 'negative'
    : refresh.running
      ? 'neutral'
      : 'positive';

  const progress = Math.max(0, Math.min(100, Number(refresh.progress_pct || 0)));
  const rangeText = refresh.refresh_from && refresh.refresh_to
    ? `${formatDate(refresh.refresh_from)} → ${formatDate(refresh.refresh_to)}`
    : 'Preparing refresh range';
  const timingText = refresh.running
    ? `Started ${formatDateTime(refresh.started_at)}`
    : refresh.finished_at
      ? `Finished ${formatDateTime(refresh.finished_at)}`
      : 'Waiting to start';
  const retryDisabled = state.loading || state.refreshing || state.refreshStatus.running || !refresh.retry_from_stage;

  return `
    <section class="refresh-progress refresh-progress--${tone}" aria-live="polite">
      <div class="refresh-progress__header">
        <div>
          <p class="eyebrow">Background refresh</p>
          <h2>${escapeHtml(refresh.running ? 'Refreshing analysis pipeline' : refresh.status === 'error' ? 'Refresh failed' : 'Refresh completed')}</h2>
          <p class="panel__lede">${escapeHtml(refresh.stage || 'Waiting')}</p>
        </div>
        <div class="panel__actions">
          <span class="pill pill--outline">Run from · ${escapeHtml(formatRefreshStageLabel(refresh.start_stage))}</span>
          ${refresh.retry_from_stage ? `<span class="pill pill--warning">Retry from · ${escapeHtml(formatRefreshStageLabel(refresh.retry_from_stage))}</span>` : ''}
          <span class="pill pill--${tone}">${escapeHtml(`${formatInteger(progress)}%`)}</span>
        </div>
      </div>
      <div class="refresh-progress__bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${escapeHtml(progress)}">
        <span class="refresh-progress__fill" style="width: ${progress}%"></span>
      </div>
      <div class="refresh-progress__meta-row">
        <span>${escapeHtml(rangeText)}</span>
        <span>${escapeHtml(timingText)}</span>
      </div>
      ${refresh.status === 'error' ? `
        <div class="refresh-progress__meta-row">
          <button
            id="retryRefreshButton"
            class="button button--secondary button--compact"
            ${retryDisabled ? 'disabled' : ''}
          >
            Retry failed stage
          </button>
        </div>
      ` : ''}
      ${refresh.error ? `<p class="refresh-progress__error">${escapeHtml(refresh.error)}</p>` : ''}
      ${refresh.status === 'success' ? renderTrustSummaryNotice(state.snapshot, true) : ''}
    </section>
  `;
}

function renderDateSelector() {
  const hasDates = state.availableDates.length > 0;
  const activeReportDate = getActiveReportDate();
  const latestAvailableDate = getLatestAvailableDate(state.snapshot);
  const isLatestSelected = Boolean(latestAvailableDate) && activeReportDate === latestAvailableDate;
  const controlsDisabled = state.loading || state.refreshing || state.refreshStatus.running;
  const selectedValue = hasDates
    ? resolveSelectedReportDate(state.availableDates, activeReportDate)
    : '';
  const optionCountLabel = `${formatInteger(state.availableDates.length)} selectable date${state.availableDates.length === 1 ? '' : 's'}`;

  return `
    <div class="hero__control">
      <div class="control-field">
        <div class="control-field__header">
          <label class="control-field__label" for="scopeSelect">Scope & analysis date</label>
          <button
            id="jumpToLatestButton"
            class="button button--secondary button--compact"
            ${(!hasDates || controlsDisabled || !latestAvailableDate || isLatestSelected) ? 'disabled' : ''}
          >
            ${isLatestSelected ? 'Latest selected' : 'Jump to latest'}
          </button>
        </div>
        <select
          id="scopeSelect"
          class="select-control"
          ${controlsDisabled ? 'disabled' : ''}
        >
          <option value="global" ${state.selectedScope === 'global' ? 'selected' : ''}>GLOBAL · Shared latest date</option>
          <option value="cn" ${state.selectedScope === 'cn' ? 'selected' : ''}>CN · A-share complete latest date</option>
          <option value="hk" ${state.selectedScope === 'hk' ? 'selected' : ''}>HK · Hong Kong complete latest date</option>
        </select>
        <select
          id="reportDateSelect"
          class="select-control"
          ${(!hasDates || controlsDisabled) ? 'disabled' : ''}
        >
          ${hasDates
            ? state.availableDates
              .map(
                (date, index) => `
                  <option value="${escapeHtml(date)}" ${selectedValue === date ? 'selected' : ''}>
                    ${escapeHtml(formatDate(date))}${index === 0 ? ' · Latest' : ''}
                  </option>
                `,
              )
              .join('')
            : '<option value="">No analysis dates available</option>'}
        </select>
        <div class="control-field__toolbar">
          <span class="panel__meta">Scope · ${escapeHtml(formatScopeLabel(state.selectedScope))}</span>
          <span class="panel__meta">${latestAvailableDate ? `Latest available · ${escapeHtml(formatDate(latestAvailableDate))}` : 'Latest available date unavailable'}</span>
        </div>
        <span class="control-field__hint">
          ${hasDates
            ? `Scope controls which market set defines the latest complete report date. Selected analysis date drives every panel below. Latest available analysis loads by default; change the date to inspect historical snapshots and export that same report. Regime as-of date shows when macro posture inputs were last refreshed. ${escapeHtml(optionCountLabel)}.`
            : 'No dashboard analysis dates are available yet.'}
        </span>
      </div>
    </div>
  `;
}


function renderSkeleton() {
  return `
    <section class="skeleton-grid" aria-hidden="true">
      <div class="skeleton-card"></div>
      <div class="skeleton-card"></div>
      <div class="skeleton-card"></div>
      <div class="skeleton-card"></div>
    </section>
  `;
}

function commitRender() {
  const { status, snapshot, dataHealth: dataHealthSummary } = state;
  const shellState = (state.loading || state.refreshing || state.refreshStatus.running) ? 'true' : 'false';

  app.innerHTML = `
    <main class="shell" aria-busy="${shellState}">
      <section class="hero">
        <div class="hero__frame">
          <div class="hero__copy">
            <p class="eyebrow">Quant Desktop · Phase 8</p>
            <h1>Operational dashboard</h1>
            <p class="hero__lede">
              A compact control room for environment health, data quality, current and historical
              market posture, leadership rotation, and highest-conviction signals.
            </p>
          </div>
          <div class="hero__actions">
            ${renderDateSelector()}
            ${usageGuides.renderUsageEntry()}
            <div class="hero__action-row">
              <select
                id="refreshStageSelect"
                class="select-control select-control--compact"
                ${(state.loading || state.refreshing || state.refreshStatus.running) ? 'disabled' : ''}
              >
                ${REFRESH_START_STAGE_OPTIONS
                  .map(
                    (option) => `<option value="${escapeHtml(option.value)}" ${state.selectedRefreshStartStage === option.value ? 'selected' : ''}>${escapeHtml(option.label)}</option>`,
                  )
                  .join('')}
              </select>
              <button id="refreshButton" class="button button--secondary" ${(state.loading || state.refreshing || state.refreshStatus.running) ? 'disabled' : ''}>
                ${(state.refreshing || state.refreshStatus.running) ? 'Refreshing…' : state.selectedRefreshStartStage === 'full' ? 'Refresh data' : `Run from ${formatRefreshStageLabel(state.selectedRefreshStartStage)}`}
              </button>
              <button
                id="exportButton"
                class="button button--primary"
                ${(state.exporting || !snapshot || state.loading || state.refreshing || state.refreshStatus.running) ? 'disabled' : ''}
              >
                ${state.exporting ? 'Exporting…' : 'Export report'}
              </button>
            </div>
            <p class="hero__timestamp">Last sync ${escapeHtml(formatDateTime(state.lastUpdatedAt))}</p>
          </div>
        </div>
        <div class="hero__ambient" aria-hidden="true"></div>
      </section>

      ${renderRefreshProgress()}

      ${state.error ? `<section class="notice notice--error"><div><strong>Data load failed</strong><p>${escapeHtml(state.error)}</p></div></section>` : ''}
      ${renderNotice(state.exportResult)}
      ${renderTrustSummaryPanel(snapshot, state.pipelineDates, dataHealthSummary)}
      ${renderHealthStrip(status, snapshot)}

      ${renderTimeContext(snapshot)}

      ${state.loading ? renderSkeleton() : ''}

      <section class="dashboard-grid ${(state.loading || state.refreshing || state.refreshStatus.running) ? 'dashboard-grid--dimmed' : ''}">
        <div class="dashboard-grid__status">${renderStatusPanel(status, state.pipelineDates)}</div>
        <div class="dashboard-grid__regime">${renderRegimePanel(snapshot)}</div>
          <div class="dashboard-grid__environment">${environmentBreadthRenderers.renderEnvironmentPanel(snapshot)}</div>
          <div class="dashboard-grid__breadth">${environmentBreadthRenderers.renderWatchlistBreadthPanel(snapshot)}</div>
        <div class="dashboard-grid__rotation">${renderRotationPanel(snapshot)}</div>
        <div class="dashboard-grid__signals">${renderSignalsPanel(snapshot)}</div>
        <div class="dashboard-grid__backtest">${renderBacktestPanel(snapshot)}</div>
        <div class="dashboard-grid__reports">${recentReports.renderRecentReportsPanel()}</div>
        <div class="dashboard-grid__data-health">${dataHealth.renderPanel(dataHealthSummary)}</div>
      </section>
    </main>
    ${usageGuides.renderUsageGuidesViewer()}
    ${renderSignalDetailModal(state.selectedSignalDetail)}
  `;

  document.body.classList.toggle('body--guide-viewer-open', state.isUsageGuideOpen);
  document.body.classList.toggle('body--signal-modal-open', Boolean(state.selectedSignalDetail));

  document.querySelector('#refreshButton').onclick = () => {
    startRefreshJob(state.selectedRefreshStartStage);
  };

  document.querySelector('#refreshStageSelect').onchange = (event) => {
    state.selectedRefreshStartStage = String(event.target.value || 'full');
    render();
  };

  document.querySelector('#reportDateSelect').onchange = (event) => {
    const nextDate = event.target.value;
    if (!nextDate || nextDate === state.selectedReportDate || state.loading) return;

    state.selectedReportDate = nextDate;
    state.exportResult = null;
    state.selectedSignalDetail = null;
    loadSelectedSnapshot();
  };

  document.querySelector('#scopeSelect').onchange = (event) => {
    const nextScope = normalizeScope(event.target.value);
    if (nextScope === state.selectedScope || state.loading) return;

    state.selectedScope = nextScope;
    state.selectedReportDate = '';
    state.snapshot = null;
    state.exportResult = null;
    state.selectedSignalDetail = null;
    loadDashboard();
  };

  document.querySelector('#jumpToLatestButton').onclick = () => {
    const latestAvailableDate = getLatestAvailableDate(state.snapshot);
    if (!latestAvailableDate || latestAvailableDate === getActiveReportDate() || state.loading) return;

    state.selectedReportDate = latestAvailableDate;
    state.exportResult = null;
    state.selectedSignalDetail = null;
    loadSelectedSnapshot();
  };

  document.querySelector('#exportButton').onclick = () => {
    exportReport();
  };

  const retryRefreshButton = document.querySelector('#retryRefreshButton');
  if (retryRefreshButton) {
    retryRefreshButton.onclick = () => {
      retryFailedRefresh();
    };
  }

  document.querySelectorAll('.signal-card--interactive').forEach((button) => {
    button.onclick = () => {
      const group = button.dataset.signalGroup;
      const index = Number(button.dataset.signalIndex);
      const signals = group === 'bullish'
        ? state.snapshot?.bullish_signals
        : group === 'defensive'
          ? state.snapshot?.defensive_signals
          : state.snapshot?.top_signals;
      const signal = Array.isArray(signals) ? signals[index] : null;
      if (!signal) return;

      state.selectedSignalDetail = signal;
      render();
    };
  });

  const closeSignalDetail = () => {
    state.selectedSignalDetail = null;
    render();
  };

  const signalDetailBackdrop = document.querySelector('.signal-detail__backdrop');
  if (signalDetailBackdrop) {
    signalDetailBackdrop.onclick = closeSignalDetail;
  }

  const closeSignalDetailButton = document.querySelector('#closeSignalDetailButton');
  if (closeSignalDetailButton) {
    closeSignalDetailButton.onclick = closeSignalDetail;
  }

  dataHealth.bindEvents(document);
  recentReports.bindEvents(document);
  usageGuides.bindUsageGuideEvents(document);
}

function render() {
  if (renderFrame) return;
  renderFrame = window.requestAnimationFrame(() => {
    renderFrame = 0;
    commitRender();
  });
}

function stopRefreshPolling() {
  if (refreshPollTimer) {
    clearTimeout(refreshPollTimer);
    refreshPollTimer = null;
  }
}

function scheduleRefreshPoll(delay = 1000) {
  stopRefreshPolling();
  if (!state.refreshing && !state.refreshStatus.running) return;
  refreshPollTimer = setTimeout(() => {
    pollRefreshStatus();
  }, delay);
}

async function pollRefreshStatus() {
  try {
    state.refreshStatus = normalizeRefreshStatus(await invoke(COMMANDS.refreshStatus));
    state.refreshing = state.refreshStatus.running;
    render();

    if (state.refreshStatus.running) {
      scheduleRefreshPoll(1000);
      return;
    }

    stopRefreshPolling();
    if (state.refreshStatus.status === 'success') {
      await loadDashboard();
    }
  } catch (error) {
    stopRefreshPolling();
    state.refreshing = false;
    state.refreshStatus = {
      ...state.refreshStatus,
      running: false,
      status: 'error',
      error: getErrorMessage(error),
    };
    render();
  }
}

async function startRefreshJob(startStage = 'full') {
  if (state.refreshing || state.refreshStatus.running) return;

  state.error = '';
  state.refreshing = true;
  state.selectedSignalDetail = null;
  render();

  try {
    state.refreshStatus = normalizeRefreshStatus(await invoke(COMMANDS.startRefresh, {
      startStage: startStage === 'full' ? null : startStage,
    }));
    state.refreshing = state.refreshStatus.running;
    render();
    scheduleRefreshPoll(500);
  } catch (error) {
    state.refreshing = false;
    state.refreshStatus = {
      ...state.refreshStatus,
      running: false,
      status: 'error',
      error: getErrorMessage(error),
    };
    render();
  }
}

async function retryFailedRefresh() {
  if (state.refreshing || state.refreshStatus.running || !state.refreshStatus.retry_from_stage) return;

  state.error = '';
  state.refreshing = true;
  state.selectedSignalDetail = null;
  render();

  try {
    state.refreshStatus = normalizeRefreshStatus(await invoke(COMMANDS.retryRefresh));
    state.refreshing = state.refreshStatus.running;
    render();
    scheduleRefreshPoll(500);
  } catch (error) {
    state.refreshing = false;
    state.refreshStatus = {
      ...state.refreshStatus,
      running: false,
      status: 'error',
      error: getErrorMessage(error),
    };
    render();
  }
}

async function loadDashboard() {
  state.loading = true;
  state.error = '';
  state.selectedSignalDetail = null;
  render();

  try {
    const previousSelectedReportDate = state.selectedReportDate;
    const activeScope = normalizeScope(state.selectedScope);
    const bundleResult = await invoke(COMMANDS.dashboardBundle, {
      reportDate: previousSelectedReportDate || null,
      scope: activeScope,
      recentReportLimit: RECENT_REPORT_LIMIT,
    });

    const errors = [];

    if (bundleResult) {
      state.status = bundleResult.status || null;
      state.pipelineDates = bundleResult.pipeline_dates || null;
      state.availableDates = normalizeAvailableDates(bundleResult.available_dates);
      state.recentReports = normalizeRecentReports(bundleResult.recent_reports, RECENT_REPORT_LIMIT);
      state.snapshot = bundleResult.snapshot || null;
      state.selectedScope = normalizeScope(state.snapshot?.scope || activeScope);
      state.selectedReportDate = state.snapshot?.report_date
        || resolveSelectedReportDate(state.availableDates, previousSelectedReportDate);
      state.refreshStatus = normalizeRefreshStatus(bundleResult.refresh_status);
      state.refreshing = state.refreshStatus.running;
      if (state.refreshStatus.running) {
        scheduleRefreshPoll(1000);
      }
    }

    if (previousSelectedReportDate !== state.selectedReportDate) {
      state.exportResult = null;
    }

    if (
      bundleResult
      || state.snapshot
    ) {
      state.lastUpdatedAt = new Date().toISOString();
    }

    if (errors.length) {
      state.error = errors.join(' · ');
    }
  } catch (error) {
    state.snapshot = null;
    state.error = getErrorMessage(error);
  } finally {
    state.loading = false;
    render();
    if (!dataHealth.isCacheFresh()) {
      void dataHealth.loadSummary();
    }
  }
}

async function loadSelectedSnapshot() {
  state.loading = true;
  state.error = '';
  state.selectedSignalDetail = null;
  render();

  try {
    const activeReportDate = getActiveReportDate();
    const activeScope = normalizeScope(state.selectedScope);
    state.snapshot = await invoke(
      COMMANDS.snapshot,
      activeReportDate ? { reportDate: activeReportDate, scope: activeScope } : { scope: activeScope },
    );

    if (state.snapshot?.report_date) {
      state.selectedScope = normalizeScope(state.snapshot.scope || activeScope);
      state.selectedReportDate = state.snapshot.report_date;
    }

    state.lastUpdatedAt = new Date().toISOString();
  } catch (error) {
    state.snapshot = null;
    state.error = `Dashboard snapshot: ${getErrorMessage(error)}`;
  } finally {
    state.loading = false;
    render();
  }
}

async function exportReport() {
  const activeReportDate = getActiveReportDate();
  if (!state.snapshot || !activeReportDate || state.exporting) return;
  const activeScope = normalizeScope(state.selectedScope);

  state.exporting = true;
  state.exportResult = null;
  render();

  try {
    const result = await invoke(COMMANDS.exportReport, { reportDate: activeReportDate, scope: activeScope });
    const exportedReportDate = result.report_date || activeReportDate;
    state.exportResult = {
      kind: 'success',
      title: 'Report exported',
      message: `Saved report for ${exportedReportDate}.`,
      output_path: result.output_path,
      failed_items: Array.isArray(result.failed_items) ? result.failed_items : [],
    };
    if (result.output_path) {
      const reportType = activeScope === 'cn'
        ? 'DAILY_REPORT_CN'
        : activeScope === 'hk'
          ? 'DAILY_REPORT_HK'
          : 'DAILY_REPORT';
      pushRecentReport(reportType, exportedReportDate, result.output_path);
    }
  } catch (error) {
    state.exportResult = {
      kind: 'error',
      title: 'Export failed',
      message: getErrorMessage(error),
    };
  } finally {
    state.exporting = false;
    render();
  }
}

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && state.isUsageGuideOpen) {
    usageGuides.closeUsageGuides();
  }

  if (event.key === 'Escape' && state.selectedSignalDetail) {
    state.selectedSignalDetail = null;
    render();
  }
});

render();
loadDashboard();

import { invoke } from '@tauri-apps/api/core';
import './styles.css';

const app = document.querySelector('#app');

const RECENT_REPORT_LIMIT = 8;
const DATA_HEALTH_CACHE_MS = 5 * 60 * 1000;

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
  selectedUsageGuideId: '',
  lastUpdatedAt: null,
  refreshStatus: {
    running: false,
    status: 'idle',
    progress_pct: 0,
    stage: 'Idle',
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
  refreshStatus: 'dashboard_refresh_status',
};

function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function formatDate(value) {
  if (!value) return 'Unavailable';
  const dateOnlyMatch = String(value).match(/^(\d{4})-(\d{2})-(\d{2})$/);
  const date = dateOnlyMatch
    ? new Date(Number(dateOnlyMatch[1]), Number(dateOnlyMatch[2]) - 1, Number(dateOnlyMatch[3]))
    : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return escapeHtml(value);
  }
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  }).format(date);
}

function formatDateTime(value) {
  if (!value) return 'Not yet synced';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return escapeHtml(value);
  }
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(date);
}

function normalizeRefreshStatus(payload) {
  return {
    running: Boolean(payload?.running),
    status: String(payload?.status ?? 'idle'),
    progress_pct: Math.max(0, Math.min(100, Number(payload?.progress_pct ?? 0))),
    stage: String(payload?.stage ?? 'Idle'),
    refresh_from: payload?.refresh_from ? String(payload.refresh_from) : null,
    refresh_to: payload?.refresh_to ? String(payload.refresh_to) : null,
    started_at: payload?.started_at ? String(payload.started_at) : null,
    finished_at: payload?.finished_at ? String(payload.finished_at) : null,
    error: payload?.error ? String(payload.error) : null,
  };
}

function formatNumber(value, digits = 2) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return new Intl.NumberFormat(undefined, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(numeric);
}

function getFiniteNumber(value) {
  if (value === null || value === undefined || value === '') return null;
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

function formatInteger(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return new Intl.NumberFormat().format(numeric);
}

function formatPercent(value, digits = 2) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return `${formatNumber(numeric * 100, digits)}%`;
}

function formatDisplayPercent(value, digits = 1) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return `${formatNumber(numeric, digits)}%`;
}

function formatDeltaPoints(value, digits = 1) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  const sign = numeric > 0 ? '+' : '';
  return `${sign}${formatNumber(numeric, digits)} pts`;
}

function formatCurrency(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 0,
  }).format(numeric);
}

function clampScore(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return 0;
  return Math.max(0, Math.min(100, numeric));
}

function prettifyToken(value) {
  return String(value ?? '')
    .replaceAll('_', ' ')
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/\b\w/g, (match) => match.toUpperCase());
}

function formatCanonicalAdjustment(value) {
  const adjustment = String(value ?? '').trim();
  if (!adjustment) return 'Unknown';
  return adjustment.length <= 5 ? adjustment.toUpperCase() : prettifyToken(adjustment);
}

function regimeTone(label) {
  const normalized = String(label ?? '').toLowerCase();
  if (normalized.includes('risk_on')) return 'positive';
  if (normalized.includes('risk_off')) return 'negative';
  return 'neutral';
}

function breadthTone(label) {
  const normalized = String(label ?? '').toLowerCase();
  if (normalized === 'improving' || normalized === 'strong') return 'positive';
  if (normalized === 'weakening' || normalized === 'weak') return 'negative';
  if (normalized === 'near_local_low' || normalized === 'near_local_high') return 'warning';
  if (normalized === 'unavailable') return 'outline';
  return 'neutral';
}

function signalTone(label) {
  const normalized = String(label ?? '').toLowerCase();
  if (normalized.includes('strongbuy') || normalized.includes('buy')) return 'positive';
  if (normalized.includes('sell') || normalized.includes('reduce')) return 'negative';
  return 'neutral';
}

function healthTone(status) {
  const normalized = String(status ?? '').toLowerCase();
  if (normalized === 'healthy') return 'positive';
  if (normalized === 'critical') return 'negative';
  return 'neutral';
}

function dataHealthTone(summary) {
  if (!summary) return 'neutral';
  if (Number(summary.critical_symbols) > 0 || Number(summary.critical_macro_sources) > 0) return 'negative';
  if (Number(summary.review_symbols) > 0 || Number(summary.review_macro_sources) > 0) return 'neutral';
  return 'positive';
}

function getFlaggedSymbols(summary) {
  if (!Array.isArray(summary?.symbols)) return [];

  const weightByStatus = {
    critical: 0,
    review: 1,
    healthy: 2,
  };

  return [...summary.symbols]
    .filter((item) => String(item?.status ?? '').toLowerCase() !== 'healthy')
    .sort((left, right) => {
      const statusDelta = (weightByStatus[String(left?.status ?? '').toLowerCase()] ?? 3)
        - (weightByStatus[String(right?.status ?? '').toLowerCase()] ?? 3);
      if (statusDelta !== 0) return statusDelta;

      const noteDelta = (right?.notes?.length ?? 0) - (left?.notes?.length ?? 0);
      if (noteDelta !== 0) return noteDelta;

      return String(left?.symbol ?? '').localeCompare(String(right?.symbol ?? ''));
    });
}

function getFlaggedMacroSources(summary) {
  if (!Array.isArray(summary?.macro_sources)) return [];

  const weightByStatus = {
    critical: 0,
    review: 1,
    healthy: 2,
  };

  return [...summary.macro_sources]
    .filter((item) => String(item?.status ?? '').toLowerCase() !== 'healthy')
    .sort((left, right) => {
      const statusDelta = (weightByStatus[String(left?.status ?? '').toLowerCase()] ?? 3)
        - (weightByStatus[String(right?.status ?? '').toLowerCase()] ?? 3);
      if (statusDelta !== 0) return statusDelta;
      return String(left?.factor_name ?? '').localeCompare(String(right?.factor_name ?? ''));
    });
}

function formatDateRange(start, end) {
  if (!start && !end) return 'Date range unavailable';
  return `${formatDate(start)} → ${formatDate(end)}`;
}

function normalizeAvailableDates(values) {
  if (!Array.isArray(values)) return [];

  return [...new Set(values.map((value) => String(value ?? '').trim()).filter(Boolean))]
    .sort((left, right) => right.localeCompare(left));
}

function normalizeRecentReports(values) {
  if (!Array.isArray(values)) return [];

  const seen = new Set();

  return values
    .map((item) => ({
      report_type: String(item?.report_type ?? '').trim(),
      report_date: String(item?.report_date ?? '').trim(),
      artifact_path: String(item?.artifact_path ?? '').trim(),
    }))
    .filter((item) => item.report_type && item.report_date && item.artifact_path)
    .filter((item) => {
      const key = `${item.report_type}::${item.report_date}::${item.artifact_path}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .slice(0, RECENT_REPORT_LIMIT);
}

function normalizeUsageGuides(values) {
  if (!Array.isArray(values)) return [];

  const seen = new Set();

  return values
    .map((item, index) => ({
      id: String(item?.id ?? `guide-${index + 1}`).trim(),
      title: String(item?.title ?? '').trim(),
      content: String(item?.content ?? ''),
    }))
    .filter((item) => item.id && item.title)
    .filter((item) => {
      if (seen.has(item.id)) return false;
      seen.add(item.id);
      return true;
    });
}

function resolveSelectedUsageGuide(guides, currentGuideId) {
  if (!guides.length) return '';
  if (currentGuideId && guides.some((guide) => guide.id === currentGuideId)) return currentGuideId;
  return guides[0].id;
}

function getSelectedUsageGuide() {
  return state.usageGuides.find((guide) => guide.id === state.selectedUsageGuideId) || state.usageGuides[0] || null;
}

function resolveSelectedReportDate(availableDates, currentDate) {
  if (!availableDates.length) return '';
  if (currentDate && availableDates.includes(currentDate)) return currentDate;
  return availableDates[0];
}

function formatReportType(value) {
  const normalized = String(value ?? '').trim().toUpperCase();

  if (normalized === 'DAILY_REPORT') return 'Daily report';
  if (normalized === 'DAILY_REPORT_CN') return 'Daily report (CN)';
  if (normalized === 'DAILY_REPORT_HK') return 'Daily report (HK)';
  if (normalized === 'DATA_HEALTH_REPORT') return 'Data health';

  return prettifyToken(value);
}

function parseDateValue(value) {
  if (!value) return null;

  const dateOnlyMatch = String(value).match(/^(\d{4})-(\d{2})-(\d{2})$/);
  if (dateOnlyMatch) {
    return Date.UTC(
      Number(dateOnlyMatch[1]),
      Number(dateOnlyMatch[2]) - 1,
      Number(dateOnlyMatch[3]),
    );
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  return date.getTime();
}

function normalizeScope(value) {
  const normalized = String(value ?? 'global').trim().toLowerCase();
  if (normalized === 'cn' || normalized === 'hk') return normalized;
  return 'global';
}

function formatScopeLabel(value) {
  const normalized = normalizeScope(value);
  if (normalized === 'cn') return 'CN';
  if (normalized === 'hk') return 'HK';
  return 'GLOBAL';
}

function isDataHealthCacheFresh() {
  if (!state.dataHealth || !state.dataHealthFetchedAt) return false;
  const fetchedAt = new Date(state.dataHealthFetchedAt).getTime();
  if (!Number.isFinite(fetchedAt)) return false;
  return (Date.now() - fetchedAt) < DATA_HEALTH_CACHE_MS;
}

function getDayDifference(earlier, later) {
  const earlierValue = parseDateValue(earlier);
  const laterValue = parseDateValue(later);

  if (!Number.isFinite(earlierValue) || !Number.isFinite(laterValue)) {
    return null;
  }

  return Math.round((laterValue - earlierValue) / 86400000);
}

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
  ]);
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

function formatFallbackState(value) {
  if (value === true) return 'Fallback ok';
  if (value === false) return 'Fallback down';
  return 'Fallback n/a';
}

function getErrorMessage(error) {
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object' && 'message' in error) {
    return String(error.message);
  }
  return 'Unable to complete the request.';
}

function renderMarkdownInline(value) {
  let html = escapeHtml(value);
  const inlineCodeTokens = [];

  html = html.replace(/`([^`]+)`/g, (_, code) => {
    const token = `__INLINE_CODE_${inlineCodeTokens.length}__`;
    inlineCodeTokens.push(`<code>${code}</code>`);
    return token;
  });

  html = html.replace(/\[([^\]]+)\]\((https?:\/\/[^\s)]+)\)/g, (_, label, url) => (
    `<a href="${url}" target="_blank" rel="noreferrer">${label}</a>`
  ));
  html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*([^*]+)\*/g, '<em>$1</em>');

  return html.replace(/__INLINE_CODE_(\d+)__/g, (_, index) => inlineCodeTokens[Number(index)] || '');
}

function renderMarkdownContent(value) {
  const lines = String(value ?? '').replace(/\r\n/g, '\n').split('\n');
  const blocks = [];
  let paragraphLines = [];
  let listType = '';
  let listItems = [];
  let quoteLines = [];
  let codeFenceActive = false;
  let codeFenceLanguage = '';
  let codeFenceLines = [];

  const flushParagraph = () => {
    if (!paragraphLines.length) return;
    blocks.push(`<p>${paragraphLines.map((line) => renderMarkdownInline(line)).join(' ')}</p>`);
    paragraphLines = [];
  };

  const flushList = () => {
    if (!listItems.length || !listType) return;
    blocks.push(`<${listType}>${listItems.map((item) => `<li>${item}</li>`).join('')}</${listType}>`);
    listItems = [];
    listType = '';
  };

  const flushQuote = () => {
    if (!quoteLines.length) return;
    blocks.push(`<blockquote><p>${quoteLines.map((line) => renderMarkdownInline(line)).join('<br>')}</p></blockquote>`);
    quoteLines = [];
  };

  const flushContentBlocks = () => {
    flushParagraph();
    flushList();
    flushQuote();
  };

  const flushCodeBlock = () => {
    const languageClass = codeFenceLanguage ? ` class="language-${escapeHtml(codeFenceLanguage)}"` : '';
    blocks.push(`<pre><code${languageClass}>${escapeHtml(codeFenceLines.join('\n'))}</code></pre>`);
    codeFenceActive = false;
    codeFenceLanguage = '';
    codeFenceLines = [];
  };

  for (const rawLine of lines) {
    const trimmedLine = rawLine.trim();

    if (codeFenceActive) {
      if (trimmedLine.startsWith('```')) {
        flushCodeBlock();
      } else {
        codeFenceLines.push(rawLine);
      }
      continue;
    }

    if (!trimmedLine) {
      flushContentBlocks();
      continue;
    }

    if (trimmedLine.startsWith('```')) {
      flushContentBlocks();
      codeFenceActive = true;
      codeFenceLanguage = trimmedLine.slice(3).trim();
      codeFenceLines = [];
      continue;
    }

    const headingMatch = trimmedLine.match(/^(#{1,6})\s+(.+)$/);
    if (headingMatch) {
      flushContentBlocks();
      const level = headingMatch[1].length;
      blocks.push(`<h${level}>${renderMarkdownInline(headingMatch[2].trim())}</h${level}>`);
      continue;
    }

    if (/^(-{3,}|\*{3,})$/.test(trimmedLine)) {
      flushContentBlocks();
      blocks.push('<hr>');
      continue;
    }

    const quoteMatch = rawLine.match(/^\s*>\s?(.*)$/);
    if (quoteMatch) {
      flushParagraph();
      flushList();
      quoteLines.push(quoteMatch[1]);
      continue;
    }

    flushQuote();

    const unorderedListMatch = rawLine.match(/^\s*[-*+]\s+(.+)$/);
    if (unorderedListMatch) {
      flushParagraph();
      if (listType && listType !== 'ul') flushList();
      listType = 'ul';
      listItems.push(renderMarkdownInline(unorderedListMatch[1]));
      continue;
    }

    const orderedListMatch = rawLine.match(/^\s*\d+\.\s+(.+)$/);
    if (orderedListMatch) {
      flushParagraph();
      if (listType && listType !== 'ol') flushList();
      listType = 'ol';
      listItems.push(renderMarkdownInline(orderedListMatch[1]));
      continue;
    }

    if (listItems.length && /^\s{2,}\S/.test(rawLine)) {
      listItems[listItems.length - 1] = `${listItems[listItems.length - 1]}<br>${renderMarkdownInline(trimmedLine)}`;
      continue;
    }

    flushList();
    paragraphLines.push(trimmedLine);
  }

  if (codeFenceActive) {
    flushCodeBlock();
  }

  flushContentBlocks();

  return blocks.join('');
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

function renderHealthStrip(status, snapshot, dataHealth) {
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
      label: 'Data health',
      value: dataHealth
        ? Number(dataHealth.critical_symbols) > 0
          ? `${formatInteger(dataHealth.critical_symbols)} critical · ${formatInteger(dataHealth.review_symbols)} review`
          : Number(dataHealth.review_symbols) > 0
            ? `${formatInteger(dataHealth.review_symbols)} review symbols`
            : 'All checked symbols healthy'
        : 'No health summary yet',
      tone: dataHealthTone(dataHealth),
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

function renderEnvironmentPanel(snapshot) {
  const environment = snapshot?.environment;

  if (!environment) {
    return `
      <article class="panel panel--soft">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Environment layer</p>
            <h2>Unavailable</h2>
            <p class="panel__lede">Run the macro/environment pipeline to populate per-scope environment diagnostics.</p>
          </div>
        </div>
        <div class="empty-state">
          <p>No environment snapshot is available for the selected report date.</p>
        </div>
      </article>
    `;
  }

  const scores = [
    ['Environment score', environment.environment_score],
    ['Liquidity proxy', environment.liquidity_proxy_score],
    ['Stress proxy', environment.stress_proxy_score],
  ];

  return `
    <article class="panel panel--soft">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Environment layer</p>
          <h2>${escapeHtml(prettifyToken(environment.environment_label))}</h2>
          <p class="panel__lede">Scope-aware participation, liquidity proxy, and stress posture for the selected report date.</p>
        </div>
        <div class="panel__actions">
          <span class="pill pill--${breadthTone(environment.breadth_state)}">Breadth · ${escapeHtml(prettifyToken(environment.breadth_state))}</span>
          <span class="pill pill--outline">Regime as-of · ${escapeHtml(formatDate(environment.regime_as_of_date))}</span>
        </div>
      </div>
      <div class="mini-metrics">
        ${renderMetricCard('Breadth', formatDisplayPercent(environment.breadth_pct, 1), `${formatInteger(environment.breadth_above_count)} / ${formatInteger(environment.breadth_eligible_count)} above MA30`, 'neutral')}
        ${renderMetricCard('Breadth SMA5', formatDisplayPercent(environment.breadth_pct_sma5, 1), 'Smoothed participation', 'neutral')}
        ${renderMetricCard('Breadth 5d', formatDeltaPoints(environment.breadth_5d_delta, 1), 'Repair or deterioration speed', 'neutral')}
        ${renderMetricCard('Volume expansion', formatDisplayPercent(environment.volume_expansion_pct, 1), 'Share of tracked symbols above vol_ma20', 'neutral')}
      </div>
      <div class="score-stack environment-panel__scores">
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
      <p class="breadth-panel__note">Tracked-universe proxy only. Environment breadth remains based on enabled INDEX + ETF instruments, not a full stock-universe breadth series.</p>
    </article>
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
  const bullishSignals = snapshot?.bullish_signals || [];
  const defensiveSignals = snapshot?.defensive_signals || [];

  if (!bullishSignals.length && !defensiveSignals.length) {
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

  return `
    <article class="panel">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Signal stack</p>
          <h2>Buy & defensive groups</h2>
          <p class="panel__lede">Bullish opportunities separated from defensive or sell-side signals for the selected report date.</p>
        </div>
        <span class="panel__meta">Grouped signal view for ${escapeHtml(snapshot.report_date)}</span>
      </div>
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
                    (item) => `
                      <article class="signal-card signal-card--bullish">
                        <div class="signal-card__header">
                          <div>
                            <strong class="signal-card__symbol">${escapeHtml(item.symbol)}</strong>
                            <p class="signal-card__score">Score ${escapeHtml(formatNumber(item.final_score, 2))}</p>
                          </div>
                          <span class="pill pill--${signalTone(item.signal_label)}">${escapeHtml(prettifyToken(item.signal_label))}</span>
                        </div>
                        <p class="signal-card__text">${escapeHtml(item.explanation)}</p>
                      </article>
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
                    (item) => `
                      <article class="signal-card signal-card--defensive">
                        <div class="signal-card__header">
                          <div>
                            <strong class="signal-card__symbol">${escapeHtml(item.symbol)}</strong>
                            <p class="signal-card__score">Score ${escapeHtml(formatNumber(item.final_score, 2))}</p>
                          </div>
                          <span class="pill pill--${signalTone(item.signal_label)}">${escapeHtml(prettifyToken(item.signal_label))}</span>
                        </div>
                        <p class="signal-card__text">${escapeHtml(item.explanation)}</p>
                      </article>
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

function renderWatchlistBreadthPanel(snapshot) {
  const breadth = snapshot?.watchlist_breadth;

  if (!breadth?.markets?.length) {
    return `
      <article class="panel panel--soft">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Participation</p>
            <h2>Watchlist Breadth (MA30)</h2>
            <p class="panel__lede">Tracked INDEX + ETF participation above MA30 for the selected report date.</p>
          </div>
          <span class="pill pill--outline">Proxy only</span>
        </div>
        <div class="empty-state">
          <p>Watchlist breadth is unavailable for this dashboard snapshot.</p>
        </div>
      </article>
    `;
  }

  const marketCards = breadth.markets
    .map((market) => {
      const eligibleCount = Number(market?.eligible_count ?? 0);
      const aboveCount = Number(market?.above_count ?? 0);
      const unavailable = String(market?.status_label ?? '').toLowerCase() === 'unavailable' || eligibleCount === 0;
      const breadthValue = unavailable ? 'Unavailable' : formatDisplayPercent(market?.breadth_pct, 1);
      const summaryText = unavailable
        ? 'No eligible instruments with both close and MA30 on this date.'
        : `${formatInteger(aboveCount)} / ${formatInteger(eligibleCount)} above MA30`;
      const rangePosition = getFiniteNumber(market?.range_position_60d);

      return `
        <section class="breadth-market ${unavailable ? 'breadth-market--unavailable' : ''}">
          <div class="panel__subheader">
            <p class="panel__section-title">${escapeHtml(market?.universe_label || `${market?.market || 'Tracked'} universe`)}</p>
            <span class="pill pill--${breadthTone(market?.status_label)}">${escapeHtml(prettifyToken(market?.status_label || 'Unavailable'))}</span>
          </div>

          <div class="breadth-market__headline">
            <strong class="breadth-market__value">${escapeHtml(breadthValue)}</strong>
            <span class="breadth-market__summary">${escapeHtml(summaryText)}</span>
          </div>

          <div class="breadth-market__metrics">
            <article class="breadth-market__metric">
              <span class="breadth-market__metric-label">SMA5</span>
              <strong class="breadth-market__metric-value">${escapeHtml(formatDisplayPercent(market?.breadth_pct_sma5, 1))}</strong>
            </article>
            <article class="breadth-market__metric">
              <span class="breadth-market__metric-label">5d delta</span>
              <strong class="breadth-market__metric-value">${escapeHtml(formatDeltaPoints(market?.breadth_5d_delta, 1))}</strong>
            </article>
            <article class="breadth-market__metric">
              <span class="breadth-market__metric-label">60d range</span>
              <strong class="breadth-market__metric-value">${escapeHtml(rangePosition === null ? 'N/A' : formatPercent(rangePosition, 0))}</strong>
            </article>
          </div>

          <div class="score-row breadth-market__range-row">
            <div class="score-row__meta">
              <span>Current breadth</span>
              <strong>${escapeHtml(unavailable ? 'N/A' : formatDisplayPercent(market?.breadth_pct, 1))}</strong>
            </div>
            <div class="score-bar" aria-hidden="true">
              <span class="score-bar__fill" style="width: ${rangePosition === null ? 0 : clampScore(rangePosition * 100)}%"></span>
            </div>
          </div>
        </section>
      `;
    })
    .join('');

  return `
    <article class="panel panel--soft">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Participation</p>
          <h2>Watchlist Breadth (MA30)</h2>
          <p class="panel__lede">Tracked INDEX + ETF participation above MA30 for the selected report date.</p>
        </div>
        <span class="pill pill--outline">Proxy only · not full-market stock breadth</span>
      </div>
      <div class="breadth-market-grid">
        ${marketCards}
      </div>
      <p class="breadth-panel__note">${escapeHtml(breadth.methodology_note || 'Eligible tracked instruments require both close and MA30 on the selected date.')}</p>
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

  return `
    <article class="panel panel--soft">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Validation</p>
          <h2>Latest backtest</h2>
          <p class="panel__lede">Recent strategy validation snapshot generated from the same pipeline.</p>
        </div>
        <span class="panel__meta">${escapeHtml(backtest.strategy_name)}</span>
      </div>
      <div class="mini-metrics">
        ${renderMetricCard('CAGR', formatPercent(backtest.cagr), 'Annualized return', 'positive')}
        ${renderMetricCard('Max drawdown', formatPercent(backtest.max_drawdown), 'Peak-to-trough', 'negative')}
        ${renderMetricCard('Sharpe', formatNumber(backtest.sharpe, 2), 'Risk-adjusted', 'neutral')}
        ${renderMetricCard('Final equity', formatCurrency(backtest.final_equity), `${formatInteger(backtest.trades)} trades · ${formatInteger(backtest.trading_days)} days`, 'neutral')}
      </div>
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

  return `
    <section class="refresh-progress refresh-progress--${tone}" aria-live="polite">
      <div class="refresh-progress__header">
        <div>
          <p class="eyebrow">Background refresh</p>
          <h2>${escapeHtml(refresh.running ? 'Refreshing analysis pipeline' : refresh.status === 'error' ? 'Refresh failed' : 'Refresh completed')}</h2>
          <p class="panel__lede">${escapeHtml(refresh.stage || 'Waiting')}</p>
        </div>
        <span class="pill pill--${tone}">${escapeHtml(`${formatInteger(progress)}%`)}</span>
      </div>
      <div class="refresh-progress__bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${escapeHtml(progress)}">
        <span class="refresh-progress__fill" style="width: ${progress}%"></span>
      </div>
      <div class="refresh-progress__meta-row">
        <span>${escapeHtml(rangeText)}</span>
        <span>${escapeHtml(timingText)}</span>
      </div>
      ${refresh.error ? `<p class="refresh-progress__error">${escapeHtml(refresh.error)}</p>` : ''}
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

function renderUsageEntry() {
  const guideCount = state.usageGuides.length;
  const statusText = state.usageGuidesLoading
    ? 'Loading manuals from the desktop runtime.'
    : state.usageGuidesError
      ? 'Guide content is temporarily unavailable. Open the viewer to retry.'
      : state.usageGuidesLoaded
        ? `${formatInteger(guideCount)} in-app guide${guideCount === 1 ? '' : 's'} ready for reading.`
        : 'Open the in-app manuals for day-to-day operations and analysis workflow guidance.';

  return `
    <div class="hero__control hero__control--guide">
      <div class="control-field">
        <div class="control-field__header">
          <span class="control-field__label">Help / Usage</span>
          <span class="pill pill--${state.usageGuidesError ? 'negative' : 'outline'}">
            ${state.usageGuidesLoading ? 'Syncing guides' : state.usageGuidesLoaded ? `${escapeHtml(formatInteger(guideCount))} guides` : 'Guide viewer'}
          </span>
        </div>
        <span class="control-field__hint">${escapeHtml(statusText)}</span>
        <button
          id="openUsageGuidesButton"
          class="button button--secondary guide-entry__button"
          ${state.usageGuidesLoading && !state.isUsageGuideOpen ? 'disabled' : ''}
        >
          ${state.usageGuidesLoading && state.isUsageGuideOpen ? 'Loading guides…' : 'Open guides'}
        </button>
      </div>
    </div>
  `;
}

function renderRecentReportsPanel() {
  return `
    <article class="panel panel--soft">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Exports</p>
          <h2>Recent reports</h2>
          <p class="panel__lede">Recently exported report artifacts for quick recall after browsing historical analysis dates.</p>
        </div>
        <span class="panel__meta">Latest ${escapeHtml(formatInteger(state.recentReports.length))} exports</span>
      </div>

      ${state.recentReports.length
        ? `
          <div class="report-history" aria-label="Recent report history">
            ${state.recentReports
              .map(
                (item) => `
                  <article class="report-history__item">
                    <div class="report-history__row">
                      <span class="pill pill--outline">${escapeHtml(formatReportType(item.report_type))}</span>
                      <span class="report-history__date">${escapeHtml(formatDate(item.report_date))}</span>
                    </div>
                    <p class="report-history__path"><code>${escapeHtml(item.artifact_path)}</code></p>
                  </article>
                `,
              )
              .join('')}
          </div>
        `
        : `
          <div class="empty-state empty-state--compact">
            <p>No report artifacts have been exported yet.</p>
          </div>
        `}
    </article>
  `;
}

function renderUsageGuidesViewer() {
  const selectedGuide = getSelectedUsageGuide();
  const guideCount = state.usageGuides.length;

  return `
    <section class="guide-viewer ${state.isUsageGuideOpen ? 'guide-viewer--open' : ''}" aria-hidden="${state.isUsageGuideOpen ? 'false' : 'true'}">
      <button class="guide-viewer__backdrop" type="button" data-guide-close="true" aria-label="Close guide viewer"></button>

      <div class="guide-viewer__panel" role="dialog" aria-modal="true" aria-labelledby="usageGuideViewerTitle">
        <div class="guide-viewer__header">
          <div>
            <p class="eyebrow">Help / Usage</p>
            <h2 id="usageGuideViewerTitle">Guide library</h2>
            <p class="panel__lede">Read the desktop usage manuals without leaving the dashboard.</p>
          </div>
          <div class="panel__actions">
            <span class="pill pill--outline">${escapeHtml(formatInteger(guideCount))} available</span>
            <button
              id="reloadUsageGuidesButton"
              class="button button--secondary button--compact"
              ${state.usageGuidesLoading ? 'disabled' : ''}
            >
              ${state.usageGuidesLoading ? 'Refreshing…' : 'Reload'}
            </button>
            <button id="closeUsageGuidesButton" class="button button--secondary button--compact">Close</button>
          </div>
        </div>

        <div class="guide-viewer__body">
          <aside class="guide-viewer__sidebar" aria-label="Usage guide navigation">
            ${guideCount
              ? `
                <div class="guide-tab-list" role="tablist" aria-label="Usage guides">
                  ${state.usageGuides
                    .map(
                      (guide) => `
                        <button
                          class="guide-tab ${guide.id === state.selectedUsageGuideId ? 'guide-tab--active' : ''}"
                          type="button"
                          role="tab"
                          aria-selected="${guide.id === state.selectedUsageGuideId ? 'true' : 'false'}"
                          data-guide-id="${escapeHtml(guide.id)}"
                        >
                          <span class="guide-tab__eyebrow">Manual</span>
                          <strong class="guide-tab__title">${escapeHtml(guide.title)}</strong>
                        </button>
                      `,
                    )
                    .join('')}
                </div>
              `
              : `
                <div class="empty-state empty-state--compact guide-viewer__empty-nav">
                  <p>${state.usageGuidesLoading ? 'Preparing guide navigation…' : 'No guides loaded yet.'}</p>
                </div>
              `}
          </aside>

          <div class="guide-viewer__content">
            ${state.usageGuidesLoading && !guideCount
              ? `
                <div class="empty-state">
                  <p>Loading in-app manuals from the desktop runtime…</p>
                </div>
              `
              : state.usageGuidesError
                ? `
                  <section class="notice notice--error notice--inline">
                    <div>
                      <strong>Guide library unavailable</strong>
                      <p>${escapeHtml(state.usageGuidesError)}</p>
                    </div>
                  </section>
                `
                : !selectedGuide
                  ? `
                    <div class="empty-state">
                      <p>No usage guides were returned by the desktop runtime.</p>
                    </div>
                  `
                  : `
                    <article class="guide-article">
                      <header class="guide-article__header">
                        <p class="eyebrow">Manual</p>
                        <h3>${escapeHtml(selectedGuide.title)}</h3>
                      </header>
                      <div class="guide-article__content">${renderMarkdownContent(selectedGuide.content)}</div>
                    </article>
                  `}
          </div>
        </div>
      </div>
    </section>
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

function renderDataHealthPanel(summary) {
  const flaggedSymbols = getFlaggedSymbols(summary);
  const flaggedMacroSources = getFlaggedMacroSources(summary);
  const exportDisabled = state.loading || state.refreshing || state.refreshStatus.running || state.dataHealthLoading || state.dataHealthExporting || !summary;
  const refreshHealthDisabled = state.dataHealthLoading || state.refreshing || state.refreshStatus.running;
  const healthStatusMeta = state.dataHealthLoading
    ? 'Refreshing health summary…'
    : state.dataHealthError
      ? `Health refresh issue · ${state.dataHealthError}`
      : summary
        ? `Generated ${formatDateTime(summary.generated_at)} · session cache active`
        : 'Health summary not loaded';

  if (!summary) {
    return `
      <article class="panel panel--soft">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Data quality</p>
            <h2>Data health</h2>
            <p class="panel__lede">Coverage, provider reachability, and anomaly checks will appear here after the summary loads.</p>
          </div>
          <div class="panel__actions">
            <span class="pill pill--outline">${escapeHtml(healthStatusMeta)}</span>
            <button id="refreshHealthButton" class="button button--secondary button--compact" ${refreshHealthDisabled ? 'disabled' : ''}>
              ${state.dataHealthLoading ? 'Refreshing…' : 'Load health summary'}
            </button>
            <button id="exportHealthButton" class="button button--secondary button--compact" disabled>
              Export data-health report
            </button>
          </div>
        </div>
        <div class="empty-state">
          <p>${state.dataHealthLoading ? 'Refreshing data-health summary in the background…' : 'Data-health summary is unavailable.'}</p>
        </div>
      </article>
    `;
  }

  return `
    <article class="panel panel--soft">
      <div class="panel__header">
        <div>
          <p class="eyebrow">Data quality</p>
          <h2>Data health</h2>
          <p class="panel__lede">Provider availability and daily-bar sanity checks for the active universe.</p>
        </div>
        <div class="panel__actions">
          <span class="pill pill--outline">Canonical ${escapeHtml(formatCanonicalAdjustment(summary.canonical_adjustment))}</span>
          <button
            id="refreshHealthButton"
            class="button button--secondary button--compact"
            ${refreshHealthDisabled ? 'disabled' : ''}
          >
            ${state.dataHealthLoading ? 'Refreshing…' : 'Refresh health summary'}
          </button>
          <button
            id="exportHealthButton"
            class="button button--secondary button--compact"
            ${exportDisabled ? 'disabled' : ''}
          >
            ${state.dataHealthExporting ? 'Exporting…' : 'Export data-health report'}
          </button>
        </div>
      </div>

      <div class="panel__meta-row">
        <span class="panel__meta">${escapeHtml(healthStatusMeta)}</span>
        <span class="panel__meta">${escapeHtml(formatInteger(summary.checked_symbols))} symbols checked</span>
        <span class="panel__meta">Freshest market date · ${escapeHtml(formatDate(summary.freshest_market_date))}</span>
      </div>

      <div class="mini-metrics">
        ${renderMetricCard('Healthy', formatInteger(summary.healthy_symbols), 'Clear for use', Number(summary.healthy_symbols) > 0 ? 'positive' : 'neutral')}
        ${renderMetricCard('Review', formatInteger(summary.review_symbols), 'Needs analyst review', Number(summary.review_symbols) > 0 ? 'neutral' : 'positive')}
        ${renderMetricCard('Critical', formatInteger(summary.critical_symbols), 'Needs immediate follow-up', Number(summary.critical_symbols) > 0 ? 'negative' : 'neutral')}
        ${renderMetricCard('Checked', formatInteger(summary.checked_symbols), 'Universe coverage', 'neutral')}
      </div>

      <div class="mini-metrics">
        ${renderMetricCard('Latest-day coverage', `${formatInteger(summary.symbols_on_freshest_market_date)}/${formatInteger(summary.checked_symbols)}`, 'Symbols with bars on the freshest stored market date', summary.freshest_market_date_complete ? 'positive' : 'warning')}
        ${renderMetricCard('Missing latest-day', formatInteger(summary.symbols_missing_freshest_market_date), 'Symbols not updated on the freshest stored market date', Number(summary.symbols_missing_freshest_market_date) > 0 ? 'warning' : 'positive')}
        ${renderMetricCard('Freshest date complete', summary.freshest_market_date_complete ? 'Yes' : 'No', 'Latest stored market date has full symbol coverage', summary.freshest_market_date_complete ? 'positive' : 'warning')}
        ${renderMetricCard('Freshest date', formatDate(summary.freshest_market_date), 'Reference date for latest-day coverage checks', 'neutral')}
      </div>

      <div class="mini-metrics">
        ${renderMetricCard('Macro healthy', formatInteger(summary.healthy_macro_sources), 'Primary reqwest path', Number(summary.healthy_macro_sources) > 0 ? 'positive' : 'neutral')}
        ${renderMetricCard('Macro review', formatInteger(summary.review_macro_sources), 'Compatibility fallback in use', Number(summary.review_macro_sources) > 0 ? 'neutral' : 'positive')}
        ${renderMetricCard('Macro critical', formatInteger(summary.critical_macro_sources), 'Macro source unavailable', Number(summary.critical_macro_sources) > 0 ? 'negative' : 'neutral')}
        ${renderMetricCard('Macro sources', formatInteger(summary.macro_sources?.length ?? 0), 'FRED factor coverage', 'neutral')}
      </div>

      ${renderNotice(state.dataHealthExportResult, 'notice--inline')}

      <section class="data-health__review-block">
        <div class="panel__subheader">
          <p class="panel__section-title">Macro source status</p>
          <span class="panel__meta">
            ${flaggedMacroSources.length
              ? `${escapeHtml(formatInteger(flaggedMacroSources.length))} source${flaggedMacroSources.length === 1 ? '' : 's'} degraded`
              : 'All macro sources on primary path'}
          </span>
        </div>

        ${flaggedMacroSources.length
          ? `
            <div class="table-wrap">
              <table class="data-table data-table--compact">
                <thead>
                  <tr>
                    <th>Factor</th>
                    <th>Status</th>
                    <th>Transport</th>
                    <th>Coverage</th>
                    <th>Notes</th>
                  </tr>
                </thead>
                <tbody>
                  ${flaggedMacroSources
                    .map(
                      (item) => `
                        <tr>
                          <td>
                            <div class="table-symbol">
                              <strong class="data-table__symbol">${escapeHtml(prettifyToken(item.factor_name))}</strong>
                              <span class="table-symbol__meta">${escapeHtml(item.source)}</span>
                            </div>
                          </td>
                          <td>
                            <span class="pill pill--${healthTone(item.status)}">${escapeHtml(prettifyToken(item.status))}</span>
                          </td>
                          <td>
                            <span class="pill pill--outline">${escapeHtml(item.transport)}</span>
                          </td>
                          <td>
                            <div class="table-stack">
                              <strong>${escapeHtml(formatInteger(item.rows))} rows</strong>
                              <span class="table-stack__meta">${escapeHtml(formatDateRange(item.first_date, item.last_date))}</span>
                            </div>
                          </td>
                          <td>
                            ${item.notes?.length
                              ? `<ul class="note-list">${item.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join('')}</ul>`
                              : '<span class="table-stack__meta">No notes provided</span>'}
                          </td>
                        </tr>
                      `,
                    )
                    .join('')}
                </tbody>
              </table>
            </div>
          `
          : `
            <div class="empty-state empty-state--compact">
              <p>All macro sources are currently using the primary transport path.</p>
            </div>
          `}
      </section>

      <section class="data-health__review-block">
        <div class="panel__subheader">
          <p class="panel__section-title">Review queue</p>
          <span class="panel__meta">
            ${flaggedSymbols.length
              ? `${escapeHtml(formatInteger(flaggedSymbols.length))} symbol${flaggedSymbols.length === 1 ? '' : 's'} flagged`
              : 'No symbols currently flagged'}
          </span>
        </div>

        ${flaggedSymbols.length
          ? `
            <div class="table-wrap">
              <table class="data-table data-table--compact">
                <thead>
                  <tr>
                    <th>Symbol</th>
                    <th>Status</th>
                    <th>Coverage</th>
                    <th>Checks</th>
                    <th>Notes</th>
                  </tr>
                </thead>
                <tbody>
                  ${flaggedSymbols
                    .map(
                      (item) => `
                        <tr>
                          <td>
                            <div class="table-symbol">
                              <strong class="data-table__symbol">${escapeHtml(item.display_symbol || item.symbol)}</strong>
                              <span class="table-symbol__meta">${escapeHtml(item.name)}${item.display_symbol ? ` · ${escapeHtml(item.symbol)}` : ''}</span>
                            </div>
                          </td>
                          <td>
                            <span class="pill pill--${healthTone(item.status)}">${escapeHtml(prettifyToken(item.status))}</span>
                          </td>
                          <td>
                            <div class="table-stack">
                              <strong>${escapeHtml(formatInteger(item.rows))} rows</strong>
                              <span class="table-stack__meta">${escapeHtml(formatDateRange(item.first_date, item.last_date))}</span>
                            </div>
                          </td>
                          <td>
                            <div class="table-flags">
                              <span>${escapeHtml(`${item.primary_provider_ok ? 'Primary ok' : 'Primary down'} · ${formatFallbackState(item.fallback_provider_ok)}`)}</span>
                              <span>${escapeHtml(`${formatInteger(item.gap_count)} gaps · max ${formatInteger(item.max_gap_days)}d`)}</span>
                              <span>${escapeHtml(`${formatInteger(item.suspicious_jump_count)} jumps · max move ${formatNumber(item.max_abs_daily_return_pct, 1)}%`)}</span>
                              <span>${escapeHtml(`${formatInteger(item.missing_turnover_rows)} turnover missing`)}</span>
                            </div>
                          </td>
                          <td>
                            ${item.notes?.length
                              ? `<ul class="note-list">${item.notes.map((note) => `<li>${escapeHtml(note)}</li>`).join('')}</ul>`
                              : '<span class="table-stack__meta">No notes provided</span>'}
                          </td>
                        </tr>
                      `,
                    )
                    .join('')}
                </tbody>
              </table>
            </div>
          `
          : `
            <div class="empty-state empty-state--compact">
              <p>All checked symbols passed the current health thresholds.</p>
            </div>
          `}
      </section>
    </article>
  `;
}

function commitRender() {
  const { status, snapshot, dataHealth } = state;
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
            ${renderUsageEntry()}
            <div class="hero__action-row">
              <button id="refreshButton" class="button button--secondary" ${(state.loading || state.refreshing || state.refreshStatus.running) ? 'disabled' : ''}>
                ${(state.refreshing || state.refreshStatus.running) ? 'Refreshing…' : 'Refresh data'}
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
      ${renderHealthStrip(status, snapshot, dataHealth)}

      ${renderTimeContext(snapshot)}

      ${state.loading ? renderSkeleton() : ''}

      <section class="dashboard-grid ${(state.loading || state.refreshing || state.refreshStatus.running) ? 'dashboard-grid--dimmed' : ''}">
        <div class="dashboard-grid__status">${renderStatusPanel(status, state.pipelineDates)}</div>
        <div class="dashboard-grid__regime">${renderRegimePanel(snapshot)}</div>
          <div class="dashboard-grid__environment">${renderEnvironmentPanel(snapshot)}</div>
          <div class="dashboard-grid__breadth">${renderWatchlistBreadthPanel(snapshot)}</div>
        <div class="dashboard-grid__rotation">${renderRotationPanel(snapshot)}</div>
        <div class="dashboard-grid__signals">${renderSignalsPanel(snapshot)}</div>
        <div class="dashboard-grid__backtest">${renderBacktestPanel(snapshot)}</div>
        <div class="dashboard-grid__reports">${renderRecentReportsPanel()}</div>
        <div class="dashboard-grid__data-health">${renderDataHealthPanel(dataHealth)}</div>
      </section>
    </main>
    ${renderUsageGuidesViewer()}
  `;

  document.body.classList.toggle('body--guide-viewer-open', state.isUsageGuideOpen);

  document.querySelector('#refreshButton').onclick = () => {
    startRefreshJob();
  };

  document.querySelector('#openUsageGuidesButton').onclick = () => {
    openUsageGuides();
  };

  document.querySelector('#reportDateSelect').onchange = (event) => {
    const nextDate = event.target.value;
    if (!nextDate || nextDate === state.selectedReportDate || state.loading) return;

    state.selectedReportDate = nextDate;
    state.exportResult = null;
    loadSelectedSnapshot();
  };

  document.querySelector('#scopeSelect').onchange = (event) => {
    const nextScope = normalizeScope(event.target.value);
    if (nextScope === state.selectedScope || state.loading) return;

    state.selectedScope = nextScope;
    state.selectedReportDate = '';
    state.snapshot = null;
    state.exportResult = null;
    loadDashboard();
  };

  document.querySelector('#jumpToLatestButton').onclick = () => {
    const latestAvailableDate = getLatestAvailableDate(state.snapshot);
    if (!latestAvailableDate || latestAvailableDate === getActiveReportDate() || state.loading) return;

    state.selectedReportDate = latestAvailableDate;
    state.exportResult = null;
    loadSelectedSnapshot();
  };

  document.querySelector('#exportButton').onclick = () => {
    exportReport();
  };

  document.querySelector('#exportHealthButton').onclick = () => {
    exportDataHealthReport();
  };

  document.querySelector('#refreshHealthButton').onclick = () => {
    void loadDataHealthSummary({ force: true });
  };

  document.querySelector('#closeUsageGuidesButton').onclick = () => {
    closeUsageGuides();
  };

  document.querySelector('#reloadUsageGuidesButton').onclick = () => {
    loadUsageGuides();
  };

  document.querySelectorAll('[data-guide-close="true"]').forEach((element) => {
    element.onclick = () => {
      closeUsageGuides();
    };
  });

  document.querySelectorAll('[data-guide-id]').forEach((element) => {
    element.onclick = () => {
      const nextGuideId = element.getAttribute('data-guide-id');
      if (!nextGuideId || nextGuideId === state.selectedUsageGuideId) return;
      state.selectedUsageGuideId = nextGuideId;
      render();
    };
  });
}

function render() {
  if (renderFrame) return;
  renderFrame = window.requestAnimationFrame(() => {
    renderFrame = 0;
    commitRender();
  });
}

function openUsageGuides() {
  state.isUsageGuideOpen = true;
  render();

  if (!state.usageGuidesLoaded && !state.usageGuidesLoading) {
    loadUsageGuides();
  }
}

function closeUsageGuides() {
  if (!state.isUsageGuideOpen) return;
  state.isUsageGuideOpen = false;
  render();
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

async function startRefreshJob() {
  if (state.refreshing || state.refreshStatus.running) return;

  state.error = '';
  state.refreshing = true;
  render();

  try {
    state.refreshStatus = normalizeRefreshStatus(await invoke(COMMANDS.startRefresh));
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

async function loadUsageGuides() {
  if (state.usageGuidesLoading) return;

  state.usageGuidesLoading = true;
  state.usageGuidesError = '';
  render();

  try {
    state.usageGuides = normalizeUsageGuides(await invoke(COMMANDS.usageGuides));
    state.usageGuidesLoaded = true;
    state.selectedUsageGuideId = resolveSelectedUsageGuide(state.usageGuides, state.selectedUsageGuideId);
  } catch (error) {
    state.usageGuides = [];
    state.usageGuidesLoaded = false;
    state.selectedUsageGuideId = '';
    state.usageGuidesError = getErrorMessage(error);
  } finally {
    state.usageGuidesLoading = false;
    render();
  }
}

async function loadDataHealthSummary({ force = false } = {}) {
  if (state.dataHealthLoading) return;
  if (!force && isDataHealthCacheFresh()) return;

  state.dataHealthLoading = true;
  state.dataHealthError = '';
  render();

  try {
    state.dataHealth = await invoke(COMMANDS.dataHealthSummary);
    state.dataHealthFetchedAt = new Date().toISOString();
    state.lastUpdatedAt = new Date().toISOString();
  } catch (error) {
    state.dataHealthError = getErrorMessage(error);
  } finally {
    state.dataHealthLoading = false;
    render();
  }
}

async function loadDashboard() {
  state.loading = true;
  state.error = '';
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
      state.recentReports = normalizeRecentReports(bundleResult.recent_reports);
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
    if (!isDataHealthCacheFresh()) {
      void loadDataHealthSummary();
    }
  }
}

async function loadSelectedSnapshot() {
  state.loading = true;
  state.error = '';
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

async function exportDataHealthReport() {
  if (!state.dataHealth || state.dataHealthExporting) return;

  state.dataHealthExporting = true;
  state.dataHealthExportResult = null;
  render();

  try {
    const result = await invoke(COMMANDS.exportDataHealthReport);
    state.dataHealthExportResult = {
      kind: 'success',
      title: 'Data-health report exported',
      message: `Saved data-health report for ${result.report_date}.`,
      output_path: result.output_path,
      failed_items: Array.isArray(result.failed_items) ? result.failed_items : [],
    };
    if (result.output_path) {
      pushRecentReport('DATA_HEALTH_REPORT', result.report_date, result.output_path);
    }
  } catch (error) {
    state.dataHealthExportResult = {
      kind: 'error',
      title: 'Data-health export failed',
      message: getErrorMessage(error),
    };
  } finally {
    state.dataHealthExporting = false;
    render();
  }
}

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && state.isUsageGuideOpen) {
    closeUsageGuides();
  }
});

render();
loadDashboard();

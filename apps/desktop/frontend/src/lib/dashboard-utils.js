/**
 * Frontend-only dashboard helpers.
 *
 * Contract:
 * - Most exports return plain display strings, numbers, arrays, or tone tokens.
 * - `renderMarkdownContent()` returns an HTML fragment intended for controlled dashboard insertion.
 * - `formatDate()` / `formatDateTime()` return safe fallback text for invalid raw values.
 */
export function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

export function formatDate(value) {
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

export function formatDateTime(value) {
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

export function normalizeRefreshStatus(payload) {
  return {
    running: Boolean(payload?.running),
    status: String(payload?.status ?? 'idle'),
    progress_pct: Math.max(0, Math.min(100, Number(payload?.progress_pct ?? 0))),
    stage: String(payload?.stage ?? 'Idle'),
    current_stage: payload?.current_stage ? String(payload.current_stage) : null,
    start_stage: payload?.start_stage ? String(payload.start_stage) : 'full',
    retry_from_stage: payload?.retry_from_stage ? String(payload.retry_from_stage) : null,
    refresh_from: payload?.refresh_from ? String(payload.refresh_from) : null,
    refresh_to: payload?.refresh_to ? String(payload.refresh_to) : null,
    started_at: payload?.started_at ? String(payload.started_at) : null,
    finished_at: payload?.finished_at ? String(payload.finished_at) : null,
    error: payload?.error ? String(payload.error) : null,
  };
}

export function formatNumber(value, digits = 2) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return new Intl.NumberFormat(undefined, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(numeric);
}

export function getFiniteNumber(value) {
  if (value === null || value === undefined || value === '') return null;
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

export function formatInteger(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return new Intl.NumberFormat().format(numeric);
}

export function formatPercent(value, digits = 2) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return `${formatNumber(numeric * 100, digits)}%`;
}

export function formatDisplayPercent(value, digits = 1) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return `${formatNumber(numeric, digits)}%`;
}

export function formatDeltaPoints(value, digits = 1) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  const sign = numeric > 0 ? '+' : '';
  return `${sign}${formatNumber(numeric, digits)} pts`;
}

export function formatCurrency(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return '—';
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 0,
  }).format(numeric);
}

export function clampScore(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return 0;
  return Math.max(0, Math.min(100, numeric));
}

export function prettifyToken(value) {
  return String(value ?? '')
    .replaceAll('_', ' ')
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/\b\w/g, (match) => match.toUpperCase());
}

export function formatCanonicalAdjustment(value) {
  const adjustment = String(value ?? '').trim();
  if (!adjustment) return 'Unknown';
  return adjustment.length <= 5 ? adjustment.toUpperCase() : prettifyToken(adjustment);
}

export function regimeTone(label) {
  const normalized = String(label ?? '').toLowerCase();
  if (normalized.includes('risk_on')) return 'positive';
  if (normalized.includes('risk_off')) return 'negative';
  return 'neutral';
}

export function breadthTone(label) {
  const normalized = String(label ?? '').toLowerCase();
  if (normalized === 'improving' || normalized === 'strong') return 'positive';
  if (normalized === 'weakening' || normalized === 'weak') return 'negative';
  if (normalized === 'near_local_low' || normalized === 'near_local_high') return 'warning';
  if (normalized === 'unavailable') return 'outline';
  return 'neutral';
}

export function signalTone(label) {
  const normalized = String(label ?? '').toLowerCase();
  if (normalized.includes('strongbuy') || normalized.includes('buy')) return 'positive';
  if (normalized.includes('sell') || normalized.includes('reduce')) return 'negative';
  return 'neutral';
}

export function healthTone(status) {
  const normalized = String(status ?? '').toLowerCase();
  if (normalized === 'healthy') return 'positive';
  if (normalized === 'critical') return 'negative';
  return 'neutral';
}

export function dataHealthTone(summary) {
  if (!summary) return 'neutral';
  if (Number(summary.critical_symbols) > 0 || Number(summary.critical_macro_sources) > 0) return 'negative';
  if (Number(summary.review_symbols) > 0 || Number(summary.review_macro_sources) > 0) return 'neutral';
  return 'positive';
}

export function getFlaggedSymbols(summary) {
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

export function getFlaggedMacroSources(summary) {
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

export function formatDateRange(start, end) {
  if (!start && !end) return 'Date range unavailable';
  return `${formatDate(start)} → ${formatDate(end)}`;
}

export function normalizeAvailableDates(values) {
  if (!Array.isArray(values)) return [];

  return [...new Set(values.map((value) => String(value ?? '').trim()).filter(Boolean))]
    .sort((left, right) => right.localeCompare(left));
}

export function normalizeRecentReports(values, limit = Infinity) {
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
    .slice(0, limit);
}

export function normalizeUsageGuides(values) {
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

export function resolveSelectedUsageGuide(guides, currentGuideId) {
  if (!guides.length) return '';
  if (currentGuideId && guides.some((guide) => guide.id === currentGuideId)) return currentGuideId;
  return guides[0].id;
}

export function resolveSelectedReportDate(availableDates, currentDate) {
  if (!availableDates.length) return '';
  if (currentDate && availableDates.includes(currentDate)) return currentDate;
  return availableDates[0];
}

export function formatReportType(value) {
  const normalized = String(value ?? '').trim().toUpperCase();

  if (normalized === 'DAILY_REPORT') return 'Daily report';
  if (normalized === 'DAILY_REPORT_CN') return 'Daily report (CN)';
  if (normalized === 'DAILY_REPORT_HK') return 'Daily report (HK)';
  if (normalized === 'DATA_HEALTH_REPORT') return 'Data health';

  return prettifyToken(value);
}

export function getRecentReportScope(value) {
  const normalized = String(value ?? '').trim().toUpperCase();
  if (normalized === 'DAILY_REPORT_CN') return 'cn';
  if (normalized === 'DAILY_REPORT_HK') return 'hk';
  if (normalized === 'DAILY_REPORT') return 'global';
  return null;
}

export function parseDateValue(value) {
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

export function normalizeScope(value) {
  const normalized = String(value ?? 'global').trim().toLowerCase();
  if (normalized === 'cn' || normalized === 'hk') return normalized;
  return 'global';
}

export function formatScopeLabel(value) {
  const normalized = normalizeScope(value);
  if (normalized === 'cn') return 'CN';
  if (normalized === 'hk') return 'HK';
  return 'GLOBAL';
}

export function getDayDifference(earlier, later) {
  const earlierValue = parseDateValue(earlier);
  const laterValue = parseDateValue(later);

  if (!Number.isFinite(earlierValue) || !Number.isFinite(laterValue)) {
    return null;
  }

  return Math.round((laterValue - earlierValue) / 86400000);
}

export function formatFallbackState(value) {
  if (value === true) return 'Fallback ok';
  if (value === false) return 'Fallback down';
  return 'Fallback n/a';
}

export function getErrorMessage(error) {
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

/**
 * Returns a sanitized HTML fragment for the in-app guide viewer.
 */
export function renderMarkdownContent(value) {
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

export function trustTone(level) {
  const normalized = String(level || '').toLowerCase();
  if (normalized === 'trusted') return 'positive';
  if (normalized === 'degraded') return 'negative';
  return 'warning';
}

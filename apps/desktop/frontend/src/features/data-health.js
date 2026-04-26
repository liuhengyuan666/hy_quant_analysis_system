/**
 * Data-health slice.
 * Owns data-health cache checks, summary loading, export flow, rendering, and
 * data-health-specific event wiring while leaving dashboard-wide orchestration in `main.js`.
 */
export function createDataHealthSlice({
  state,
  render,
  invoke,
  commands,
  cacheMs,
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
}) {
  function isCacheFresh() {
    if (!state.dataHealth || !state.dataHealthFetchedAt) return false;
    const fetchedAt = new Date(state.dataHealthFetchedAt).getTime();
    if (!Number.isFinite(fetchedAt)) return false;
    return (Date.now() - fetchedAt) < cacheMs;
  }

  function renderPanel(summary) {
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

  async function loadSummary({ force = false } = {}) {
    if (state.dataHealthLoading) return;
    if (!force && isCacheFresh()) return;

    state.dataHealthLoading = true;
    state.dataHealthError = '';
    render();

    try {
      state.dataHealth = await invoke(commands.dataHealthSummary);
      state.dataHealthFetchedAt = new Date().toISOString();
      state.lastUpdatedAt = new Date().toISOString();
    } catch (error) {
      state.dataHealthError = getErrorMessage(error);
    } finally {
      state.dataHealthLoading = false;
      render();
    }
  }

  async function exportReport() {
    if (!state.dataHealth || state.dataHealthExporting) return;

    state.dataHealthExporting = true;
    state.dataHealthExportResult = null;
    render();

    try {
      const result = await invoke(commands.exportDataHealthReport);
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

  function bindEvents(root = document) {
    const exportButton = root.querySelector('#exportHealthButton');
    if (exportButton) {
      exportButton.onclick = () => {
        void exportReport();
      };
    }

    const refreshButton = root.querySelector('#refreshHealthButton');
    if (refreshButton) {
      refreshButton.onclick = () => {
        void loadSummary({ force: true });
      };
    }
  }

  return {
    bindEvents,
    exportReport,
    isCacheFresh,
    loadSummary,
    renderPanel,
  };
}

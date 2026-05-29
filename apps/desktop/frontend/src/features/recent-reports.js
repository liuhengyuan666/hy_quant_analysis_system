function copyTextFallback(text) {
  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.setAttribute('readonly', 'true');
  textarea.style.position = 'fixed';
  textarea.style.opacity = '0';
  textarea.style.pointerEvents = 'none';
  document.body.appendChild(textarea);
  textarea.select();
  textarea.setSelectionRange(0, textarea.value.length);
  const success = document.execCommand('copy');
  document.body.removeChild(textarea);
  if (!success) {
    throw new Error('Clipboard copy is unavailable in this runtime.');
  }
}

/**
 * Recent-reports slice.
 * Owns report-history rendering and lightweight report-management actions.
 */
export function createRecentReportsSlice({
  state,
  render,
  invoke,
  commands,
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
}) {
  function buildReportIndex(index) {
    return String(index);
  }

  function getReportItemByIndex(index) {
    const numericIndex = Number(index);
    if (!Number.isInteger(numericIndex) || numericIndex < 0) return null;
    return state.recentReports[numericIndex] || null;
  }

  function isCurrentSnapshot(item, scope) {
    if (!scope) return false;
    return normalizeScope(state.selectedScope) === scope && getActiveReportDate() === item.report_date;
  }

  function clearActionResult() {
    state.recentReportActionResult = null;
  }

  async function openArtifact(item) {
    if (!item?.artifact_path) return;

    try {
      await invoke(commands.openReportArtifact, { artifactPath: item.artifact_path });
      state.recentReportActionResult = {
        kind: 'success',
        title: 'Artifact opened',
        message: `Opened ${formatReportType(item.report_type)} for ${item.report_date}.`,
        output_path: item.artifact_path,
      };
    } catch (error) {
      state.recentReportActionResult = {
        kind: 'error',
        title: 'Open failed',
        message: getErrorMessage(error),
      };
    }
    render();
  }

  async function copyArtifactPath(item) {
    if (!item?.artifact_path) return;

    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(item.artifact_path);
      } else {
        copyTextFallback(item.artifact_path);
      }
      state.recentReportActionResult = {
        kind: 'success',
        title: 'Path copied',
        message: `Copied artifact path for ${formatReportType(item.report_type)} (${item.report_date}).`,
        output_path: item.artifact_path,
      };
    } catch (error) {
      state.recentReportActionResult = {
        kind: 'error',
        title: 'Copy failed',
        message: getErrorMessage(error),
      };
    }
    render();
  }

  async function openSnapshot(item) {
    const nextScope = getRecentReportScope(item?.report_type);
    if (!item?.report_date || !nextScope) return;
    if (state.loading || state.refreshing || state.refreshStatus.running) return;

    const currentScope = normalizeScope(state.selectedScope);
    const currentDate = getActiveReportDate();
    if (currentScope === nextScope && currentDate === item.report_date) {
      state.recentReportActionResult = {
        kind: 'success',
        title: 'Snapshot already open',
        message: `Already viewing ${formatReportType(item.report_type)} for ${item.report_date}.`,
      };
      render();
      return;
    }

    clearActionResult();
    state.selectedReportDate = item.report_date;
    state.exportResult = null;

    if (currentScope === nextScope) {
      await loadSelectedSnapshot();
      return;
    }

    state.selectedScope = nextScope;
    state.snapshot = null;
    await loadDashboard();
  }

  function renderRecentReportsPanel({ limit = 3, showViewAll = true } = {}) {
    const totalReports = state.recentReports.length;
    const displayReports = limit ? state.recentReports.slice(0, limit) : state.recentReports;
    const hasMore = totalReports > limit;

    return `
      <article class="panel panel--soft">
        <div class="panel__header">
          <div>
            <p class="eyebrow">Research results</p>
            <h2>Recent reports</h2>
            <p class="panel__lede">Recent exported artifacts can reopen matching analysis snapshots, open the generated artifact directly, or provide quick artifact-path access.</p>
          </div>
          <div class="panel__header-actions">
            <span class="panel__meta">Latest ${escapeHtml(formatInteger(totalReports))} exports</span>
            ${hasMore && showViewAll ? `
              <button
                class="button button--secondary button--compact"
                data-report-view-all="true"
              >
                View All (${escapeHtml(formatInteger(totalReports))})
              </button>
            ` : ''}
          </div>
        </div>

        ${renderNotice(state.recentReportActionResult, 'notice--inline')}

        ${displayReports.length
          ? `
            <div class="report-history" aria-label="Recent report history">
              ${displayReports
                .map((item, index) => {
                  const reportScope = getRecentReportScope(item.report_type);
                  const canOpenSnapshot = Boolean(reportScope);
                  const currentView = canOpenSnapshot && isCurrentSnapshot(item, reportScope);
                  return `
                    <article class="report-history__item">
                      <div class="report-history__row">
                        <span class="pill pill--outline">${escapeHtml(formatReportType(item.report_type))}</span>
                        <span class="report-history__date">${escapeHtml(formatDate(item.report_date))}</span>
                      </div>
                      <div class="report-history__meta-row">
                        <span class="panel__meta">${canOpenSnapshot ? `Analysis scope · ${escapeHtml(formatScopeLabel(reportScope))}` : 'Artifact only · no snapshot jump'}</span>
                        <span class="panel__meta">${currentView ? 'Current dashboard view' : canOpenSnapshot ? 'Snapshot jump available' : 'Artifact actions available'}</span>
                      </div>
                      <p class="report-history__path"><code>${escapeHtml(item.artifact_path)}</code></p>
                      <div class="report-history__actions">
                        ${canOpenSnapshot ? `
                          <button
                            class="button button--secondary button--compact"
                            data-report-open-snapshot="${escapeHtml(buildReportIndex(index))}"
                            ${state.loading || state.refreshing || state.refreshStatus.running || currentView ? 'disabled' : ''}
                          >
                            ${currentView ? 'Current view' : 'Open snapshot'}
                          </button>
                        ` : ''}
                        <button
                          class="button button--secondary button--compact"
                          data-report-open-artifact="${escapeHtml(buildReportIndex(index))}"
                        >
                          Open artifact
                        </button>
                        <button
                          class="button button--secondary button--compact"
                          data-report-copy-path="${escapeHtml(buildReportIndex(index))}"
                        >
                          Copy path
                        </button>
                      </div>
                    </article>
                  `;
                })
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

  function bindEvents(root = document) {
    root.querySelectorAll('[data-report-open-snapshot]').forEach((element) => {
      element.onclick = () => {
        const item = getReportItemByIndex(element.getAttribute('data-report-open-snapshot'));
        if (!item) return;
        void openSnapshot(item);
      };
    });

    root.querySelectorAll('[data-report-open-artifact]').forEach((element) => {
      element.onclick = () => {
        const item = getReportItemByIndex(element.getAttribute('data-report-open-artifact'));
        if (!item) return;
        void openArtifact(item);
      };
    });

    root.querySelectorAll('[data-report-copy-path]').forEach((element) => {
      element.onclick = () => {
        const item = getReportItemByIndex(element.getAttribute('data-report-copy-path'));
        if (!item) return;
        void copyArtifactPath(item);
      };
    });

    // View All button - open modal with all reports
    root.querySelectorAll('[data-report-view-all]').forEach((element) => {
      element.onclick = () => {
        openAllReportsModal();
      };
    });
  }

  function openAllReportsModal() {
    // Create modal overlay
    const overlay = document.createElement('div');
    overlay.className = 'reports-modal-overlay';
    overlay.onclick = (e) => {
      if (e.target === overlay) {
        document.body.removeChild(overlay);
      }
    };

    // Create modal content
    const modal = document.createElement('div');
    modal.className = 'reports-modal';

    const header = document.createElement('div');
    header.className = 'reports-modal__header';
    header.innerHTML = `
      <h2>All Recent Reports</h2>
      <button class="reports-modal__close" aria-label="Close">×</button>
    `;

    const content = document.createElement('div');
    content.className = 'reports-modal__content';
    content.innerHTML = renderRecentReportsPanel({ limit: null, showViewAll: false });

    modal.appendChild(header);
    modal.appendChild(content);
    overlay.appendChild(modal);
    document.body.appendChild(overlay);

    // Close button handler
    header.querySelector('.reports-modal__close').onclick = () => {
      document.body.removeChild(overlay);
    };

    // Bind events for the modal content
    bindEvents(content);
  }

  return {
    bindEvents,
    renderRecentReportsPanel,
  };
}

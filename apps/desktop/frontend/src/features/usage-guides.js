/**
 * Usage-guides slice.
 * Owns guide-specific loading, rendering, open/close behavior, and guide event wiring.
 * Leaves dashboard-wide state orchestration and main render scheduling in `main.js`.
 */
export function createUsageGuidesSlice({
  state,
  render,
  invoke,
  commands,
  escapeHtml,
  formatInteger,
  normalizeUsageGuides,
  resolveSelectedUsageGuide,
  renderMarkdownContent,
  getErrorMessage,
}) {
  function getSelectedUsageGuide() {
    return state.usageGuides.find((guide) => guide.id === state.selectedUsageGuideId) || state.usageGuides[0] || null;
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

  function openUsageGuides() {
    state.isUsageGuideOpen = true;
    render();

    if (!state.usageGuidesLoaded && !state.usageGuidesLoading) {
      void loadUsageGuides();
    }
  }

  function closeUsageGuides() {
    if (!state.isUsageGuideOpen) return;
    state.isUsageGuideOpen = false;
    render();
  }

  async function loadUsageGuides() {
    if (state.usageGuidesLoading) return;

    state.usageGuidesLoading = true;
    state.usageGuidesError = '';
    render();

    try {
      state.usageGuides = normalizeUsageGuides(await invoke(commands.usageGuides));
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

  function bindUsageGuideEvents(root = document) {
    const openButton = root.querySelector('#openUsageGuidesButton');
    if (openButton) {
      openButton.onclick = () => {
        openUsageGuides();
      };
    }

    const closeButton = root.querySelector('#closeUsageGuidesButton');
    if (closeButton) {
      closeButton.onclick = () => {
        closeUsageGuides();
      };
    }

    const reloadButton = root.querySelector('#reloadUsageGuidesButton');
    if (reloadButton) {
      reloadButton.onclick = () => {
        void loadUsageGuides();
      };
    }

    root.querySelectorAll('[data-guide-close="true"]').forEach((element) => {
      element.onclick = () => {
        closeUsageGuides();
      };
    });

    root.querySelectorAll('[data-guide-id]').forEach((element) => {
      element.onclick = () => {
        const nextGuideId = element.getAttribute('data-guide-id');
        if (!nextGuideId || nextGuideId === state.selectedUsageGuideId) return;
        state.selectedUsageGuideId = nextGuideId;
        render();
      };
    });
  }

  return {
    bindUsageGuideEvents,
    closeUsageGuides,
    loadUsageGuides,
    openUsageGuides,
    renderUsageEntry,
    renderUsageGuidesViewer,
  };
}

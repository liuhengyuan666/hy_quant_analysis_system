import {
  breadthTone,
  clampScore,
  escapeHtml,
  formatDate,
  formatDeltaPoints,
  formatDisplayPercent,
  formatInteger,
  formatNumber,
  formatPercent,
  getFiniteNumber,
  prettifyToken,
} from '../lib/dashboard-utils.js';

/**
 * Renderers for the environment explanation layer and watchlist breadth proxy.
 * These panels intentionally stay together because they present the same market-participation story
 * from two complementary angles: environment decomposition vs raw breadth proxy.
 */
export function createEnvironmentBreadthRenderers({ renderMetricCard }) {
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

  return {
    renderEnvironmentPanel,
    renderWatchlistBreadthPanel,
  };
}

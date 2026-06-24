<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatNumber, prettifyToken, signalTone } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const snapshot = computed(() => dashboardStore.snapshot);
const topSignals = computed(() => snapshot.value?.top_signals || []);
const bullishSignals = computed(() => snapshot.value?.bullish_signals || []);
const defensiveSignals = computed(() => snapshot.value?.defensive_signals || []);
const symbolNames = computed(() => snapshot.value?.symbol_names || {});

// Merge top + bullish, deduplicate by symbol, keep higher score, show all
const mergedSignals = computed(() => {
  const map = new Map();
  [...topSignals.value, ...bullishSignals.value].forEach(item => {
    const existing = map.get(item.symbol);
    if (!existing || (item.final_score || 0) > (existing.final_score || 0)) {
      map.set(item.symbol, item);
    }
  });
  return Array.from(map.values());
});

const hasSignals = computed(() => mergedSignals.value.length > 0 || defensiveSignals.value.length > 0);

const signalDistribution = computed(() => {
  const counts = { StrongBuy: 0, Buy: 0, Other: 0 };
  mergedSignals.value.forEach(item => {
    const label = item.signal_label || '';
    if (label === 'StrongBuy') counts.StrongBuy++;
    else if (label === 'Buy') counts.Buy++;
    else counts.Other++;
  });
  return counts;
});

const signalBasis = computed(() => {
  if (!snapshot.value || mergedSignals.value.length === 0) return null;
  const signal = mergedSignals.value[0];

  const analysisScope = String(signal.analysis_scope || 'GLOBAL').toUpperCase();
  const regimeBasisScope = String(signal.regime_basis_scope || 'GLOBAL').toUpperCase();
  const snapshotScope = String(snapshot.value.scope || 'GLOBAL').toUpperCase();

  return {
    analysisScope,
    regimeBasisScope,
    snapshotScope,
    mismatched: regimeBasisScope !== snapshotScope,
  };
});

function buildTooltip(item) {
  const r = item.reason || {};
  const sc = r.strategy_contribution ?? 0;
  const ac = r.alignment_contribution ?? 0;
  const rc = r.regime?.contribution ?? 0;
  const rot = r.rotation?.contribution ?? 0;
  const total = sc + ac + rc + rot;
  const best = r.best_strategy ? prettifyToken(r.best_strategy) : '-';
  const aligned = (r.aligned_strategies || []).map(prettifyToken).join(', ') || 'None';
  const rank = r.rotation?.rank ? `#${r.rotation.rank}` : '-';
  return `Strategy: ${best}  |  Score: ${formatNumber(r.strategy_score, 2)}  |  Contrib: +${formatNumber(sc, 2)}
Aligned: ${aligned}  |  Alignment: ${r.alignment ?? '-'}  |  Contrib: +${formatNumber(ac, 2)}
Regime Trend: ${formatNumber(r.regime?.trend_score, 2)}  |  Risk: ${formatNumber(r.regime?.risk_score, 2)}  |  Contrib: +${formatNumber(rc, 2)}
Rotation Rank: ${rank}  |  Momentum: ${formatNumber(r.rotation?.momentum_score, 2)}  |  Contrib: +${formatNumber(rot, 2)}
${formatNumber(sc, 2)} (Strategy) + ${formatNumber(ac, 2)} (Alignment) + ${formatNumber(rc, 2)} (Regime) + ${formatNumber(rot, 2)} (Rotation) = ${formatNumber(total, 2)}`;
}
</script>

<template>
  <article class="panel">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('signals.eyebrow') }}</p>
        <h2>{{ t('signals.title') }}</h2>
        <p v-if="hasSignals" class="panel__lede">
          {{ t('signals.lede') }}
        </p>
      </div>
      <div v-if="hasSignals" class="panel__actions">
        <span class="panel__meta">{{ t('signals.groupedView', { date: snapshot?.report_date }) }}</span>
      </div>
    </div>

    <div v-if="signalBasis" class="panel__meta-row">
      <span class="panel__meta">{{ t('signals.dashboardScope', { scope: signalBasis.snapshotScope }) }}</span>
      <span class="panel__meta">{{ t('signals.analysisScope', { scope: signalBasis.analysisScope }) }}</span>
      <span class="panel__meta">{{ t('signals.regimeBasis', { scope: signalBasis.regimeBasisScope }) }}</span>
    </div>

    <section v-if="signalBasis?.mismatched" class="staleness-banner staleness-banner--warning" aria-label="Signal provenance notice">
      <strong>{{ t('signals.basisDiffers') }}</strong>
      <p>{{ t('signals.showingSignals', { scope: signalBasis.snapshotScope, analysis: signalBasis.analysisScope, regime: signalBasis.regimeBasisScope }) }}</p>
    </section>

    <div v-if="!hasSignals" class="empty-state">
      <p>{{ t('signals.noSignals') }}</p>
    </div>

    <template v-else>
      <!-- Signal Distribution Summary -->
      <div class="signal-distribution">
        <div class="signal-distribution__item">
          <span class="signal-distribution__count signal-distribution__count--strong">{{ signalDistribution.StrongBuy }}</span>
          <span class="signal-distribution__label">StrongBuy</span>
        </div>
        <div class="signal-distribution__item">
          <span class="signal-distribution__count signal-distribution__count--buy">{{ signalDistribution.Buy }}</span>
          <span class="signal-distribution__label">Buy</span>
        </div>
        <div v-if="signalDistribution.Other > 0" class="signal-distribution__item">
          <span class="signal-distribution__count signal-distribution__count--watch">{{ signalDistribution.Other }}</span>
          <span class="signal-distribution__label">Other</span>
        </div>
      </div>

      <!-- Two-column layout: merged bullish (left) + defensive (right) -->
      <div class="signal-groups-grid">
        <!-- Left: merged top + bullish (deduplicated) -->
        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">{{ t('signals.bullishOpportunities') }}</p>
            <span class="panel__meta">{{ t('signals.strongBuyBuy') }}</span>
          </div>
          <div v-if="mergedSignals.length" class="signal-list">
            <div
              v-for="(item, index) in mergedSignals"
              :key="`merged-${index}`"
              class="signal-card"
              :class="`signal-card--${signalTone(item.signal_label)}`"
            >
              <div class="signal-card__header">
                <div>
                  <strong class="signal-card__symbol">{{ item.symbol }}</strong>
                  <span v-if="symbolNames[item.symbol]" class="signal-card__name">{{ symbolNames[item.symbol] }}</span>
                  <p class="signal-card__score">{{ t('signals.score', { score: formatNumber(item.final_score, 2) }) }}</p>
                </div>
                <span class="pill" :class="`pill--${signalTone(item.signal_label)}`">
                  {{ prettifyToken(item.signal_label) }}
                </span>
              </div>
              <p v-if="item.reason" class="signal-card__reason">{{ item.reason?.summary || '' }}</p>

              <!-- CSS Hover Tooltip -->
              <div class="signal-tooltip">
                <div class="tooltip-title">{{ item.symbol }} <span class="tooltip-symbol">{{ symbolNames[item.symbol] || '' }}</span></div>
                <div class="tooltip-divider"></div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Final Score</span>
                  <span class="tooltip-value">{{ formatNumber(item.final_score, 2) }}</span>
                </div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Strategy</span>
                  <span class="tooltip-value">+{{ formatNumber(item.reason?.strategy_contribution, 2) }}</span>
                </div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Alignment</span>
                  <span class="tooltip-value">+{{ formatNumber(item.reason?.alignment_contribution, 2) }}</span>
                </div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Regime</span>
                  <span class="tooltip-value">+{{ formatNumber(item.reason?.regime?.contribution, 2) }}</span>
                </div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Rotation</span>
                  <span class="tooltip-value">+{{ formatNumber(item.reason?.rotation?.contribution, 2) }}</span>
                </div>
                <div class="tooltip-divider"></div>
                <div class="tooltip-formula">
                  {{ formatNumber(item.reason?.strategy_contribution, 2) }} + {{ formatNumber(item.reason?.alignment_contribution, 2) }} + {{ formatNumber(item.reason?.regime?.contribution, 2) }} + {{ formatNumber(item.reason?.rotation?.contribution, 2) }} = {{ formatNumber(item.final_score, 2) }}
                </div>
              </div>
            </div>
          </div>
          <div v-else class="empty-state empty-state--compact">
            <p>{{ t('signals.noBullish') }}</p>
          </div>
        </section>

        <!-- Right: defensive signals -->
        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">{{ t('signals.defensiveSell') }}</p>
            <span class="panel__meta">{{ t('signals.watchHoldReduceSell') }}</span>
          </div>
          <div v-if="defensiveSignals.length" class="signal-list">
            <div
              v-for="(item, index) in defensiveSignals"
              :key="`defensive-${index}`"
              class="signal-card signal-card--defensive"
            >
              <div class="signal-card__header">
                <div>
                  <strong class="signal-card__symbol">{{ item.symbol }}</strong>
                  <span v-if="symbolNames[item.symbol]" class="signal-card__name">{{ symbolNames[item.symbol] }}</span>
                  <p class="signal-card__score">{{ t('signals.score', { score: formatNumber(item.final_score, 2) }) }}</p>
                </div>
                <span class="pill" :class="`pill--${signalTone(item.signal_label)}`">
                  {{ prettifyToken(item.signal_label) }}
                </span>
              </div>
              <p v-if="item.reason" class="signal-card__reason">{{ item.reason?.summary || '' }}</p>

              <!-- CSS Hover Tooltip -->
              <div class="signal-tooltip">
                <div class="tooltip-title">{{ item.symbol }} <span class="tooltip-symbol">{{ symbolNames[item.symbol] || '' }}</span></div>
                <div class="tooltip-divider"></div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Final Score</span>
                  <span class="tooltip-value">{{ formatNumber(item.final_score, 2) }}</span>
                </div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Strategy</span>
                  <span class="tooltip-value">+{{ formatNumber(item.reason?.strategy_contribution, 2) }}</span>
                </div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Alignment</span>
                  <span class="tooltip-value">+{{ formatNumber(item.reason?.alignment_contribution, 2) }}</span>
                </div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Regime</span>
                  <span class="tooltip-value">+{{ formatNumber(item.reason?.regime?.contribution, 2) }}</span>
                </div>
                <div class="tooltip-row">
                  <span class="tooltip-key">Rotation</span>
                  <span class="tooltip-value">+{{ formatNumber(item.reason?.rotation?.contribution, 2) }}</span>
                </div>
                <div class="tooltip-divider"></div>
                <div class="tooltip-formula">
                  {{ formatNumber(item.reason?.strategy_contribution, 2) }} + {{ formatNumber(item.reason?.alignment_contribution, 2) }} + {{ formatNumber(item.reason?.regime?.contribution, 2) }} + {{ formatNumber(item.reason?.rotation?.contribution, 2) }} = {{ formatNumber(item.final_score, 2) }}
                </div>
              </div>
            </div>
          </div>
          <div v-else class="empty-state empty-state--compact">
            <p>{{ t('signals.noDefensive') }}</p>
          </div>
        </section>
      </div>
    </template>
  </article>
</template>

<style scoped>
.panel {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--panel-padding);
}

.panel__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--space-4);
}

.panel__actions {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}

.panel__lede {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  margin-top: var(--space-1);
}

.panel__meta {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

.panel__meta-row {
  display: flex;
  gap: var(--space-4);
  flex-wrap: wrap;
  margin-bottom: var(--space-4);
}

.panel__subheader {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-3);
}

.panel__section-title {
  font-weight: 600;
  color: var(--text-primary);
}

.signal-distribution {
  display: flex;
  gap: var(--space-4);
  margin-bottom: var(--space-4);
  padding: var(--space-3);
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
}

.signal-distribution__item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-1);
  min-width: 4rem;
}

.signal-distribution__count {
  font-family: var(--font-mono);
  font-size: 1.5rem;
  font-weight: 700;
  line-height: 1;
}

.signal-distribution__count--strong {
  color: var(--tone-positive);
}

.signal-distribution__count--buy {
  color: var(--color-accent);
}

.signal-distribution__count--watch {
  color: var(--color-warning);
}

.signal-distribution__label {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.signal-focus-section {
  margin-bottom: var(--space-5);
}

.signal-groups-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-5);
}

@media (max-width: 720px) {
  .signal-groups-grid {
    grid-template-columns: 1fr;
  }
}

.signal-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.signal-card {
  position: relative;
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--space-3);
  text-align: left;
  cursor: default;
  transition: border-color 0.2s ease;
}

.signal-card:hover {
  border-color: var(--color-accent-border);
}

.signal-card__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.signal-card__symbol {
  display: inline;
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
}

.signal-card__name {
  display: inline;
  margin-left: var(--space-2);
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

.signal-card__score {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  margin: var(--space-1) 0 0;
}

.signal-card__reason {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  margin: var(--space-2) 0 0;
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: var(--space-1) var(--space-3);
  border-radius: var(--space-3);
  font-size: var(--font-size-label);
  font-weight: 500;
}

.pill--positive {
  background: var(--tone-positive-bg);
  color: var(--tone-positive);
}

.pill--negative {
  background: var(--tone-negative-bg);
  color: var(--tone-negative);
}

.pill--neutral {
  background: var(--tone-neutral-bg);
  color: var(--tone-neutral);
}

.pill--warning {
  background: var(--color-warning-soft);
  color: var(--color-warning);
}

.staleness-banner {
  padding: var(--space-3);
  border-radius: var(--panel-radius);
  margin-bottom: var(--space-4);
}

.staleness-banner--warning {
  background: var(--color-warning-soft);
  border: 1px solid var(--color-warning);
}

.staleness-banner strong {
  display: block;
  margin-bottom: var(--space-1);
  color: var(--color-warning);
}

.staleness-banner p {
  margin: 0;
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
}

.signal-card--positive {
  border-color: rgba(118, 212, 159, 0.18);
}

.signal-card--negative {
  border-color: rgba(240, 141, 126, 0.18);
}

.signal-card--defensive {
  background: linear-gradient(180deg, rgba(245, 176, 65, 0.06), rgba(245, 176, 65, 0.02));
}

/* CSS Hover Tooltip */
.signal-tooltip {
  visibility: hidden;
  opacity: 0;
  position: absolute;
  z-index: 100;
  bottom: calc(100% + 0.5rem);
  left: 50%;
  transform: translateX(-50%);
  transition: opacity 0.15s ease-in-out;
  background: var(--color-surface-strong);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.75rem 1rem;
  width: 18rem;
  box-shadow: var(--shadow-strong);
  pointer-events: none;
  font-family: var(--font-mono);
}

.signal-card:hover .signal-tooltip {
  visibility: visible;
  opacity: 1;
}

.tooltip-title {
  color: var(--text-primary);
  font-size: 0.95rem;
  font-weight: 600;
  margin-bottom: 0.2rem;
}

.tooltip-symbol {
  color: var(--text-secondary);
  font-weight: 400;
  font-size: 0.8rem;
}

.tooltip-divider {
  height: 1px;
  background: var(--color-border);
  margin: 0.5rem 0;
}

.tooltip-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.85rem;
  line-height: 1.7;
}

.tooltip-key {
  color: var(--text-secondary);
  font-size: 0.8rem;
}

.tooltip-value {
  font-family: var(--font-mono);
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}

.tooltip-formula {
  font-size: 0.75rem;
  color: var(--text-secondary);
  text-align: center;
  margin-top: 0.25rem;
  opacity: 0.8;
}

.eyebrow {
  font-size: var(--font-size-label);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}

h2 {
  margin: var(--space-1) 0 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.empty-state {
  padding: var(--space-5);
  text-align: center;
  color: var(--text-secondary);
}

.empty-state--compact {
  padding: var(--space-3);
}
</style>

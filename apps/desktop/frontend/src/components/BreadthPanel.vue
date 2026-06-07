<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  breadthTone,
  clampScore,
  formatDeltaPoints,
  formatDisplayPercent,
  formatInteger,
  formatPercent,
  getFiniteNumber,
  prettifyToken,
} from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const snapshot = computed(() => dashboardStore.snapshot);
const loading = computed(() => dashboardStore.loading);
const error = computed(() => dashboardStore.error);

const breadth = computed(() => snapshot.value?.watchlist_breadth);
const markets = computed(() => breadth.value?.markets || []);
const hasMarkets = computed(() => markets.value.length > 0);
const methodologyNote = computed(
  () => breadth.value?.methodology_note || t('participation.methodologyNote'),
);

function getMarketStatus(market) {
  const eligibleCount = Number(market?.eligible_count ?? 0);
  const unavailable =
    String(market?.status_label ?? '').toLowerCase() === 'unavailable' || eligibleCount === 0;
  return { unavailable, eligibleCount };
}

function getBreadthValue(market, unavailable) {
  return unavailable ? t('utils.unavailable') : formatDisplayPercent(market?.breadth_pct, 1);
}

function getSummaryText(market, unavailable, eligibleCount) {
  if (unavailable) {
    return t('participation.noEligible');
  }
  const aboveCount = Number(market?.above_count ?? 0);
  return `${formatInteger(aboveCount)} / ${formatInteger(eligibleCount)} above MA30`;
}

function getRangePosition(market) {
  return getFiniteNumber(market?.range_position_60d);
}
</script>

<template>
  <article class="panel panel--soft" v-cloak>
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('participation.eyebrow') }}</p>
        <h2>{{ t('participation.title') }}</h2>
        <p class="panel__lede">
          {{ t('participation.lede') }}
        </p>
      </div>
      <span class="pill pill--outline">{{ t('participation.proxyOnly') }}</span>
    </div>

    <div v-if="loading" class="empty-state">
      <p>{{ t('participation.loadingBreadth') }}</p>
    </div>

    <div v-else-if="error" class="empty-state">
      <p>{{ error }}</p>
    </div>

    <div v-else-if="!hasMarkets" class="empty-state">
      <p>{{ t('participation.unavailable') }}</p>
    </div>

    <template v-else>
      <div class="breadth-market-grid">
        <section
          v-for="market in markets"
          :key="market.market || market.universe_label"
          class="breadth-market"
          :class="{ 'breadth-market--unavailable': getMarketStatus(market).unavailable }"
        >
          <div class="panel__subheader">
            <p class="panel__section-title">
              {{ market?.universe_label || `${market?.market || 'Tracked'} universe` }}
            </p>
            <span
              class="pill"
              :class="`pill--${breadthTone(market?.status_label)}`"
            >
              {{ prettifyToken(market?.status_label || 'Unavailable') }}
            </span>
          </div>

          <div class="breadth-market__headline">
            <strong class="breadth-market__value">
              {{ getBreadthValue(market, getMarketStatus(market).unavailable) }}
            </strong>
            <span class="breadth-market__summary">
              {{ getSummaryText(market, getMarketStatus(market).unavailable, getMarketStatus(market).eligibleCount) }}
            </span>
          </div>

          <div class="breadth-market__metrics">
            <article class="breadth-market__metric">
              <span class="breadth-market__metric-label">{{ t('participation.sma5') }}</span>
              <strong class="breadth-market__metric-value">
                {{ formatDisplayPercent(market?.breadth_pct_sma5, 1) }}
              </strong>
            </article>
            <article class="breadth-market__metric">
              <span class="breadth-market__metric-label">{{ t('participation.delta5d') }}</span>
              <strong class="breadth-market__metric-value">
                {{ formatDeltaPoints(market?.breadth_5d_delta, 1) }}
              </strong>
            </article>
            <article class="breadth-market__metric">
              <span class="breadth-market__metric-label">{{ t('participation.range60d') }}</span>
              <strong class="breadth-market__metric-value">
                {{ getRangePosition(market) === null ? 'N/A' : formatPercent(getRangePosition(market), 0) }}
              </strong>
            </article>
          </div>

          <div class="score-row breadth-market__range-row">
            <div class="score-row__meta">
              <span>{{ t('participation.currentBreadth') }}</span>
              <strong>
                {{ getMarketStatus(market).unavailable ? 'N/A' : formatDisplayPercent(market?.breadth_pct, 1) }}
              </strong>
            </div>
            <div class="score-bar" aria-hidden="true">
              <span
                class="score-bar__fill"
                :style="{ width: `${getRangePosition(market) === null ? 0 : clampScore(getRangePosition(market) * 100)}%` }"
              ></span>
            </div>
          </div>
        </section>
      </div>

      <p class="breadth-panel__note">{{ methodologyNote }}</p>
    </template>
  </article>
</template>

<style scoped>
/* Component-specific styles - consume global CSS variables via bridge */
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

.panel__lede {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  margin-top: var(--space-1);
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

.breadth-market-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: var(--space-4);
}

.breadth-market {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--space-4);
}

.breadth-market--unavailable {
  opacity: 0.6;
}

.breadth-market__headline {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}

.breadth-market__value {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--text-primary);
}

.breadth-market__summary {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
}

.breadth-market__metrics {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}

.breadth-market__metric {
  text-align: center;
}

.breadth-market__metric-label {
  display: block;
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  margin-bottom: var(--space-1);
}

.breadth-market__metric-value {
  display: block;
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
}

.breadth-market__range-row {
  margin-top: var(--space-3);
}

.score-row {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.score-row__meta {
  display: flex;
  justify-content: space-between;
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
}

.score-bar {
  height: 6px;
  background: var(--score-bar-bg);
  border-radius: var(--space-1);
  overflow: hidden;
}

.score-bar__fill {
  display: block;
  height: 100%;
  background: var(--accent-primary);
  border-radius: var(--space-1);
  transition: width 0.3s ease;
}

.breadth-panel__note {
  margin-top: var(--space-4);
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  font-style: italic;
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: var(--space-1) var(--space-3);
  border-radius: var(--space-3);
  font-size: var(--font-size-label);
  font-weight: 500;
}

.pill--outline {
  border: 1px solid var(--pill-outline-border);
  color: var(--text-secondary);
  background: transparent;
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

.empty-state {
  padding: var(--space-5);
  text-align: center;
  color: var(--text-secondary);
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
</style>
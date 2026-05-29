<script setup>
import { computed } from 'vue';
import {
  clampScore,
  formatDate,
  formatInteger,
  formatNumber,
  prettifyToken,
  regimeTone,
} from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const snapshot = computed(() => dashboardStore.snapshot);

const scores = computed(() => [
  ['Trend score', snapshot.value?.trend_score],
  ['Liquidity score', snapshot.value?.liquidity_score],
  ['Risk score', snapshot.value?.risk_score],
]);

const freshness = computed(() => {
  if (!snapshot.value?.report_date) return null;
  const asOfDate = snapshot.value.regime_as_of_date || snapshot.value.report_date;
  const lagDays = snapshot.value.regime_stale_days ?? 0;
  return {
    stale: lagDays > 0,
    tone: lagDays > 2 ? 'negative' : lagDays > 0 ? 'warning' : 'positive',
    asOfDate,
    lagDays,
  };
});

const latestAvailableDate = computed(() => snapshot.value?.latest_available_date || '');

const freshnessMessage = computed(() => {
  if (!freshness.value?.stale) return '';
  if (freshness.value.lagDays > 0) {
    return `Macro regime inputs lag the selected report by ${formatInteger(freshness.value.lagDays)} day${freshness.value.lagDays === 1 ? '' : 's'}. Posture is shown as of ${formatDate(freshness.value.asOfDate)} while rotation and signal data reflect ${formatDate(snapshot.value?.report_date)}.`;
  }
  return `Macro regime inputs were last updated on ${formatDate(freshness.value.asOfDate)} while this dashboard view reflects ${formatDate(snapshot.value?.report_date)}.`;
});
</script>

<template>
  <article class="panel panel--accent">
    <div class="panel__header">
      <div>
        <p class="eyebrow">Market regime</p>
        <h2>{{ snapshot ? prettifyToken(snapshot.regime_label) : 'Waiting for snapshot' }}</h2>
      </div>
      <div v-if="snapshot" class="panel__actions">
        <span
          class="pill"
          :class="`pill--${snapshot.report_date === latestAvailableDate ? 'positive' : regimeTone(snapshot.regime_label)}`"
        >
          Selected analysis · {{ formatDate(snapshot.report_date) }}
        </span>
        <span v-if="latestAvailableDate" class="pill pill--outline">
          Latest available · {{ formatDate(latestAvailableDate) }}
        </span>
        <span class="pill" :class="`pill--${freshness?.stale ? 'warning' : 'outline'}`">
          Regime as-of · {{ formatDate(freshness?.asOfDate || snapshot.report_date) }}
        </span>
      </div>
    </div>

    <p class="panel__lede">Latest inferred posture across macro trend, liquidity, and risk inputs.</p>

    <section v-if="freshness?.stale" class="staleness-banner staleness-banner--warning" aria-label="Regime staleness notice">
      <strong>Macro regime is lagging the selected report</strong>
      <p>{{ freshnessMessage }}</p>
    </section>

    <div v-if="snapshot" class="score-stack">
      <div v-for="[label, value] in scores" :key="label" class="score-row">
        <div class="score-row__meta">
          <span>{{ label }}</span>
          <strong>{{ formatNumber(value, 1) }}</strong>
        </div>
        <div class="score-bar">
          <span class="score-bar__fill" :style="{ width: `${clampScore(value)}%` }"></span>
        </div>
      </div>
    </div>

    <div v-else class="empty-state">
      <p>Run the analysis pipeline to populate the dashboard snapshot.</p>
    </div>
  </article>
</template>

<style scoped>
.panel {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--panel-padding);
}

.panel--accent {
  border-color: var(--color-accent-border);
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
  flex-wrap: wrap;
}

.panel__lede {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  margin-bottom: var(--space-4);
}

.score-stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
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

.pill--warning {
  background: var(--color-warning-soft);
  color: var(--color-warning);
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
</style>

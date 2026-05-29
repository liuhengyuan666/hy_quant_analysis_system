<script setup>
import { computed } from 'vue';
import {
  breadthTone,
  clampScore,
  formatDate,
  formatDeltaPoints,
  formatDisplayPercent,
  formatInteger,
  formatNumber,
  prettifyToken,
} from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';
import MetricCard from './MetricCard.vue';

const snapshot = computed(() => dashboardStore.snapshot);
const environment = computed(() => snapshot.value?.environment);

const scores = computed(() => [
  ['Environment score', environment.value?.environment_score],
  ['Liquidity proxy', environment.value?.liquidity_proxy_score],
  ['Stress proxy', environment.value?.stress_proxy_score],
]);
</script>

<template>
  <article class="panel panel--soft">
    <div class="panel__header">
      <div>
        <p class="eyebrow">Environment layer</p>
        <h2>{{ environment ? prettifyToken(environment.environment_label) : 'Unavailable' }}</h2>
        <p v-if="environment" class="panel__lede">
          Scope-aware participation, liquidity proxy, and stress posture for the selected report date.
        </p>
      </div>
      <div v-if="environment" class="panel__actions">
        <span class="pill" :class="`pill--${breadthTone(environment.breadth_state)}`">
          Breadth · {{ prettifyToken(environment.breadth_state) }}
        </span>
        <span class="pill pill--outline">
          Regime as-of · {{ formatDate(environment.regime_as_of_date) }}
        </span>
      </div>
    </div>

    <div v-if="!environment" class="empty-state">
      <p>No environment snapshot is available for the selected report date.</p>
    </div>

    <template v-else>
      <div class="mini-metrics">
        <MetricCard
          label="Breadth"
          :value="formatDisplayPercent(environment.breadth_pct, 1)"
          :meta="`${formatInteger(environment.breadth_above_count)} / ${formatInteger(environment.breadth_eligible_count)} above MA30`"
        />
        <MetricCard
          label="Breadth SMA5"
          :value="formatDisplayPercent(environment.breadth_pct_sma5, 1)"
          meta="Smoothed participation"
        />
        <MetricCard
          label="Breadth 5d"
          :value="formatDeltaPoints(environment.breadth_5d_delta, 1)"
          meta="Repair or deterioration speed"
        />
        <MetricCard
          label="Volume expansion"
          :value="formatDisplayPercent(environment.volume_expansion_pct, 1)"
          meta="Share of tracked symbols above vol_ma20"
        />
      </div>

      <div class="score-stack">
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

      <p class="panel__note">
        Tracked-universe proxy only. Environment breadth remains based on enabled INDEX + ETF instruments, not a full stock-universe breadth series.
      </p>
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

.panel--soft {
  background: var(--panel-bg-secondary);
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

.panel__note {
  margin-top: var(--space-4);
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  font-style: italic;
}

.mini-metrics {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: var(--space-3);
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

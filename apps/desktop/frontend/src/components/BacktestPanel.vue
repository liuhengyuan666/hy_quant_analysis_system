<script setup>
import { computed } from 'vue';
import {
  formatCurrency,
  formatDate,
  formatInteger,
  formatNumber,
  formatPercent,
} from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';
import MetricCard from './MetricCard.vue';

const snapshot = computed(() => dashboardStore.snapshot);
const backtest = computed(() => snapshot.value?.latest_backtest);

const provenance = computed(() => {
  if (!backtest.value || !snapshot.value) return null;
  const snapshotScope = String(snapshot.value.scope || 'GLOBAL').toUpperCase();
  const analysisScope = String(backtest.value.analysis_scope || 'GLOBAL').toUpperCase();
  const signalScope = String(backtest.value.signal_scope || 'GLOBAL').toUpperCase();
  const regimeBasisScope = String(backtest.value.regime_basis_scope || 'GLOBAL').toUpperCase();
  const signalEndDate = backtest.value.signal_end_date || '';
  return {
    analysisScope,
    signalScope,
    regimeBasisScope,
    signalEndDate,
    matchesCurrentSnapshot: analysisScope === snapshotScope,
  };
});
</script>

<template>
  <article class="panel panel--soft">
    <div class="panel__header">
      <div>
        <p class="eyebrow">Validation</p>
        <h2>Latest backtest</h2>
        <p v-if="backtest" class="panel__lede">
          Recent strategy validation snapshot generated from the same pipeline.
        </p>
      </div>
      <div v-if="backtest" class="panel__actions">
        <span class="panel__meta">{{ backtest.strategy_name }}</span>
        <span
          v-if="provenance"
          class="pill"
          :class="`pill--${provenance.matchesCurrentSnapshot ? 'positive' : 'warning'}`"
        >
          {{ provenance.matchesCurrentSnapshot ? 'Matches current snapshot' : 'Snapshot mismatch' }}
        </span>
      </div>
    </div>

    <div v-if="provenance" class="panel__meta-row">
      <span class="panel__meta">Analysis scope · {{ provenance.analysisScope }}</span>
      <span class="panel__meta">Signal scope · {{ provenance.signalScope }}</span>
      <span class="panel__meta">Regime basis · {{ provenance.regimeBasisScope }}</span>
      <span class="panel__meta">Signal end · {{ provenance.signalEndDate ? formatDate(provenance.signalEndDate) : 'N/A' }}</span>
    </div>

    <div v-if="backtest" class="mini-metrics">
      <MetricCard label="CAGR" :value="formatPercent(backtest.cagr)" meta="Annualized return" tone="positive" />
      <MetricCard label="Max drawdown" :value="formatPercent(backtest.max_drawdown)" meta="Peak-to-trough" tone="negative" />
      <MetricCard label="Sharpe" :value="formatNumber(backtest.sharpe, 2)" meta="Risk-adjusted" />
      <MetricCard label="Final equity" :value="formatCurrency(backtest.final_equity)" :meta="`${formatInteger(backtest.trades)} trades · ${formatInteger(backtest.trading_days)} days`" />
    </div>

    <p v-if="backtest?.config_summary" class="panel__note">Config · {{ backtest.config_summary }}</p>

    <div v-if="!backtest" class="empty-state">
      <p>No backtest result is available yet.</p>
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

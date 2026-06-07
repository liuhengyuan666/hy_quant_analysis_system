<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  formatCurrency,
  formatDate,
  formatInteger,
  formatNumber,
  formatPercent,
} from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';
import MetricCard from './MetricCard.vue';

const { t } = useI18n();

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
        <p class="eyebrow">{{ t('backtest.eyebrow') }}</p>
        <h2>{{ t('backtest.title') }}</h2>
        <p v-if="backtest" class="panel__lede">
          {{ t('backtest.lede') }}
        </p>
      </div>
      <div v-if="backtest" class="panel__actions">
        <span class="panel__meta">{{ backtest.strategy_name }}</span>
        <span
          v-if="provenance"
          class="pill"
          :class="`pill--${provenance.matchesCurrentSnapshot ? 'positive' : 'warning'}`"
        >
          {{ provenance.matchesCurrentSnapshot ? t('backtest.matchesSnapshot') : t('backtest.snapshotMismatch') }}
        </span>
      </div>
    </div>

    <div v-if="provenance" class="panel__meta-row">
      <span class="panel__meta">{{ t('backtest.analysisScope', { scope: provenance.analysisScope }) }}</span>
      <span class="panel__meta">{{ t('backtest.signalScope', { scope: provenance.signalScope }) }}</span>
      <span class="panel__meta">{{ t('backtest.regimeBasis', { scope: provenance.regimeBasisScope }) }}</span>
      <span class="panel__meta">{{ t('backtest.signalEnd', { date: provenance.signalEndDate ? formatDate(provenance.signalEndDate) : 'N/A' }) }}</span>
    </div>

    <div v-if="backtest" class="mini-metrics">
      <MetricCard :label="t('backtest.cagr')" :value="formatPercent(backtest.cagr)" :meta="t('backtest.annualizedReturn')" tone="positive" />
      <MetricCard :label="t('backtest.maxDrawdown')" :value="formatPercent(backtest.max_drawdown)" :meta="t('backtest.peakToTrough')" tone="negative" />
      <MetricCard :label="t('backtest.sharpe')" :value="formatNumber(backtest.sharpe, 2)" :meta="t('backtest.riskAdjusted')" />
      <MetricCard :label="t('backtest.finalEquity')" :value="formatCurrency(backtest.final_equity)" :meta="t('backtest.tradesDays', { trades: formatInteger(backtest.trades), days: formatInteger(backtest.trading_days) })" />
    </div>

    <p v-if="backtest?.config_summary" class="panel__note">{{ t('backtest.config', { config: backtest.config_summary }) }}</p>

    <div v-if="!backtest" class="empty-state">
      <p>{{ t('backtest.noBacktest') }}</p>
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
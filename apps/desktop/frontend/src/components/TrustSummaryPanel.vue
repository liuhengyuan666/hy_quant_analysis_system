<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  formatDateTime,
  formatInteger,
  prettifyToken,
  trustTone,
} from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';
import MetricCard from './MetricCard.vue';

const { t } = useI18n();

const snapshot = computed(() => dashboardStore.snapshot);
const trust = computed(() => snapshot.value?.trust_summary);

const trustLevelTone = computed(() => trust.value ? trustTone(trust.value.level) : 'neutral');

const freshnessValue = computed(() => {
  if (!trust.value) return '';
  return trust.value.pipeline_has_stale_stage
    ? t('trust.staleStage', { count: formatInteger(trust.value.pipeline_stale_stage_count) })
    : trust.value.pipeline_has_partial_latest
      ? t('trust.partialLatest', { count: formatInteger(trust.value.pipeline_partial_latest_stage_count) })
      : t('trust.decisionStagesFresh');
});

const freshnessTone = computed(() => {
  if (!trust.value) return 'neutral';
  return trust.value.pipeline_has_stale_stage
    ? 'negative'
    : trust.value.pipeline_has_partial_latest || !trust.value.latest_day_complete
      ? 'warning'
      : 'positive';
});

const hasDataHealth = computed(() => trust.value?.data_health_generated_at !== null && trust.value?.data_health_generated_at !== undefined);

const dataHealthValue = computed(() => {
  if (!trust.value) return '';
  if (!hasDataHealth.value) return t('trust.dataHealthNotChecked');
  if (trust.value.data_health_critical_symbols > 0 || trust.value.data_health_critical_macro_sources > 0) {
    return t('trust.symbolMacroCritical', { symbols: formatInteger(trust.value.data_health_critical_symbols), macro: formatInteger(trust.value.data_health_critical_macro_sources) });
  }
  if (trust.value.data_health_review_symbols > 0 || trust.value.data_health_review_macro_sources > 0) {
    return t('trust.symbolMacroReview', { symbols: formatInteger(trust.value.data_health_review_symbols), macro: formatInteger(trust.value.data_health_review_macro_sources) });
  }
  return t('trust.noCriticalWarnings');
});

const dataHealthToneValue = computed(() => {
  if (!trust.value) return 'neutral';
  if (!hasDataHealth.value) return 'neutral';
  if (trust.value.data_health_critical_symbols > 0 || trust.value.data_health_critical_macro_sources > 0) return 'negative';
  if (trust.value.data_health_review_symbols > 0 || trust.value.data_health_review_macro_sources > 0) return 'warning';
  return 'positive';
});

const freshestMarketDate = computed(() => trust.value?.freshest_market_date || 'N/A');
const latestAvailableDate = computed(() => trust.value?.latest_available_date || 'N/A');

const historicalEvidenceNote = computed(() => {
  if (!snapshot.value?.report_date || !trust.value?.latest_available_date) return '';
  if (snapshot.value.report_date === trust.value.latest_available_date) return '';
  return t('trust.historicalNote', { date: trust.value.latest_available_date });
});
</script>

<template>
  <article v-if="trust" class="panel panel--accent">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('trust.eyebrow') }}</p>
        <h2>{{ trust.headline }}</h2>
        <p class="panel__lede">
          {{ t('trust.lede') }}
        </p>
      </div>
      <div class="panel__actions">
        <span class="pill" :class="`pill--${trustLevelTone}`">
          {{ prettifyToken(trust.level) }}
        </span>
      </div>
    </div>

    <p>{{ trust.message }}</p>

    <div class="panel__meta-row">
      <span class="panel__meta">{{ t('trust.dashboardScope', { scope: snapshot?.scope }) }}</span>
      <span class="panel__meta">{{ t('trust.signalAnalysisScope', { scope: trust.signal_analysis_scope || 'N/A' }) }}</span>
      <span class="panel__meta">{{ t('trust.signalRegimeBasisScope', { scope: trust.signal_regime_basis_scope || 'N/A' }) }}</span>
      <span class="panel__meta">
        {{ t('trust.backtestMatches', { matches: trust.backtest_matches_snapshot == null ? 'N/A' : trust.backtest_matches_snapshot ? 'yes' : 'no' }) }}
      </span>
    </div>

    <p v-if="historicalEvidenceNote" class="panel__note">{{ historicalEvidenceNote }}</p>

    <div class="mini-metrics">
      <MetricCard
        :label="t('trust.trustLevel')"
        :value="prettifyToken(trust.level)"
        :meta="trust.headline"
        :tone="trustLevelTone"
      />
      <MetricCard
        :label="t('trust.latestDayCoverage')"
        :value="`${formatInteger(trust.scoped_symbols_on_freshest_market_date)}/${formatInteger(trust.scoped_symbols_expected)}`"
        :meta="t('trust.freshestMarketDate', { date: freshestMarketDate })"
        :tone="trust.latest_day_complete ? 'positive' : 'warning'"
      />
      <MetricCard
        :label="t('trust.pipelineEvidence')"
        :value="freshnessValue"
        :meta="t('trust.latestAvailableMeta', { date: latestAvailableDate })"
        :tone="freshnessTone"
      />
      <MetricCard
        :label="t('trust.dataHealthEvidence')"
        :value="dataHealthValue"
        :meta="trust.data_health_generated_at ? `Generated ${formatDateTime(trust.data_health_generated_at)}` : t('trust.detailedHealthNotLoaded')"
        :tone="dataHealthToneValue"
      />
    </div>

    <section>
      <div class="panel__subheader">
        <p class="panel__section-title">{{ t('trust.freshnessEvidence') }}</p>
      </div>
      <p class="panel__note">
        {{ t('trust.freshnessVerdict', { verdict: freshnessValue, complete: trust.latest_day_complete ? 'yes' : 'no' }) }}
      </p>
    </section>

    <section>
      <div class="panel__subheader">
        <p class="panel__section-title">{{ t('trust.dataHealthSection') }}</p>
        <span class="panel__meta">{{ t('trust.macroStatus', { status: prettifyToken(trust.macro_status) }) }}</span>
      </div>
      <p class="panel__note">
        {{ t('trust.dataHealthVerdict', { digest: dataHealthValue }) }}
      </p>
    </section>

    <ul v-if="trust.notes?.length" class="note-list">
      <li v-for="(note, index) in trust.notes" :key="index">{{ note }}</li>
    </ul>
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
  margin: var(--space-4) 0;
}

.panel__subheader {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin: var(--space-4) 0 var(--space-2);
}

.panel__section-title {
  font-weight: 600;
  color: var(--text-primary);
}

.panel__note {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  font-style: italic;
  margin: var(--space-2) 0;
}

.mini-metrics {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: var(--space-3);
  margin: var(--space-4) 0;
}

.note-list {
  margin: var(--space-4) 0 0;
  padding-left: var(--space-5);
  list-style: disc;
}

.note-list li {
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
  margin-bottom: var(--space-1);
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

.pill--warning {
  background: var(--color-warning-soft);
  color: var(--color-warning);
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
</style>
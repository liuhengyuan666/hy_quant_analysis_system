<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatDate } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';
import MetricCard from './MetricCard.vue';

const { t } = useI18n();

const snapshot = computed(() => dashboardStore.snapshot);
const selectedReportDate = computed(() => dashboardStore.selectedReportDate);

const selectedDate = computed(() => snapshot.value?.report_date || selectedReportDate.value || '');
const latestAvailableDate = computed(() => snapshot.value?.latest_available_date || '');
const regimeAsOfDate = computed(() => snapshot.value?.regime_as_of_date || snapshot.value?.report_date || '');

const regimeFreshness = computed(() => {
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

const viewMode = computed(() => {
  if (!selectedDate.value || !latestAvailableDate.value) {
    return { value: t('timeContext.awaitingSnapshot'), meta: t('timeContext.loadToInspect'), tone: 'neutral' };
  }
  if (selectedDate.value === latestAvailableDate.value) {
    return { value: t('timeContext.latestSnapshot'), meta: t('timeContext.viewingNewest'), tone: 'positive' };
  }
  return { value: t('timeContext.historicalView'), meta: t('timeContext.behindLatest'), tone: 'neutral' };
});
</script>

<template>
  <article class="time-context-bar">
    <section class="overview-grid" aria-label="Dashboard time context">
      <MetricCard
        :label="t('timeContext.selectedDate')"
        :value="selectedDate ? formatDate(selectedDate) : t('timeContext.unavailable')"
        :meta="selectedDate ? t('timeContext.reflectSnapshot') : t('timeContext.loadToInspect')"
      />
      <MetricCard
        :label="t('timeContext.latestAnalysis')"
        :value="latestAvailableDate ? formatDate(latestAvailableDate) : t('timeContext.unavailable')"
        :meta="latestAvailableDate ? (selectedDate === latestAvailableDate ? t('timeContext.viewingNewest') : t('timeContext.newestStored')) : t('timeContext.noDates')"
        :tone="selectedDate && latestAvailableDate && selectedDate === latestAvailableDate ? 'positive' : 'neutral'"
      />
      <MetricCard
        :label="t('timeContext.regimeAsOf')"
        :value="regimeAsOfDate ? formatDate(regimeAsOfDate) : t('timeContext.unavailable')"
        :meta="regimeAsOfDate ? (regimeFreshness?.stale ? t('timeContext.macroRefreshedBefore') : t('timeContext.macroAligned')) : t('timeContext.macroUnavailable')"
        :tone="regimeFreshness?.tone || 'neutral'"
      />
      <MetricCard
        :label="t('timeContext.viewMode')"
        :value="viewMode.value"
        :meta="viewMode.meta"
        :tone="viewMode.tone"
      />
    </section>
  </article>
</template>

<style scoped>
.time-context-bar {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--space-3) var(--space-4);
  margin-top: var(--space-4);
}

.overview-grid {
  display: flex;
  gap: var(--space-2);
  justify-content: space-between;
  align-items: stretch;
}

.overview-grid :deep(.metric-card) {
  flex: 1 1 0;
  min-width: 0;
}
</style>

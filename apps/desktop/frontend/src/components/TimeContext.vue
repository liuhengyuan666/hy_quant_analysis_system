<script setup>
import { computed } from 'vue';
import { formatDate } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';
import MetricCard from './MetricCard.vue';

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
    return { value: 'Awaiting snapshot', meta: 'Load a dashboard snapshot to inspect a report date.', tone: 'neutral' };
  }
  if (selectedDate.value === latestAvailableDate.value) {
    return { value: 'Latest snapshot', meta: 'Selected analysis date matches the newest available analysis.', tone: 'positive' };
  }
  return { value: 'Historical view', meta: 'Selected analysis is behind the latest available analysis.', tone: 'neutral' };
});
</script>

<template>
  <section class="overview-grid" aria-label="Dashboard time context">
    <MetricCard
      label="Selected analysis date"
      :value="selectedDate ? formatDate(selectedDate) : 'Unavailable'"
      :meta="selectedDate ? 'All dashboard panels below reflect this analysis snapshot.' : 'Load a dashboard snapshot to inspect a report date.'"
    />
    <MetricCard
      label="Latest available analysis"
      :value="latestAvailableDate ? formatDate(latestAvailableDate) : 'Unavailable'"
      :meta="latestAvailableDate ? (selectedDate === latestAvailableDate ? 'You are viewing the newest analysis currently available.' : 'Newest stored analysis date available from the selector.') : 'No analysis dates are available yet.'"
      :tone="selectedDate && latestAvailableDate && selectedDate === latestAvailableDate ? 'positive' : 'neutral'"
    />
    <MetricCard
      label="Regime as-of date"
      :value="regimeAsOfDate ? formatDate(regimeAsOfDate) : 'Unavailable'"
      :meta="regimeAsOfDate ? (regimeFreshness?.stale ? 'Macro posture inputs were last refreshed before the selected analysis date.' : 'Macro posture inputs are aligned with the selected analysis date.') : 'Macro posture timestamp is unavailable.'"
      :tone="regimeFreshness?.tone || 'neutral'"
    />
    <MetricCard
      label="View mode"
      :value="viewMode.value"
      :meta="viewMode.meta"
      :tone="viewMode.tone"
    />
  </section>
</template>

<style scoped>
.overview-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: var(--space-3);
}
</style>

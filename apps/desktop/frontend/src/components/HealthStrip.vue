<script setup>
import { computed } from 'vue';
import { formatDate } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const status = computed(() => dashboardStore.status);
const snapshot = computed(() => dashboardStore.snapshot);
const exporting = computed(() => dashboardStore.exporting);
const exportResult = computed(() => dashboardStore.exportResult);

const exportTone = computed(() => {
  if (exportResult.value?.kind === 'success') return 'positive';
  if (exportResult.value?.kind === 'error') return 'negative';
  return 'neutral';
});

const items = computed(() => [
  {
    label: 'Runtime',
    value: status.value ? 'Connected' : 'Awaiting runtime',
    tone: status.value ? 'positive' : 'neutral',
  },
  {
    label: 'Snapshot',
    value: snapshot.value?.report_date
      ? `Loaded · ${formatDate(snapshot.value.report_date)}`
      : 'No report snapshot yet',
    tone: snapshot.value?.report_date ? 'positive' : 'neutral',
  },
  {
    label: 'Export',
    value: exporting.value
      ? 'Export in progress'
      : exportResult.value?.kind === 'success'
        ? 'Last export succeeded'
        : exportResult.value?.kind === 'error'
          ? 'Last export failed'
          : snapshot.value
            ? 'Ready to export'
            : 'Waiting for snapshot',
    tone: exporting.value ? 'neutral' : exportTone.value,
  },
]);
</script>

<template>
  <section class="health-strip" aria-label="Dashboard health summary">
    <article
      v-for="item in items"
      :key="item.label"
      class="status-chip"
      :class="`status-chip--${item.tone}`"
    >
      <span class="status-chip__label">{{ item.label }}</span>
      <strong class="status-chip__value">{{ item.value }}</strong>
    </article>
  </section>
</template>

<style scoped>
.health-strip {
  display: flex;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.status-chip {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
}

.status-chip__label {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.status-chip__value {
  font-size: var(--font-size-meta);
  font-weight: 600;
  color: var(--text-primary);
}

.status-chip--positive .status-chip__value {
  color: var(--tone-positive);
}

.status-chip--negative .status-chip__value {
  color: var(--tone-negative);
}
</style>

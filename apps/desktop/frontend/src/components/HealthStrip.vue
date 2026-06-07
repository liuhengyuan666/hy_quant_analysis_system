<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatDate } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

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
    label: t('healthStrip.runtime'),
    value: status.value ? t('healthStrip.connected') : t('healthStrip.awaitingRuntime'),
    tone: status.value ? 'positive' : 'neutral',
  },
  {
    label: t('healthStrip.snapshot'),
    value: snapshot.value?.report_date
      ? t('healthStrip.loaded', { date: formatDate(snapshot.value.report_date) })
      : t('healthStrip.noSnapshot'),
    tone: snapshot.value?.report_date ? 'positive' : 'neutral',
  },
  {
    label: t('healthStrip.export'),
    value: exporting.value
      ? t('healthStrip.exportInProgress')
      : exportResult.value?.kind === 'success'
        ? t('healthStrip.lastExportSuccess')
        : exportResult.value?.kind === 'error'
          ? t('healthStrip.lastExportFailed')
          : snapshot.value
            ? t('healthStrip.readyToExport')
            : t('healthStrip.waitingForSnapshot'),
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

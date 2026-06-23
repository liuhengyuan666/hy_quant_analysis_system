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
  <article class="panel">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('healthStrip.eyebrow') }}</p>
        <h2>{{ t('healthStrip.title') }}</h2>
      </div>
    </div>

    <div class="status-list">
      <div
        v-for="item in items"
        :key="item.label"
        class="status-item"
        :class="`status-item--${item.tone}`"
      >
        <span class="status-item__label">{{ item.label }}</span>
        <span class="status-item__value">{{ item.value }}</span>
      </div>
    </div>
  </article>
</template>

<style scoped>
.panel {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--panel-padding);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  height: 100%;
}

.panel__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--space-2);
}

.eyebrow {
  font-size: var(--font-size-label);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  margin: 0;
}

h2 {
  margin: var(--space-1) 0 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.status-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.status-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--panel-bg-secondary);
  border-radius: var(--panel-radius);
  border-left: 3px solid transparent;
  transition: border-color 0.15s ease;
}

.status-item--positive {
  border-left-color: var(--tone-positive);
}

.status-item--negative {
  border-left-color: var(--tone-negative);
}

.status-item--neutral {
  border-left-color: var(--color-border);
}

.status-item__label {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.status-item__value {
  font-size: var(--font-size-meta);
  font-weight: 600;
  color: var(--text-primary);
  text-align: right;
}

.status-item--positive .status-item__value {
  color: var(--tone-positive);
}

.status-item--negative .status-item__value {
  color: var(--tone-negative);
}
</style>

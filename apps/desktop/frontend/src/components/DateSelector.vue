<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatDate, formatScopeLabel } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const { t } = useI18n();
const availableDates = computed(() => dashboardStore.availableDates || []);
const selectedScope = computed(() => dashboardStore.selectedScope);
const selectedReportDate = computed(() => dashboardStore.selectedReportDate);
const snapshot = computed(() => dashboardStore.snapshot);
const loading = computed(() => dashboardStore.loading);

const hasDates = computed(() => availableDates.value.length > 0);
const latestAvailableDate = computed(() => snapshot.value?.latest_available_date || '');
const isLatestSelected = computed(() => latestAvailableDate.value && selectedReportDate.value === latestAvailableDate.value);

const emit = defineEmits(['update:scope', 'update:date', 'jump-to-latest']);
</script>

<template>
  <div class="hero__control">
    <div class="control-field">
      <div class="control-field__header">
        <label class="control-field__label" for="scopeSelect">{{ t('dateSelector.scopeDate') }}</label>
        <button
          class="button button--secondary button--compact"
          :disabled="!hasDates || loading || !latestAvailableDate || isLatestSelected"
          @click="emit('jump-to-latest')"
        >
          {{ isLatestSelected ? t('dateSelector.latestSelected') : t('dateSelector.jumpToLatest') }}
        </button>
      </div>

      <div class="select-row">
        <select
          id="scopeSelect"
          class="select-control"
          :value="selectedScope"
          :disabled="loading"
          @change="emit('update:scope', $event.target.value)"
        >
          <option value="global">{{ t('dateSelector.globalShared') }}</option>
          <option value="cn">{{ t('dateSelector.cnComplete') }}</option>
          <option value="hk">{{ t('dateSelector.hkComplete') }}</option>
        </select>

        <select
          id="reportDateSelect"
          class="select-control"
          :value="selectedReportDate"
          :disabled="!hasDates || loading"
          @change="emit('update:date', $event.target.value)"
        >
          <option v-if="!hasDates" value="">{{ t('dateSelector.noDates') }}</option>
          <option
            v-for="(date, index) in availableDates"
            :key="date"
            :value="date"
          >
            {{ formatDate(date) }}{{ index === 0 ? t('dateSelector.latestSuffix') : '' }}
          </option>
        </select>
      </div>

      <div class="control-field__toolbar">
        <span class="panel__meta">{{ t('dateSelector.scopeLabel', { scope: formatScopeLabel(selectedScope) }) }}</span>
        <span class="panel__meta">
          {{ latestAvailableDate ? t('dateSelector.latestAvailable', { date: formatDate(latestAvailableDate) }) : t('dateSelector.latestUnavailable') }}
        </span>
      </div>

      <span class="control-field__hint">
        {{ hasDates
          ? t('dateSelector.scopeHint', { count: availableDates.length })
          : t('dateSelector.noDashboardDates')
        }}
      </span>
    </div>
  </div>
</template>

<style scoped>
.hero__control {
  margin-bottom: var(--space-4);
}

.control-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.control-field__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.control-field__label {
  font-size: var(--font-size-meta);
  font-weight: 600;
  color: var(--text-primary);
}

.control-field__toolbar {
  display: flex;
  gap: var(--space-4);
  flex-wrap: wrap;
}

.control-field__hint {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

.select-row {
  display: flex;
  gap: var(--space-3);
}

.select-row .select-control {
  flex: 1;
  min-width: 0;
}

.select-control {
  padding: var(--space-2) var(--space-3);
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  color: var(--text-primary);
  font-size: var(--font-size-meta);
  cursor: pointer;
}

.select-control:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.button {
  padding: var(--space-2) var(--space-4);
  border-radius: var(--panel-radius);
  font-size: var(--font-size-meta);
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s ease;
}

.button--secondary {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  color: var(--text-primary);
}

.button--secondary:hover {
  background: var(--panel-bg);
}

.button--compact {
  padding: var(--space-2) var(--space-3);
  font-size: var(--font-size-label);
}

.button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.panel__meta {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}
</style>
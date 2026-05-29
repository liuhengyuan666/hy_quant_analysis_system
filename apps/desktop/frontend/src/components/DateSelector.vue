<script setup>
import { computed } from 'vue';
import { formatDate, formatScopeLabel } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

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
        <label class="control-field__label" for="scopeSelect">Scope & analysis date</label>
        <button
          class="button button--secondary button--compact"
          :disabled="!hasDates || loading || !latestAvailableDate || isLatestSelected"
          @click="emit('jump-to-latest')"
        >
          {{ isLatestSelected ? 'Latest selected' : 'Jump to latest' }}
        </button>
      </div>

      <select
        id="scopeSelect"
        class="select-control"
        :value="selectedScope"
        :disabled="loading"
        @change="emit('update:scope', $event.target.value)"
      >
        <option value="global">GLOBAL · Shared latest date</option>
        <option value="cn">CN · A-share complete latest date</option>
        <option value="hk">HK · Hong Kong complete latest date</option>
      </select>

      <select
        id="reportDateSelect"
        class="select-control"
        :value="selectedReportDate"
        :disabled="!hasDates || loading"
        @change="emit('update:date', $event.target.value)"
      >
        <option v-if="!hasDates" value="">No analysis dates available</option>
        <option
          v-for="(date, index) in availableDates"
          :key="date"
          :value="date"
        >
          {{ formatDate(date) }}{{ index === 0 ? ' · Latest' : '' }}
        </option>
      </select>

      <div class="control-field__toolbar">
        <span class="panel__meta">Scope · {{ formatScopeLabel(selectedScope) }}</span>
        <span class="panel__meta">
          {{ latestAvailableDate ? `Latest available · ${formatDate(latestAvailableDate)}` : 'Latest available date unavailable' }}
        </span>
      </div>

      <span class="control-field__hint">
        {{ hasDates
          ? `Scope controls which market set defines the latest complete report date. Selected analysis date drives every panel below. ${availableDates.length} selectable date${availableDates.length === 1 ? '' : 's'}.`
          : 'No dashboard analysis dates are available yet.'
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

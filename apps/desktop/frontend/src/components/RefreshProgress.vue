<script setup>
import { computed } from 'vue';
import { formatDate, formatDateTime, formatInteger } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const refresh = computed(() => dashboardStore.refreshStatus);
const loading = computed(() => dashboardStore.loading);

const isVisible = computed(() => {
  return refresh.value.running || ['error', 'success', 'cancelled'].includes(refresh.value.status);
});

const tone = computed(() => {
  if (refresh.value.status === 'error') return 'negative';
  if (refresh.value.status === 'cancelled') return 'warning';
  if (refresh.value.running) return 'neutral';
  return 'positive';
});

const progress = computed(() => Math.max(0, Math.min(100, Number(refresh.value.progress_pct || 0))));

const rangeText = computed(() => {
  if (refresh.value.refresh_from && refresh.value.refresh_to) {
    return `${formatDate(refresh.value.refresh_from)} → ${formatDate(refresh.value.refresh_to)}`;
  }
  return 'Preparing refresh range';
});

const timingText = computed(() => {
  if (refresh.value.running) return `Started ${formatDateTime(refresh.value.started_at)}`;
  if (refresh.value.finished_at) return `Finished ${formatDateTime(refresh.value.finished_at)}`;
  return 'Waiting to start';
});

const title = computed(() => {
  if (refresh.value.running) return refresh.value.cancelling ? 'Cancelling refresh' : 'Refreshing analysis pipeline';
  if (refresh.value.status === 'error') return 'Refresh failed';
  if (refresh.value.status === 'cancelled') return 'Refresh cancelled';
  return 'Refresh completed';
});

const emit = defineEmits(['cancel', 'retry', 'resume']);
</script>

<template>
  <section v-if="isVisible" class="refresh-progress" :class="`refresh-progress--${tone}`" aria-live="polite">
    <div class="refresh-progress__header">
      <div>
        <p class="eyebrow">Background refresh</p>
        <h2>{{ title }}</h2>
        <p class="panel__lede">{{ refresh.stage || 'Waiting' }}</p>
      </div>
      <div class="panel__actions">
        <span class="pill pill--outline">Run from · {{ refresh.start_stage }}</span>
        <span v-if="refresh.retry_from_stage" class="pill pill--warning">
          Retry from · {{ refresh.retry_from_stage }}
        </span>
        <span v-if="refresh.cancelling" class="pill pill--warning">Cancelling...</span>
        <span class="pill" :class="`pill--${tone}`">{{ formatInteger(progress) }}%</span>
      </div>
    </div>

    <div class="refresh-progress__bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" :aria-valuenow="progress">
      <span class="refresh-progress__fill" :style="{ width: `${progress}%` }"></span>
    </div>

    <div class="refresh-progress__meta-row">
      <span>{{ rangeText }}</span>
      <span>{{ timingText }}</span>
    </div>

    <div v-if="refresh.running && !refresh.cancelling" class="refresh-progress__meta-row">
      <button
        class="button button--secondary button--compact"
        :disabled="loading"
        @click="emit('cancel')"
      >
        Cancel
      </button>
    </div>

    <section v-if="refresh.status === 'cancelled'" class="notice notice--warning notice--inline">
      <div>
        <strong>Refresh was cancelled</strong>
        <p>Last successful stage: {{ refresh.last_successful_stage || 'none' }}</p>
      </div>
    </section>

    <div v-if="refresh.status === 'cancelled'" class="refresh-progress__meta-row">
      <button
        class="button button--secondary button--compact"
        :disabled="loading || refresh.running"
        @click="emit('resume')"
      >
        Resume
      </button>
    </div>

    <div v-if="refresh.status === 'error'" class="refresh-progress__meta-row">
      <button
        class="button button--secondary button--compact"
        :disabled="loading || refresh.running"
        @click="emit('retry')"
      >
        Retry failed stage
      </button>
    </div>

    <p v-if="refresh.error" class="refresh-progress__error">{{ refresh.error }}</p>
  </section>
</template>

<style scoped>
.refresh-progress {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--panel-padding);
  margin-bottom: var(--space-4);
}

.refresh-progress--negative {
  border-color: var(--tone-negative);
}

.refresh-progress--warning {
  border-color: var(--color-warning);
}

.refresh-progress--positive {
  border-color: var(--tone-positive);
}

.refresh-progress__header {
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

.refresh-progress__bar {
  height: 8px;
  background: var(--score-bar-bg);
  border-radius: var(--space-1);
  overflow: hidden;
  margin-bottom: var(--space-3);
}

.refresh-progress__fill {
  display: block;
  height: 100%;
  background: var(--accent-primary);
  border-radius: var(--space-1);
  transition: width 0.3s ease;
}

.refresh-progress__meta-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
}

.refresh-progress__error {
  font-size: var(--font-size-meta);
  color: var(--tone-negative);
  margin-top: var(--space-2);
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

.pill {
  display: inline-flex;
  align-items: center;
  padding: var(--space-1) var(--space-3);
  border-radius: var(--space-3);
  font-size: var(--font-size-label);
  font-weight: 500;
}

.pill--outline {
  border: 1px solid var(--pill-outline-border);
  color: var(--text-secondary);
  background: transparent;
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

.notice {
  padding: var(--space-3) var(--space-4);
  border-radius: var(--panel-radius);
  margin: var(--space-3) 0;
}

.notice--warning {
  background: var(--color-warning-soft);
  border: 1px solid var(--color-warning);
}

.notice strong {
  display: block;
  margin-bottom: var(--space-1);
  color: var(--color-warning);
}

.notice p {
  margin: var(--space-1) 0;
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
}
</style>

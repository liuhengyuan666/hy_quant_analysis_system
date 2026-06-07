<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const insight = computed(() => dashboardStore.insight);

const hasInsight = computed(() => Boolean(insight.value));

const confidencePercent = computed(() => {
  if (!insight.value) return 0;
  return Math.round(insight.value.confidence * 100);
});

const confidenceLabel = computed(() => {
  const pct = confidencePercent.value;
  if (pct >= 80) return t('insight.confidenceHigh');
  if (pct >= 60) return t('insight.confidenceMedium');
  return t('insight.confidenceLow');
});
</script>

<template>
  <div v-if="hasInsight" class="insight-panel">
    <div class="insight-panel__header">
      <h2 class="insight-panel__headline">{{ insight.headline }}</h2>
      <span
        class="insight-panel__confidence"
        :class="{
          'insight-panel__confidence--high': confidencePercent >= 80,
          'insight-panel__confidence--medium': confidencePercent >= 60 && confidencePercent < 80,
          'insight-panel__confidence--low': confidencePercent < 60,
        }"
      >
        {{ confidenceLabel }} ({{ confidencePercent }}%)
      </span>
    </div>

    <p class="insight-panel__summary">{{ insight.summary }}</p>

    <div v-if="insight.regime_transition" class="insight-panel__transition">
      <span class="insight-panel__transition-icon">&#9432;</span>
      {{ insight.regime_transition }}
    </div>

    <div v-if="insight.implications?.length" class="insight-panel__section">
      <h3 class="insight-panel__section-title">{{ t('insight.implications') }}</h3>
      <ul class="insight-panel__list">
        <li v-for="(item, index) in insight.implications" :key="`impl-${index}`" class="insight-panel__list-item">
          {{ item }}
        </li>
      </ul>
    </div>

    <div v-if="insight.recommendations?.length" class="insight-panel__section">
      <h3 class="insight-panel__section-title">{{ t('insight.recommendations') }}</h3>
      <ul class="insight-panel__list insight-panel__list--recommendations">
        <li v-for="(item, index) in insight.recommendations" :key="`rec-${index}`" class="insight-panel__list-item">
          <span class="insight-panel__bullet">&#8226;</span>
          {{ item }}
        </li>
      </ul>
    </div>
  </div>
</template>

<style scoped>
.insight-panel {
  background: var(--color-surface-elevated);
  border: 1px solid var(--color-border-subtle);
  border-radius: var(--radius-lg);
  padding: var(--space-5);
  margin-bottom: var(--space-4);
}

.insight-panel__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}

.insight-panel__headline {
  font-size: var(--font-size-xl);
  font-weight: 700;
  color: var(--color-text-primary);
  line-height: 1.3;
  margin: 0;
  flex: 1;
}

.insight-panel__confidence {
  font-size: var(--font-size-sm);
  font-weight: 600;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-md);
  white-space: nowrap;
  flex-shrink: 0;
}

.insight-panel__confidence--high {
  background: var(--color-success-subtle);
  color: var(--color-success);
}

.insight-panel__confidence--medium {
  background: var(--color-warning-subtle);
  color: var(--color-warning);
}

.insight-panel__confidence--low {
  background: var(--color-error-subtle);
  color: var(--color-error);
}

.insight-panel__summary {
  font-size: var(--font-size-base);
  color: var(--color-text-secondary);
  line-height: 1.6;
  margin: 0 0 var(--space-3);
}

.insight-panel__transition {
  font-size: var(--font-size-sm);
  color: var(--color-text-tertiary);
  background: var(--color-surface);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-3);
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.insight-panel__transition-icon {
  font-size: var(--font-size-base);
  line-height: 1;
}

.insight-panel__section {
  margin-top: var(--space-3);
}

.insight-panel__section-title {
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 0 0 var(--space-2);
}

.insight-panel__list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.insight-panel__list-item {
  font-size: var(--font-size-base);
  color: var(--color-text-primary);
  line-height: 1.5;
  padding-left: var(--space-3);
  position: relative;
}

.insight-panel__list-item::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0.6em;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-text-tertiary);
}

.insight-panel__list--recommendations .insight-panel__list-item::before {
  background: var(--color-primary);
}

.insight-panel__bullet {
  display: none;
}
</style>

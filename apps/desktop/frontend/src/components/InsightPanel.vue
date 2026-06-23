<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const insight = computed(() => dashboardStore.insight);
const hasInsight = computed(() => Boolean(insight.value));
</script>

<template>
  <article class="panel">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('insight.eyebrow') }}</p>
        <h2>{{ insight?.headline || t('insight.awaiting') }}</h2>
        <p v-if="insight?.summary" class="panel__lede">{{ insight.summary }}</p>
      </div>
      <span v-if="insight?.regime_transition" class="pill pill--warning">
        {{ t('insight.transition') }}
      </span>
    </div>

    <div v-if="insight?.implications?.length" class="insight-list">
      <div
        v-for="(item, index) in insight.implications"
        :key="`impl-${index}`"
        class="insight-item"
      >
        <span class="insight-item__icon">▸</span>
        <span class="insight-item__text">{{ item }}</span>
      </div>
    </div>

    <div v-else class="insight-placeholder">
      <span class="insight-placeholder__icon">?</span>
      <p>{{ t('insight.placeholder') }}</p>
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
  gap: var(--space-3);
  margin-bottom: var(--space-2);
}

.panel__lede {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  margin-top: var(--space-1);
  margin-bottom: 0;
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

.insight-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.insight-item {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--panel-bg-secondary);
  border-radius: var(--panel-radius);
  border-left: 3px solid var(--color-accent);
}

.insight-item__icon {
  color: var(--color-accent);
  font-size: 1rem;
  line-height: 1.5;
  flex-shrink: 0;
}

.insight-item__text {
  color: var(--text-primary);
  font-size: var(--font-size-meta);
  line-height: 1.5;
}

.insight-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  padding: var(--space-5);
  background: var(--panel-bg-secondary);
  border-radius: var(--panel-radius);
  border: 1px dashed var(--panel-border);
  flex: 1;
}

.insight-placeholder__icon {
  font-size: 2rem;
  width: 3rem;
  height: 3rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--color-surface-raised);
  color: var(--text-secondary);
  border: 2px solid var(--panel-border);
  font-family: var(--font-display);
}

.insight-placeholder p {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  text-align: center;
  margin: 0;
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: var(--space-1) var(--space-3);
  border-radius: var(--space-3);
  font-size: var(--font-size-label);
  font-weight: 500;
  flex-shrink: 0;
}

.pill--warning {
  background: var(--color-warning-soft);
  color: var(--color-warning);
}
</style>

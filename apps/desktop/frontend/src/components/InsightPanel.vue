<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const insight = computed(() => dashboardStore.insight);
const hasInsight = computed(() => Boolean(insight.value));

function renderMarkdown(text) {
  if (!text) return '';
  let html = String(text)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');

  html = html
    .replace(/^# (.*$)/gim, '<h1>$1</h1>')
    .replace(/^## (.*$)/gim, '<h2>$1</h2>')
    .replace(/^### (.*$)/gim, '<h3>$1</h3>')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.*?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\n/g, '<br>');

  const unsafePatterns = [
    /<script[^>]*>[\s\S]*?<\/script>/gi,
    /<iframe[^>]*>[\s\S]*?<\/iframe>/gi,
    /<object[^>]*>[\s\S]*?<\/object>/gi,
    /<embed[^>]*>/gi,
    /<form[^>]*>[\s\S]*?<\/form>/gi,
    /on\w+\s*=\s*["']?[^"'>]*["']?/gi,
    /javascript:/gi,
  ];
  unsafePatterns.forEach((pattern) => {
    html = html.replace(pattern, '');
  });

  return html;
}
</script>

<template>
  <div v-if="hasInsight" class="insight-panel">
    <div class="insight-panel__header">
      <h2 class="insight-panel__headline">{{ insight.headline }}</h2>
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
</style>

<script setup>
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

defineProps({
  label: {
    type: String,
    required: true,
  },
  value: {
    type: [String, Number],
    default: 'N/A',
  },
  meta: {
    type: String,
    default: '',
  },
  tone: {
    type: String,
    default: 'neutral',
    validator: (v) => ['positive', 'negative', 'neutral', 'warning'].includes(v),
  },
});
</script>

<template>
  <article class="metric-card" :class="`metric-card--${tone}`">
    <span class="metric-card__label">{{ label }}</span>
    <strong class="metric-card__value">{{ value }}</strong>
    <span v-if="meta" class="metric-card__meta">{{ meta }}</span>
  </article>
</template>

<style scoped>
.metric-card {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.metric-card__label {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.metric-card__value {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.metric-card__meta {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

.metric-card--positive .metric-card__value {
  color: var(--tone-positive);
}

.metric-card--negative .metric-card__value {
  color: var(--tone-negative);
}

.metric-card--warning .metric-card__value {
  color: var(--color-warning, #f0c57c);
}
</style>
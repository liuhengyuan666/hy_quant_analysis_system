<script setup>
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

defineProps({
  result: {
    type: Object,
    default: null,
  },
});
</script>

<template>
  <section v-if="result" class="notice" :class="`notice--${result.kind}`">
    <div>
      <strong>{{ result.title }}</strong>
      <p>{{ result.message }}</p>
      <p v-if="result.output_path" class="notice__detail">
        <code>{{ result.output_path }}</code>
      </p>
      <p v-if="result.failed_items?.length" class="notice__detail">
        {{ t('notice.warnings', { warnings: result.failed_items.join(' · ') }) }}
      </p>
      <p v-else-if="result.kind === 'success'" class="notice__detail">
        {{ t('notice.allCompleted') }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.notice {
  padding: var(--space-3) var(--space-4);
  border-radius: var(--panel-radius);
  margin-bottom: var(--space-4);
}

.notice--success {
  background: var(--tone-positive-bg);
  border: 1px solid var(--tone-positive);
}

.notice--error {
  background: var(--tone-negative-bg);
  border: 1px solid var(--tone-negative);
}

.notice strong {
  display: block;
  margin-bottom: var(--space-1);
  color: var(--text-primary);
}

.notice p {
  margin: var(--space-1) 0;
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
}

.notice__detail {
  font-size: var(--font-size-label);
  margin-top: var(--space-2);
}

.notice__detail code {
  font-family: monospace;
  font-size: var(--font-size-label);
  padding: var(--space-1) var(--space-2);
  background: var(--panel-bg);
  border-radius: var(--space-1);
}
</style>
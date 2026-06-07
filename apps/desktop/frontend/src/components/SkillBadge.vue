<script setup>
import { computed } from 'vue';

const props = defineProps({
  name: { type: String, default: '' },
  description: { type: String, default: '' },
  size: { type: String, default: 'md' }, // 'sm' | 'md'
});

const displayText = computed(() => props.description || props.name || '');
const sizeClass = computed(() => `skill-badge--${props.size}`);
</script>

<template>
  <span v-if="displayText" class="skill-badge" :class="sizeClass" :title="name">
    {{ displayText }}
  </span>
</template>

<style scoped>
.skill-badge {
  display: inline-flex;
  align-items: center;
  padding: var(--space-1) var(--space-3);
  border-radius: var(--radius-pill);
  background: var(--color-accent-soft);
  color: var(--color-accent);
  font-size: var(--font-size-meta);
  font-weight: 600;
  white-space: nowrap;
  line-height: 1.2;
}

.skill-badge--sm {
  padding: 1px var(--space-2);
  font-size: 0.7rem;
}
</style>

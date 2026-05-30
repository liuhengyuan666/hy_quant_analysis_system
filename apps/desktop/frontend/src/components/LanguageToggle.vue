<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { setLocale } from '../i18n.js';

const { t, locale } = useI18n();
const isZh = computed(() => locale.value === 'zh');

function toggle() {
  setLocale(isZh.value ? 'en' : 'zh');
}
</script>

<template>
  <button
    class="language-toggle"
    @click="toggle"
    :aria-label="isZh ? t('common.switchToEn') : t('common.switchToZh')"
  >
    {{ isZh ? 'EN' : '中' }}
  </button>
</template>

<style scoped>
.language-toggle {
  padding: var(--space-1) var(--space-2);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  background: var(--panel-bg);
  color: var(--text-primary);
  font-size: var(--font-size-label);
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s ease;
}

.language-toggle:hover {
  background: var(--panel-bg-secondary);
}
</style>

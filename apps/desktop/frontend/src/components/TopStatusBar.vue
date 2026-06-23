<script setup>
import { computed, ref, onMounted, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore } from '../store.js';
import LanguageToggle from './LanguageToggle.vue';

const { t } = useI18n();

const emit = defineEmits(['refresh', 'export', 'openGuides', 'runPrecloseAnalysis', 'changeScope']);

const snapshot = computed(() => dashboardStore.snapshot);
const refreshing = computed(() => dashboardStore.refreshing);
const refreshStatus = computed(() => dashboardStore.refreshStatus);
const exporting = computed(() => dashboardStore.exporting);
const precloseAnalyzing = computed(() => dashboardStore.precloseAnalyzing);
const isBusy = computed(() => refreshing.value || refreshStatus.value.running);

const selectedScope = computed(() => dashboardStore.selectedScope || 'global');
const scope = computed(() => snapshot.value?.scope?.toUpperCase() || 'GLOBAL');
const regime = computed(() => snapshot.value?.regime_label || '-');
const date = computed(() => snapshot.value?.report_date || '-');

const SCOPES = ['global', 'cn', 'hk'];

function handleChangeScope(newScope) {
  if (newScope !== selectedScope.value) {
    emit('changeScope', newScope);
  }
}

const regimeClass = computed(() => {
  const label = regime.value?.toLowerCase() || '';
  if (label.includes('risk_on')) return 'regime--risk-on';
  if (label.includes('risk_off')) return 'regime--risk-off';
  return 'regime--neutral';
});

// Real-time clock
const currentTime = ref('');
let timer = null;

function updateTime() {
  const now = new Date();
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, '0');
  const d = String(now.getDate()).padStart(2, '0');
  const h = String(now.getHours()).padStart(2, '0');
  const min = String(now.getMinutes()).padStart(2, '0');
  const s = String(now.getSeconds()).padStart(2, '0');
  currentTime.value = `${y}-${m}-${d} ${h}:${min}:${s}`;
}

onMounted(() => {
  updateTime();
  timer = setInterval(updateTime, 1000);
});

onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});
</script>

<template>
  <header class="top-status-bar">
    <div class="top-status-bar__left">
      <span class="logo">Q</span>
      <div class="scope-switcher">
        <button
          v-for="s in SCOPES"
          :key="s"
          class="scope-btn"
          :class="{ 'scope-btn--active': selectedScope === s }"
          @click="handleChangeScope(s)"
        >
          {{ s.toUpperCase() }}
        </button>
      </div>
      <span class="regime" :class="regimeClass">{{ regime }}</span>
      <span class="date">{{ date }}</span>
    </div>
    <div class="top-status-bar__center">
      <span class="system-clock">{{ currentTime }}</span>
    </div>
    <div class="top-status-bar__right">
      <button class="top-btn" @click="emit('openGuides')">
        {{ t('dashboardHero.guideViewer') }}
      </button>
      <button
        class="top-btn"
        :disabled="isBusy"
        @click="emit('refresh')"
      >
        {{ isBusy ? t('hero.refreshing') : t('hero.refreshData') }}
      </button>
      <button
        class="top-btn top-btn--primary"
        :disabled="exporting || !snapshot || isBusy"
        @click="emit('export')"
      >
        {{ exporting ? t('hero.exporting') : t('hero.exportReport') }}
      </button>
      <button
        class="top-btn top-btn--accent"
        :disabled="isBusy || precloseAnalyzing"
        @click="emit('runPrecloseAnalysis')"
      >
        {{ precloseAnalyzing ? t('hero.analyzing') : t('hero.runPrecloseAnalysis') }}
      </button>
      <LanguageToggle />
    </div>
  </header>
</template>

<style scoped>
.top-status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 3.5rem;
  padding: var(--space-3) var(--space-5);
  background: var(--color-surface-strong);
  border-bottom: 1px solid var(--color-border);
  position: sticky;
  top: 0;
  z-index: 20;
  font-size: var(--font-size-label);
  gap: var(--space-4);
}

.top-status-bar__left {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.top-status-bar__right {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.top-status-bar__center {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
}

.system-clock {
  font-family: var(--font-mono);
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
  letter-spacing: 0.05em;
  user-select: none;
}

.logo {
  font-family: var(--font-display);
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--color-accent);
  width: 1.75rem;
  height: 1.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--color-accent);
  border-radius: var(--radius-sm);
}

.scope {
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.05em;
}

.regime {
  padding: var(--space-1) var(--space-3);
  border-radius: var(--radius-sm);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.regime--risk-on {
  background: rgba(22, 199, 132, 0.15);
  color: #16c784;
  border: 1px solid rgba(22, 199, 132, 0.3);
}

.regime--risk-off {
  background: rgba(255, 92, 92, 0.15);
  color: #ff5c5c;
  border: 1px solid rgba(255, 92, 92, 0.3);
}

.regime--neutral {
  background: rgba(159, 177, 199, 0.15);
  color: #9fb1c7;
  border: 1px solid rgba(159, 177, 199, 0.3);
}

.date {
  color: var(--text-secondary);
  font-family: 'JetBrains Mono', monospace;
}

.top-btn {
  padding: var(--space-2) var(--space-3);
  background: var(--color-surface-raised);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: var(--font-size-label);
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
  white-space: nowrap;
}

.top-btn:hover:not(:disabled) {
  background: var(--color-surface-soft);
  border-color: var(--color-border-strong);
}

.top-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.top-btn--primary {
  background: var(--color-accent-soft);
  border-color: var(--color-accent-border);
  color: var(--color-accent);
}

.top-btn--primary:hover:not(:disabled) {
  background: var(--color-accent);
  color: var(--color-bg);
}

.top-btn--accent {
  background: var(--color-secondary-soft);
  border-color: var(--color-secondary);
  color: var(--color-secondary);
}

.top-btn--accent:hover:not(:disabled) {
  background: var(--color-secondary);
  color: var(--color-bg);
}

.scope-switcher {
  display: flex;
  gap: 1px;
  background: var(--color-border);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.scope-btn {
  padding: var(--space-1) var(--space-3);
  background: var(--color-surface-raised);
  border: none;
  color: var(--text-secondary);
  font-size: var(--font-size-label);
  font-weight: 500;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
  font-family: var(--font-mono);
  letter-spacing: 0.04em;
}

.scope-btn:hover {
  background: var(--color-surface-soft);
  color: var(--text-primary);
}

.scope-btn--active {
  background: var(--color-accent-soft);
  color: var(--color-accent);
  font-weight: 600;
}

@media (max-width: 1080px) {
  .top-status-bar {
    flex-wrap: wrap;
    height: auto;
    padding: var(--space-3);
  }
  .top-status-bar__left {
    flex-wrap: wrap;
  }
  .top-status-bar__right {
    flex-wrap: wrap;
  }
}
</style>

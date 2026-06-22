<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore, updateSelectedRefreshStartStage } from '../store.js';
import { formatDateTime } from '../lib/dashboard-utils.js';
import LanguageToggle from './LanguageToggle.vue';

const { t } = useI18n();

const REFRESH_START_STAGE_OPTIONS = computed(() => [
  { value: 'full', label: t('refreshStages.full') },
  { value: 'ingest', label: t('refreshStages.ingest') },
  { value: 'indicators', label: t('refreshStages.indicators') },
  { value: 'macro', label: t('refreshStages.macro') },
  { value: 'rotation', label: t('refreshStages.rotation') },
  { value: 'strategy', label: t('refreshStages.strategy') },
  { value: 'signals', label: t('refreshStages.signals') },
  { value: 'backtests', label: t('refreshStages.backtests') },
]);

const emit = defineEmits(['refresh', 'export', 'openGuides', 'runPrecloseAnalysis']);

const loading = computed(() => dashboardStore.loading);
const refreshing = computed(() => dashboardStore.refreshing);
const refreshStatus = computed(() => dashboardStore.refreshStatus);
const exporting = computed(() => dashboardStore.exporting);
const snapshot = computed(() => dashboardStore.snapshot);
const lastUpdatedAt = computed(() => dashboardStore.lastUpdatedAt);
const precloseAnalyzing = computed(() => dashboardStore.precloseAnalyzing);
const selectedRefreshStartStage = computed({
  get: () => dashboardStore.selectedRefreshStartStage,
  set: (value) => {
    updateSelectedRefreshStartStage(value);
  },
});

const isBusy = computed(() => loading.value || refreshing.value || refreshStatus.value.running);

function handleRefresh() {
  emit('refresh', selectedRefreshStartStage.value);
}

function handleStageChange(event) {
  selectedRefreshStartStage.value = String(event.target.value || 'full');
}
</script>

<template>
  <section class="hero">
    <div class="hero__frame">
      <div class="hero__copy">
        <p class="eyebrow">{{ t('hero.eyebrow') }}</p>
        <h1>{{ t('hero.title') }}</h1>
        <p class="hero__lede">
          {{ t('hero.lede') }}
        </p>
      </div>
      <div class="hero__actions">
        <div class="hero__control hero__control--guide">
          <div class="control-field">
        <div class="control-field__header">
          <span class="control-field__label">{{ t('dashboardHero.helpUsage') }}</span>
          <div class="control-field__actions">
            <span class="pill pill--outline">{{ t('dashboardHero.guideViewer') }}</span>
            <LanguageToggle />
          </div>
        </div>
            <span class="control-field__hint">
              {{ t('dashboardHero.guideHint') }}
            </span>
            <button class="button button--secondary guide-entry__button" @click="emit('openGuides')">
              {{ t('dashboardHero.openGuides') }}
            </button>
          </div>
        </div>
        <div class="hero__action-row">
          <select
            class="select-control select-control--compact"
            :disabled="isBusy"
            :value="selectedRefreshStartStage"
            @change="handleStageChange"
          >
            <option
              v-for="opt in REFRESH_START_STAGE_OPTIONS"
              :key="opt.value"
              :value="opt.value"
            >
              {{ opt.label }}
            </option>
          </select>
          <button
            class="button button--secondary"
            :disabled="isBusy"
            @click="handleRefresh"
          >
            {{ (refreshing || refreshStatus.running) ? t('hero.refreshing') : t('hero.refreshData') }}
          </button>
          <button
            class="button button--primary"
            :disabled="exporting || !snapshot || isBusy"
            @click="emit('export')"
          >
            {{ exporting ? t('hero.exporting') : t('hero.exportReport') }}
          </button>
          <button
            class="button button--experimental"
            :disabled="isBusy || precloseAnalyzing"
            @click="emit('runPrecloseAnalysis')"
          >
            {{ precloseAnalyzing ? t('hero.analyzing') : t('hero.runPrecloseAnalysis') }}
          </button>
        </div>
        <p class="hero__timestamp">{{ t('hero.lastSync') }} {{ formatDateTime(lastUpdatedAt) }}</p>
      </div>
    </div>
    <div class="hero__ambient" aria-hidden="true"></div>
  </section>
</template>

<style scoped>
/* Uses global styles from styles.css for .hero, .hero__frame, .hero__copy, etc. */

.hero__actions {
  flex: 0 1 26rem;
}

.control-field__actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}


</style>
<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore, updateSelectedRefreshStartStage } from '../store.js';
import { formatDateTime } from '../lib/dashboard-utils.js';

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

const emit = defineEmits(['refresh', 'export', 'openGuides']);

const loading = computed(() => dashboardStore.loading);
const refreshing = computed(() => dashboardStore.refreshing);
const refreshStatus = computed(() => dashboardStore.refreshStatus);
const exporting = computed(() => dashboardStore.exporting);
const snapshot = computed(() => dashboardStore.snapshot);
const lastUpdatedAt = computed(() => dashboardStore.lastUpdatedAt);
const selectedRefreshStartStage = computed({
  get: () => dashboardStore.selectedRefreshStartStage,
  set: (value) => {
    updateSelectedRefreshStartStage(value);
  },
});

const isBusy = computed(() => loading.value || refreshing.value || refreshStatus.value.running);

function formatRefreshStageLabel(value) {
  const normalized = String(value ?? 'full').trim().toLowerCase();
  return REFRESH_START_STAGE_OPTIONS.value.find((opt) => opt.value === normalized)?.label || t('refreshStages.full');
}

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
              <span class="pill pill--outline">{{ t('dashboardHero.guideViewer') }}</span>
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
            {{ (refreshing || refreshStatus.running)
              ? t('hero.refreshing')
              : selectedRefreshStartStage === 'full'
                ? t('hero.refreshData')
                : t('dashboardHero.runFrom', { stage: formatRefreshStageLabel(selectedRefreshStartStage) })
            }}
          </button>
          <button
            class="button button--primary"
            :disabled="exporting || !snapshot || isBusy"
            @click="emit('export')"
          >
            {{ exporting ? t('hero.exporting') : t('hero.exportReport') }}
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
</style>
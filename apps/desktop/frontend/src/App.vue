<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  dashboardStore,
  updateScope,
  updateReportDate,
  updateLlmAnalysis,
  updateLlmLoading,
  updateLlmError,
  toggleLlmPanel,
  loadDashboard as bridgeLoadDashboard,
  loadSelectedSnapshot as bridgeLoadSelectedSnapshot,
  startRefresh as bridgeStartRefresh,
  retryRefresh as bridgeRetryRefresh,
  cancelRefresh as bridgeCancelRefresh,
  exportReport as bridgeExportReport,
  analyzeWithLlm as bridgeAnalyzeWithLlm,
  runPrecloseAnalysis as bridgeRunPrecloseAnalysis,
} from './store.js';
import DashboardHero from './components/DashboardHero.vue';
import LlmAnalysisPanel from './components/LlmAnalysisPanel.vue';
import RecentReportsPanel from './components/RecentReportsPanel.vue';
import DataHealthPanel from './components/DataHealthPanel.vue';
import UsageGuidesPanel from './components/UsageGuidesPanel.vue';
import BreadthPanel from './components/BreadthPanel.vue';
import MetricCard from './components/MetricCard.vue';
import HealthStrip from './components/HealthStrip.vue';
import TimeContext from './components/TimeContext.vue';
import StatusPanel from './components/StatusPanel.vue';
import RegimePanel from './components/RegimePanel.vue';
import RotationPanel from './components/RotationPanel.vue';
import BacktestPanel from './components/BacktestPanel.vue';
import EnvironmentPanel from './components/EnvironmentPanel.vue';
import SignalsPanel from './components/SignalsPanel.vue';
import TrustSummaryPanel from './components/TrustSummaryPanel.vue';
import InsightPanel from './components/InsightPanel.vue';
import RefreshProgress from './components/RefreshProgress.vue';
import Notice from './components/Notice.vue';
import Skeleton from './components/Skeleton.vue';
import DateSelector from './components/DateSelector.vue';
import SignalDetailModal from './components/SignalDetailModal.vue';


const { t } = useI18n();

const snapshot = computed(() => dashboardStore.snapshot);
const loading = computed(() => dashboardStore.loading);
const error = computed(() => dashboardStore.error);
const exportResult = computed(() => dashboardStore.exportResult);
const startupNotice = computed(() => dashboardStore.startupNotice);

const startupNoticeResult = computed(() => {
  if (!startupNotice.value) return null;
  return {
    kind: startupNotice.value.type || 'info',
    title: t(`notice.title.${startupNotice.value.type || 'info'}`),
    message: startupNotice.value.message,
  };
});

const selectedRefreshStartStage = computed(() => dashboardStore.selectedRefreshStartStage);

const selectedSignal = ref(null);
const usageGuidesRef = ref(null);

const showLlmPanel = computed(() => dashboardStore.showLlmPanel);

// Body scroll lock - managed at App level to handle component lifecycle correctly
watch(selectedSignal, (newSignal) => {
  document.body.classList.toggle('body--signal-modal-open', Boolean(newSignal));
});

watch(showLlmPanel, (show) => {
  document.body.classList.toggle('body--llm-panel-open', show);
});

// Keyboard: ESC to close modals
function handleKeydown(event) {
  if (event.key === 'Escape') {
    if (selectedSignal.value) {
      selectedSignal.value = null;
    } else if (showLlmPanel.value) {
      toggleLlmPanel(false);
    }
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown);
});

function handleScopeChange(scope) {
  updateScope(scope);
  bridgeLoadDashboard();
}

function handleDateChange(date) {
  updateReportDate(date);
  bridgeLoadSelectedSnapshot();
}

function handleJumpToLatest() {
  const latestDate = snapshot.value?.latest_available_date;
  if (latestDate) {
    updateReportDate(latestDate);
    bridgeLoadSelectedSnapshot();
  }
}

function handleSelectSignal(signal) {
  selectedSignal.value = signal;
}

function handleCloseSignalDetail() {
  selectedSignal.value = null;
}

function handleCancelRefresh() {
  bridgeCancelRefresh();
}

function handleRetryRefresh() {
  bridgeRetryRefresh();
}

function handleResumeRefresh() {
  bridgeRetryRefresh();
}

function handleRefresh(stage) {
  bridgeStartRefresh(stage || selectedRefreshStartStage.value);
}

function handleExport() {
  bridgeExportReport();
}

function handleRunPrecloseAnalysis() {
  bridgeRunPrecloseAnalysis();
}

function handleOpenGuides() {
  if (usageGuidesRef.value) {
    usageGuidesRef.value.openUsageGuides();
  }
}

function handleOpenLlmPanel() {
  toggleLlmPanel(true);
}

function handleCloseLlmPanel() {
  toggleLlmPanel(false);
}
</script>

<template>
  <div id="vue-app" v-cloak>
    <!-- Hero section -->
    <DashboardHero
      @refresh="handleRefresh"
      @export="handleExport"
      @open-guides="handleOpenGuides"
      @run-preclose-analysis="handleRunPrecloseAnalysis"
    />

    <!-- Top section: full width -->
    <header class="dashboard-header">
      <div class="header-top">
        <DateSelector
          @update:scope="handleScopeChange"
          @update:date="handleDateChange"
          @jump-to-latest="handleJumpToLatest"
        />
        <button class="button button--secondary" @click="handleOpenLlmPanel">
          {{ t('research.openPanel') }}
        </button>
      </div>
      <RefreshProgress
        @cancel="handleCancelRefresh"
        @retry="handleRetryRefresh"
        @resume="handleResumeRefresh"
      />
      <Transition name="fade">
        <Notice v-if="error" :result="{ kind: 'error', title: t('common.dataLoadFailed'), message: error }" />
      </Transition>
      <Transition name="fade">
        <Notice v-if="exportResult" :result="exportResult" />
      </Transition>
      <Transition name="fade">
        <Notice v-if="startupNoticeResult" :result="startupNoticeResult" />
      </Transition>
      <Transition name="fade">
        <Skeleton v-if="loading" />
      </Transition>
      <TrustSummaryPanel />
      <InsightPanel />
      <HealthStrip />
    </header>

    <!-- Main grid -->
    <main class="dashboard-grid">
      <!-- Row 1: Time Context (full width, 4 metadata cards) -->
      <section class="grid-row grid-row--1">
        <TimeContext />
      </section>

      <!-- Row 2: Regime, Breadth -->
      <section class="grid-row grid-row--2">
        <RegimePanel />
        <BreadthPanel />
      </section>

      <!-- Row 3: Top Rotation (full width, limited height) -->
      <section class="grid-row grid-row--1">
        <RotationPanel class="rotation-panel--limited" />
      </section>

      <!-- Row 4: Backtest (full width) -->
      <section class="grid-row grid-row--1">
        <BacktestPanel />
      </section>

      <!-- Row 5: Signals (full width, internal buy/sale half-width) -->
      <section class="grid-row grid-row--1">
        <SignalsPanel @select-signal="handleSelectSignal" />
      </section>

      <!-- Row 6: Environment, Status -->
      <section class="grid-row grid-row--2">
        <EnvironmentPanel />
        <StatusPanel />
      </section>

      <!-- Row 7: Recent Reports -->
      <section class="grid-row grid-row--1">
        <RecentReportsPanel />
      </section>

      <!-- Row 8: Data Health -->
      <section class="grid-row grid-row--1">
        <DataHealthPanel />
      </section>
    </main>

    <!-- Usage guides full-screen viewer -->
    <UsageGuidesPanel ref="usageGuidesRef" />

    <!-- Signal detail side panel -->
    <Transition name="slide">
      <SignalDetailModal
        v-if="selectedSignal"
        :signal="selectedSignal"
        @close="handleCloseSignalDetail"
      />
    </Transition>

    <!-- LLM analysis side panel -->
    <Transition name="slide">
      <LlmAnalysisPanel
        v-if="showLlmPanel"
        @close="handleCloseLlmPanel"
        @reanalyze="handleAnalyzeWithLlm"
      />
    </Transition>
  </div>
</template>

<style scoped>
#vue-app {
  width: min(calc(100% - (var(--space-6) * 2)), var(--container-width));
  min-height: 100vh;
  padding: var(--space-5) var(--space-6);
  margin: 0 auto;
}

.dashboard-header {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  margin-bottom: var(--space-5);
}

.header-top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-4);
}

.header-top > :first-child {
  flex: 1;
  min-width: 0;
}

.dashboard-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.grid-row {
  display: grid;
  gap: var(--space-4);
}

.grid-row--1 {
  grid-template-columns: 1fr;
}

.grid-row--3 {
  grid-template-columns: repeat(3, 1fr);
}

.grid-row--2 {
  grid-template-columns: repeat(2, 1fr);
}

/* Rotation panel height limit */
:deep(.rotation-panel--limited) {
  max-height: 400px;
  overflow-y: auto;
}

/* Responsive breakpoints */
@media (max-width: 1080px) {
  .grid-row--3 {
    grid-template-columns: repeat(2, 1fr);
  }

  .grid-row--2 {
    grid-template-columns: 1fr;
  }

  #vue-app {
    width: min(calc(100% - (var(--space-5) * 2)), var(--container-width));
  }
}

@media (max-width: 720px) {
  .grid-row--3 {
    grid-template-columns: 1fr;
  }

  #vue-app {
    padding: var(--space-3);
    width: 100%;
  }
}

/* Transition animations */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.slide-enter-active,
.slide-leave-active {
  transition: transform 0.3s ease;
}

.slide-enter-from,
.slide-leave-to {
  transform: translateX(100%);
}
</style>

<script setup>
import { ref, computed } from 'vue';
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
  toggleStrategyPerspectives,
} from './store.js';
import TopStatusBar from './components/TopStatusBar.vue';
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
import ExecutionResultsPanel from './components/ExecutionResultsPanel.vue';
import StrategyPerspectivesPanel from './components/StrategyPerspectivesPanel.vue';
import RefreshProgress from './components/RefreshProgress.vue';
import Notice from './components/Notice.vue';
import Skeleton from './components/Skeleton.vue';


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

const usageGuidesRef = ref(null);

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
  bridgeStartRefresh(stage);
}

function handleExport() {
  bridgeExportReport();
}

function handleRunPrecloseAnalysis() {
  bridgeRunPrecloseAnalysis();
}

function handleChangeScope(scope) {
  updateScope(scope);
  bridgeLoadDashboard();
}

function handleOpenGuides() {
  if (usageGuidesRef.value) {
    usageGuidesRef.value.openUsageGuides();
  }
}

function handleOpenPerspectives() {
  toggleStrategyPerspectives(true);
}
</script>

<template>
  <div id="vue-app" v-cloak>
    <!-- Top Status Bar -->
    <TopStatusBar
      @refresh="handleRefresh"
      @export="handleExport"
      @open-guides="handleOpenGuides"
      @run-preclose-analysis="handleRunPrecloseAnalysis"
      @open-perspectives="handleOpenPerspectives"
      @change-scope="handleChangeScope"
    />

    <!-- Main 2-column layout -->
    <div class="dashboard-layout">
      <!-- Left column: Quant Engine (70%) -->
      <main class="dashboard-main">
        <!-- Refresh Progress + Notices -->
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

        <!-- System Overview -->
        <TrustSummaryPanel />
        <section class="grid-row grid-row--2">
          <InsightPanel />
          <HealthStrip />
        </section>
        <TimeContext />

        <!-- Quant Engine Grid -->
        <div class="dashboard-grid">
          <!-- Regime + Breadth -->
          <section class="grid-row grid-row--2">
            <RegimePanel />
            <BreadthPanel />
          </section>

          <!-- Rotation -->
          <section class="grid-row grid-row--1">
            <RotationPanel />
          </section>

          <!-- Backtest -->
          <section class="grid-row grid-row--1">
            <BacktestPanel />
          </section>

          <!-- Signals -->
          <section class="grid-row grid-row--1">
            <SignalsPanel />
          </section>

          <!-- Execution Results -->
          <section class="grid-row grid-row--1">
            <ExecutionResultsPanel />
          </section>

          <!-- Environment + Status -->
          <section class="grid-row grid-row--2">
            <EnvironmentPanel />
            <StatusPanel />
          </section>

          <!-- Recent Reports -->
          <section class="grid-row grid-row--1">
            <RecentReportsPanel />
          </section>

          <!-- Data Health -->
          <section class="grid-row grid-row--1">
            <DataHealthPanel />
          </section>
        </div>
      </main>

      <!-- Right column: Research (30%) -->
      <aside class="dashboard-research">
        <LlmAnalysisPanel />
      </aside>
    </div>

    <!-- Usage guides full-screen viewer -->
    <UsageGuidesPanel ref="usageGuidesRef" />

    <!-- Strategy perspectives research overlay -->
    <StrategyPerspectivesPanel />
  </div>
</template>

<style scoped>
#vue-app {
  width: 100%;
  min-height: 100vh;
  padding: 0;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
}

.dashboard-layout {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-4) var(--space-5);
  flex: 1;
  align-items: flex-start;
}

.dashboard-main {
  flex: 0 0 70%;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.dashboard-research {
  flex: 0 0 30%;
  min-width: 0;
  position: sticky;
  top: calc(3.5rem + var(--space-4));
  height: calc(100vh - 3.5rem - var(--space-4) * 2 - 2px);
}

.dashboard-grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.grid-row {
  display: grid;
  gap: var(--space-4);
}

.grid-row--1 {
  grid-template-columns: 1fr;
}

.grid-row--2 {
  grid-template-columns: repeat(2, 1fr);
}

/* Responsive breakpoints */
@media (max-width: 1080px) {
  .dashboard-layout {
    flex-direction: column;
  }

  .dashboard-main {
    flex: 1 1 auto;
    width: 100%;
  }

  .dashboard-research {
    flex: 1 1 auto;
    width: 100%;
    position: static;
    height: auto;
    max-height: 600px;
  }

  .grid-row--2 {
    grid-template-columns: 1fr;
  }

  #vue-app {
    width: min(calc(100% - (var(--space-5) * 2)), var(--container-width));
  }
}

@media (max-width: 720px) {
  .dashboard-layout {
    padding: var(--space-3);
  }

  #vue-app {
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

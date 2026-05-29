<script setup>
import { ref, computed, watch } from 'vue';
import {
  dashboardStore,
  updateScope,
  updateReportDate,
  loadDashboard as bridgeLoadDashboard,
  loadSelectedSnapshot as bridgeLoadSelectedSnapshot,
  startRefresh as bridgeStartRefresh,
  retryRefresh as bridgeRetryRefresh,
  cancelRefresh as bridgeCancelRefresh,
} from './store.js';
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
import RefreshProgress from './components/RefreshProgress.vue';
import Notice from './components/Notice.vue';
import Skeleton from './components/Skeleton.vue';
import DateSelector from './components/DateSelector.vue';
import SignalDetailModal from './components/SignalDetailModal.vue';

const snapshot = computed(() => dashboardStore.snapshot);
const loading = computed(() => dashboardStore.loading);
const error = computed(() => dashboardStore.error);
const exportResult = computed(() => dashboardStore.exportResult);

const selectedSignal = ref(null);

// Body scroll lock - managed at App level to handle component lifecycle correctly
watch(selectedSignal, (newSignal) => {
  document.body.classList.toggle('body--signal-modal-open', Boolean(newSignal));
});

function handleScopeChange(scope) {
  updateScope(scope);
  // Trigger data reload via event bridge
  bridgeLoadDashboard();
}

function handleDateChange(date) {
  updateReportDate(date);
  // Trigger snapshot reload via event bridge
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
</script>

<template>
  <div id="vue-app" v-cloak>
    <!-- Top section: full width -->
    <header class="dashboard-header">
      <DateSelector
        @update:scope="handleScopeChange"
        @update:date="handleDateChange"
        @jump-to-latest="handleJumpToLatest"
      />
      <RefreshProgress
        @cancel="handleCancelRefresh"
        @retry="handleRetryRefresh"
        @resume="handleResumeRefresh"
      />
      <Transition name="fade">
        <Notice v-if="error" :result="{ kind: 'error', title: 'Data load failed', message: error }" />
      </Transition>
      <Transition name="fade">
        <Notice v-if="exportResult" :result="exportResult" />
      </Transition>
      <Transition name="fade">
        <Skeleton v-if="loading" />
      </Transition>
      <TrustSummaryPanel />
      <HealthStrip />
    </header>

    <!-- Main grid -->
    <main class="dashboard-grid">
      <!-- Row 1: Regime, Breadth, Time Context -->
      <section class="grid-row grid-row--3">
        <RegimePanel />
        <BreadthPanel />
        <TimeContext />
      </section>

      <!-- Row 2: Top Rotation (full width, limited height) -->
      <section class="grid-row grid-row--1">
        <RotationPanel class="rotation-panel--limited" />
      </section>

      <!-- Row 3: Signals + Backtest (side by side) -->
      <section class="grid-row grid-row--2">
        <SignalsPanel @select-signal="handleSelectSignal" />
        <BacktestPanel />
      </section>

      <!-- Row 4: Environment, Status -->
      <section class="grid-row grid-row--2">
        <EnvironmentPanel />
        <StatusPanel />
      </section>
    </main>

    <!-- Signal detail side panel -->
    <Transition name="slide">
      <SignalDetailModal
        v-if="selectedSignal"
        :signal="selectedSignal"
        @close="handleCloseSignalDetail"
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

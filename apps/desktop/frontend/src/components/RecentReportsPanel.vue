<script setup>
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import {
  dashboardStore,
  updateScope,
  updateReportDate,
  loadDashboard as bridgeLoadDashboard,
  loadSelectedSnapshot as bridgeLoadSelectedSnapshot,
} from '../store.js';
import {
  formatDate,
  formatInteger,
  formatReportType,
  formatScopeLabel,
  getErrorMessage,
  getRecentReportScope,
  normalizeScope,
} from '../lib/dashboard-utils.js';
import Notice from './Notice.vue';

const { t } = useI18n();

const COMMANDS = {
  openReportArtifact: 'open_report_artifact',
};

const actionResult = ref(null);
const showAllModal = ref(false);

const recentReports = computed(() => dashboardStore.recentReports);
const loading = computed(() => dashboardStore.loading);
const refreshing = computed(() => dashboardStore.refreshing);
const refreshStatus = computed(() => dashboardStore.refreshStatus);

const totalReports = computed(() => recentReports.value.length);

const displayReports = computed(() => {
  if (showAllModal.value) return recentReports.value;
  return recentReports.value.slice(0, 3);
});

const hasMore = computed(() => totalReports.value > 3);

const isBusy = computed(() => loading.value || refreshing.value || refreshStatus.value.running);

function isCurrentSnapshot(item, scope) {
  if (!scope) return false;
  const currentScope = normalizeScope(dashboardStore.selectedScope);
  const currentDate = dashboardStore.selectedReportDate || dashboardStore.snapshot?.report_date || '';
  return currentScope === scope && currentDate === item.report_date;
}

function clearActionResult() {
  actionResult.value = null;
}

async function openArtifact(item) {
  if (!item?.artifact_path) return;

  try {
    await invoke(COMMANDS.openReportArtifact, { artifactPath: item.artifact_path });
    actionResult.value = {
      kind: 'success',
      title: t('recentReports.artifactOpened'),
      message: t('recentReports.openedMessage', { type: formatReportType(item.report_type), date: item.report_date }),
      output_path: item.artifact_path,
    };
  } catch (error) {
    actionResult.value = {
      kind: 'error',
      title: t('recentReports.openFailed'),
      message: getErrorMessage(error),
    };
  }
}

async function copyArtifactPath(item) {
  if (!item?.artifact_path) return;

  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(item.artifact_path);
    } else {
      copyTextFallback(item.artifact_path);
    }
    actionResult.value = {
      kind: 'success',
      title: t('recentReports.pathCopied'),
      message: t('recentReports.copiedMessage', { type: formatReportType(item.report_type), date: item.report_date }),
      output_path: item.artifact_path,
    };
  } catch (error) {
    actionResult.value = {
      kind: 'error',
      title: t('recentReports.copyFailed'),
      message: getErrorMessage(error),
    };
  }
}

function copyTextFallback(text) {
  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.setAttribute('readonly', 'true');
  textarea.style.position = 'fixed';
  textarea.style.opacity = '0';
  textarea.style.pointerEvents = 'none';
  document.body.appendChild(textarea);
  textarea.select();
  textarea.setSelectionRange(0, textarea.value.length);
  const success = document.execCommand('copy');
  document.body.removeChild(textarea);
  if (!success) {
    throw new Error('Clipboard copy is unavailable in this runtime.');
  }
}

async function openSnapshot(item) {
  const nextScope = getRecentReportScope(item?.report_type);
  if (!item?.report_date || !nextScope) return;
  if (isBusy.value) return;

  const currentScope = normalizeScope(dashboardStore.selectedScope);
  const currentDate = dashboardStore.selectedReportDate || dashboardStore.snapshot?.report_date || '';

  if (currentScope === nextScope && currentDate === item.report_date) {
    actionResult.value = {
      kind: 'success',
      title: t('recentReports.snapshotAlreadyOpen'),
      message: t('recentReports.alreadyViewing', { type: formatReportType(item.report_type), date: item.report_date }),
    };
    return;
  }

  clearActionResult();
  updateReportDate(item.report_date);
  dashboardStore.exportResult = null;

  if (currentScope === nextScope) {
    await bridgeLoadSelectedSnapshot();
    return;
  }

  updateScope(nextScope);
  dashboardStore.snapshot = null;
  await bridgeLoadDashboard();
}

function openAllReportsModal() {
  showAllModal.value = true;
}

function closeAllReportsModal() {
  showAllModal.value = false;
}
</script>

<template>
  <article class="panel panel--soft">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('recentReports.eyebrow') }}</p>
        <h2>{{ t('recentReports.title') }}</h2>
        <p class="panel__lede">
          {{ t('recentReports.lede') }}
        </p>
      </div>
      <div class="panel__header-actions">
        <span class="panel__meta">{{ t('recentReports.latestExports', { count: formatInteger(totalReports) }) }}</span>
        <button
          v-if="hasMore"
          class="button button--secondary button--compact"
          @click="openAllReportsModal"
        >
          {{ t('recentReports.viewAll', { count: formatInteger(totalReports) }) }}
        </button>
      </div>
    </div>

    <Notice v-if="actionResult" :result="actionResult" />

    <div v-if="displayReports.length" class="report-history" aria-label="Recent report history">
      <article
        v-for="(item, index) in displayReports"
        :key="`${item.report_type}::${item.report_date}::${item.artifact_path}`"
        class="report-history__item"
      >
        <div class="report-history__row">
          <span class="pill pill--outline">{{ formatReportType(item.report_type) }}</span>
          <span class="report-history__date">{{ formatDate(item.report_date) }}</span>
        </div>
        <div class="report-history__meta-row">
          <span class="panel__meta">
            {{ getRecentReportScope(item.report_type)
              ? t('recentReports.analysisScope', { scope: formatScopeLabel(getRecentReportScope(item.report_type)) })
              : t('recentReports.artifactOnly')
            }}
          </span>
          <span class="panel__meta">
            {{ getRecentReportScope(item.report_type) && isCurrentSnapshot(item, getRecentReportScope(item.report_type))
              ? t('recentReports.currentDashboardView')
              : getRecentReportScope(item.report_type)
                ? t('recentReports.snapshotJumpAvailable')
                : t('recentReports.artifactActionsAvailable')
            }}
          </span>
        </div>
        <p class="report-history__path"><code>{{ item.artifact_path }}</code></p>
        <div class="report-history__actions">
          <button
            v-if="getRecentReportScope(item.report_type)"
            class="button button--secondary button--compact"
            :disabled="isBusy || isCurrentSnapshot(item, getRecentReportScope(item.report_type))"
            @click="openSnapshot(item)"
          >
            {{ isCurrentSnapshot(item, getRecentReportScope(item.report_type)) ? t('recentReports.currentView') : t('recentReports.openSnapshot') }}
          </button>
          <button
            class="button button--secondary button--compact"
            @click="openArtifact(item)"
          >
            {{ t('recentReports.openArtifact') }}
          </button>
          <button
            class="button button--secondary button--compact"
            @click="copyArtifactPath(item)"
          >
            {{ t('recentReports.copyPath') }}
          </button>
        </div>
      </article>
    </div>

    <div v-else class="empty-state empty-state--compact">
      <p>{{ t('recentReports.noReports') }}</p>
    </div>
  </article>

  <!-- View All Modal -->
  <Transition name="fade">
    <div v-if="showAllModal" class="reports-modal-overlay" @click.self="closeAllReportsModal">
      <div class="reports-modal">
        <div class="reports-modal__header">
          <h2>{{ t('recentReports.allReports') }}</h2>
          <button class="reports-modal__close" aria-label="Close" @click="closeAllReportsModal">
            &times;
          </button>
        </div>
        <div class="reports-modal__content">
          <div v-if="recentReports.length" class="report-history" aria-label="All recent reports">
            <article
              v-for="item in recentReports"
              :key="`modal::${item.report_type}::${item.report_date}::${item.artifact_path}`"
              class="report-history__item"
            >
              <div class="report-history__row">
                <span class="pill pill--outline">{{ formatReportType(item.report_type) }}</span>
                <span class="report-history__date">{{ formatDate(item.report_date) }}</span>
              </div>
              <div class="report-history__meta-row">
                <span class="panel__meta">
                  {{ getRecentReportScope(item.report_type)
                    ? t('recentReports.analysisScope', { scope: formatScopeLabel(getRecentReportScope(item.report_type)) })
                    : t('recentReports.artifactOnly')
                  }}
                </span>
                <span class="panel__meta">
                  {{ getRecentReportScope(item.report_type) && isCurrentSnapshot(item, getRecentReportScope(item.report_type))
                    ? t('recentReports.currentDashboardView')
                    : getRecentReportScope(item.report_type)
                      ? t('recentReports.snapshotJumpAvailable')
                      : t('recentReports.artifactActionsAvailable')
                  }}
                </span>
              </div>
              <p class="report-history__path"><code>{{ item.artifact_path }}</code></p>
              <div class="report-history__actions">
                <button
                  v-if="getRecentReportScope(item.report_type)"
                  class="button button--secondary button--compact"
                  :disabled="isBusy || isCurrentSnapshot(item, getRecentReportScope(item.report_type))"
                  @click="openSnapshot(item)"
                >
                  {{ isCurrentSnapshot(item, getRecentReportScope(item.report_type)) ? t('recentReports.currentView') : t('recentReports.openSnapshot') }}
                </button>
                <button
                  class="button button--secondary button--compact"
                  @click="openArtifact(item)"
                >
                  {{ t('recentReports.openArtifact') }}
                </button>
                <button
                  class="button button--secondary button--compact"
                  @click="copyArtifactPath(item)"
                >
                  {{ t('recentReports.copyPath') }}
                </button>
              </div>
            </article>
          </div>
          <div v-else class="empty-state empty-state--compact">
            <p>{{ t('recentReports.noReports') }}</p>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
/* Uses global styles from styles.css */
.panel {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--panel-padding);
}

.panel--soft {
  background: var(--panel-bg-secondary);
}

.panel__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-4);
  margin-bottom: var(--space-4);
}

.panel__lede {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  margin-top: var(--space-1);
}

.panel__header-actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.panel__meta {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.report-history {
  display: grid;
  gap: var(--space-3);
  margin-top: var(--space-4);
}

.report-history__item {
  padding: var(--space-3);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  background: linear-gradient(180deg, var(--color-surface-soft), rgba(19, 23, 28, 0.9));
}

.report-history__row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
  margin-bottom: var(--space-2);
}

.report-history__meta-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
  margin-bottom: var(--space-2);
}

.report-history__date {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
}

.report-history__path {
  margin: 0;
  color: var(--text-secondary);
  line-height: var(--line-height-body);
}

.report-history__path code {
  display: block;
  font-family: monospace;
  font-size: var(--font-size-label);
  padding: var(--space-1) var(--space-2);
  background: var(--panel-bg);
  border-radius: var(--space-1);
  overflow-wrap: anywhere;
}

.report-history__actions {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
  margin-top: var(--space-3);
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: var(--space-1) var(--space-3);
  border-radius: var(--space-3);
  font-size: var(--font-size-label);
  font-weight: 500;
}

.pill--outline {
  border: 1px solid var(--pill-outline-border);
  color: var(--text-secondary);
  background: transparent;
}

.eyebrow {
  font-size: var(--font-size-label);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
  margin: 0 0 var(--space-2);
}

h2 {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.empty-state {
  padding: var(--space-5);
  text-align: center;
  color: var(--text-secondary);
}

.empty-state--compact {
  padding: var(--space-4);
}

.reports-modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--color-overlay);
  z-index: 40;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-5);
}

.reports-modal {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  width: 100%;
  max-width: 800px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-strong);
}

.reports-modal__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--panel-border);
}

.reports-modal__header h2 {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.reports-modal__close {
  background: none;
  border: none;
  font-size: 1.5rem;
  color: var(--text-secondary);
  cursor: pointer;
  padding: var(--space-1) var(--space-2);
  line-height: 1;
}

.reports-modal__close:hover {
  color: var(--text-primary);
}

.reports-modal__content {
  overflow-y: auto;
  padding: var(--space-4) var(--space-5);
  flex: 1;
}

.button {
  min-height: 3rem;
  border: 1px solid transparent;
  border-radius: 999px;
  padding: 0 var(--space-5);
  transition: transform 180ms ease, border-color 180ms ease, background-color 180ms ease, opacity 180ms ease;
  cursor: pointer;
  font: inherit;
}

.button:hover:not(:disabled) {
  transform: translateY(-1px);
}

.button:disabled {
  opacity: 0.55;
  cursor: wait;
}

.button--secondary {
  background: var(--color-surface-raised, rgba(255, 255, 255, 0.04));
  border-color: var(--color-border-strong, rgba(228, 213, 183, 0.24));
  color: var(--text-primary);
}

.button--compact {
  min-height: 2.5rem;
  padding: 0 var(--space-4);
}

/* Fade transition for modal */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>

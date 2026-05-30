<script setup>
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { dashboardStore, updateRecentReports } from '../store.js';
import {
  formatCanonicalAdjustment,
  formatDate,
  formatDateRange,
  formatDateTime,
  formatFallbackState,
  formatInteger,
  formatNumber,
  getErrorMessage,
  getFlaggedMacroSources,
  getFlaggedSymbols,
  healthTone,
  normalizeRecentReports,
  prettifyToken,
} from '../lib/dashboard-utils.js';
import MetricCard from './MetricCard.vue';
import Notice from './Notice.vue';

const { t } = useI18n();

const COMMANDS = {
  dataHealthSummary: 'data_health_summary',
  exportDataHealthReport: 'export_data_health_report',
};

const CACHE_MS = 5 * 60 * 1000;

// Local reactive state
const dataHealth = ref(null);
const dataHealthLoading = ref(false);
const dataHealthError = ref('');
const dataHealthFetchedAt = ref(null);
const dataHealthExporting = ref(false);
const dataHealthExportResult = ref(null);

const loading = computed(() => dashboardStore.loading);
const refreshing = computed(() => dashboardStore.refreshing);
const refreshStatus = computed(() => dashboardStore.refreshStatus);

const summary = computed(() => dataHealth.value);

const exportDisabled = computed(() =>
  loading.value || refreshing.value || refreshStatus.value.running ||
  dataHealthLoading.value || dataHealthExporting.value || !summary.value
);

const refreshHealthDisabled = computed(() =>
  dataHealthLoading.value || refreshing.value || refreshStatus.value.running
);

const healthStatusMeta = computed(() => {
  if (dataHealthLoading.value) return t('dataHealth.refreshingBackground');
  if (dataHealthError.value) return t('dataHealth.healthRefreshIssue', { error: dataHealthError.value });
  if (summary.value) return t('dataHealth.healthStatusMeta', { time: formatDateTime(summary.value.generated_at) });
  return t('dataHealth.healthNotLoaded');
});

const flaggedSymbols = computed(() => getFlaggedSymbols(summary.value));
const flaggedMacroSources = computed(() => getFlaggedMacroSources(summary.value));

function isCacheFresh() {
  if (!dataHealth.value || !dataHealthFetchedAt.value) return false;
  const fetchedAt = new Date(dataHealthFetchedAt.value).getTime();
  if (!Number.isFinite(fetchedAt)) return false;
  return (Date.now() - fetchedAt) < CACHE_MS;
}

async function loadSummary({ force = false } = {}) {
  if (dataHealthLoading.value) return;
  if (!force && isCacheFresh()) return;

  dataHealthLoading.value = true;
  dataHealthError.value = '';

  try {
    dataHealth.value = await invoke(COMMANDS.dataHealthSummary);
    dataHealthFetchedAt.value = new Date().toISOString();
    dashboardStore.lastUpdatedAt = new Date().toISOString();
  } catch (error) {
    dataHealthError.value = getErrorMessage(error);
  } finally {
    dataHealthLoading.value = false;
  }
}

async function exportReport() {
  if (!dataHealth.value || dataHealthExporting.value) return;

  dataHealthExporting.value = true;
  dataHealthExportResult.value = null;

  try {
    const result = await invoke(COMMANDS.exportDataHealthReport);
    dataHealthExportResult.value = {
      kind: 'success',
      title: t('dataHealth.healthExported'),
      message: t('dataHealth.savedMessage', { date: result.report_date }),
      output_path: result.output_path,
      failed_items: Array.isArray(result.failed_items) ? result.failed_items : [],
    };
    if (result.output_path) {
      const currentReports = dashboardStore.recentReports;
      const updated = normalizeRecentReports(
        [
          {
            report_type: 'DATA_HEALTH_REPORT',
            report_date: result.report_date,
            artifact_path: result.output_path,
          },
          ...currentReports,
        ],
        8,
      );
      updateRecentReports(updated);
    }
  } catch (error) {
    dataHealthExportResult.value = {
      kind: 'error',
      title: t('dataHealth.exportFailed'),
      message: getErrorMessage(error),
    };
  } finally {
    dataHealthExporting.value = false;
  }
}
</script>

<template>
  <article class="panel panel--soft">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('dataHealth.eyebrow') }}</p>
        <h2>{{ t('dataHealth.title') }}</h2>
        <p class="panel__lede">{{ t('dataHealth.lede') }}</p>
      </div>
      <div class="panel__actions">
      <span v-if="summary" class="pill pill--outline">
        {{ t('dataHealth.canonical', { adjustment: formatCanonicalAdjustment(summary.canonical_adjustment) }) }}
      </span>
        <button
          class="button button--secondary button--compact"
          :disabled="refreshHealthDisabled"
          @click="loadSummary({ force: true })"
        >
          {{ dataHealthLoading ? t('dataHealth.refreshing') : (summary ? t('dataHealth.refreshHealth') : t('dataHealth.loadHealth')) }}
        </button>
        <button
          class="button button--secondary button--compact"
          :disabled="exportDisabled"
          @click="exportReport"
        >
          {{ dataHealthExporting ? t('dataHealth.exporting') : t('dataHealth.exportHealth') }}
        </button>
      </div>
    </div>

    <!-- Empty state when no summary loaded -->
    <div v-if="!summary" class="empty-state">
      <p>{{ dataHealthLoading ? t('dataHealth.refreshingBackground') : t('dataHealth.unavailable') }}</p>
    </div>

    <!-- Loaded state -->
    <template v-else>
      <div class="panel__meta-row">
        <span class="panel__meta">{{ healthStatusMeta }}</span>
        <span class="panel__meta">{{ t('dataHealth.symbolsChecked', { count: formatInteger(summary.checked_symbols) }) }}</span>
        <span class="panel__meta">{{ t('dataHealth.freshestDate', { date: formatDate(summary.freshest_market_date) }) }}</span>
      </div>

      <!-- Row 1: Healthy/Review/Critical/Checked -->
      <div class="mini-metrics">
        <MetricCard
          :label="t('dataHealth.healthy')"
          :value="formatInteger(summary.healthy_symbols)"
          :meta="t('dataHealth.clearForUse')"
          :tone="Number(summary.healthy_symbols) > 0 ? 'positive' : 'neutral'"
        />
        <MetricCard
          :label="t('dataHealth.review')"
          :value="formatInteger(summary.review_symbols)"
          :meta="t('dataHealth.needsReview')"
          :tone="Number(summary.review_symbols) > 0 ? 'neutral' : 'positive'"
        />
        <MetricCard
          :label="t('dataHealth.critical')"
          :value="formatInteger(summary.critical_symbols)"
          :meta="t('dataHealth.needsFollowup')"
          :tone="Number(summary.critical_symbols) > 0 ? 'negative' : 'neutral'"
        />
        <MetricCard
          :label="t('dataHealth.checked')"
          :value="formatInteger(summary.checked_symbols)"
          :meta="t('dataHealth.universeCoverage')"
          tone="neutral"
        />
      </div>

      <!-- Row 2: Coverage metrics -->
      <div class="mini-metrics">
        <MetricCard
          :label="t('dataHealth.latestDayCoverage')"
          :value="`${formatInteger(summary.symbols_on_freshest_market_date)}/${formatInteger(summary.checked_symbols)}`"
          :meta="t('dataHealth.symbolsWithBars')"
          :tone="summary.freshest_market_date_complete ? 'positive' : 'warning'"
        />
        <MetricCard
          :label="t('dataHealth.missingLatest')"
          :value="formatInteger(summary.symbols_missing_freshest_market_date)"
          :meta="t('dataHealth.symbolsNotUpdated')"
          :tone="Number(summary.symbols_missing_freshest_market_date) > 0 ? 'warning' : 'positive'"
        />
        <MetricCard
          :label="t('dataHealth.freshestDateComplete')"
          :value="summary.freshest_market_date_complete ? t('dataHealth.yes') : t('dataHealth.no')"
          :meta="t('dataHealth.fullCoverage')"
          :tone="summary.freshest_market_date_complete ? 'positive' : 'warning'"
        />
        <MetricCard
          :label="t('dataHealth.freshestDateLabel')"
          :value="formatDate(summary.freshest_market_date)"
          :meta="t('dataHealth.referenceDate')"
          tone="neutral"
        />
      </div>

      <!-- Row 3: Macro source metrics -->
      <div class="mini-metrics">
        <MetricCard
          :label="t('dataHealth.macroHealthy')"
          :value="formatInteger(summary.healthy_macro_sources)"
          :meta="t('dataHealth.primaryPath')"
          :tone="Number(summary.healthy_macro_sources) > 0 ? 'positive' : 'neutral'"
        />
        <MetricCard
          :label="t('dataHealth.macroReview')"
          :value="formatInteger(summary.review_macro_sources)"
          :meta="t('dataHealth.compatFallback')"
          :tone="Number(summary.review_macro_sources) > 0 ? 'neutral' : 'positive'"
        />
        <MetricCard
          :label="t('dataHealth.macroCritical')"
          :value="formatInteger(summary.critical_macro_sources)"
          :meta="t('dataHealth.macroUnavailable')"
          :tone="Number(summary.critical_macro_sources) > 0 ? 'negative' : 'neutral'"
        />
        <MetricCard
          :label="t('dataHealth.macroSources')"
          :value="formatInteger(summary.macro_sources?.length ?? 0)"
          :meta="t('dataHealth.fredCoverage')"
          tone="neutral"
        />
      </div>

      <Notice v-if="dataHealthExportResult" :result="dataHealthExportResult" />

      <!-- Macro source status table -->
      <section class="data-health__review-block">
        <div class="panel__subheader">
          <p class="panel__section-title">{{ t('dataHealth.macroSourceStatus') }}</p>
          <span class="panel__meta">
            {{ flaggedMacroSources.length
              ? t('dataHealth.sourcesDegraded', { count: formatInteger(flaggedMacroSources.length) })
              : t('dataHealth.allPrimaryPath')
            }}
          </span>
        </div>

        <div v-if="flaggedMacroSources.length" class="table-wrap">
          <table class="data-table data-table--compact">
            <thead>
              <tr>
                <th>{{ t('dataHealth.factor') }}</th>
                <th>{{ t('dataHealth.status') }}</th>
                <th>{{ t('dataHealth.transport') }}</th>
                <th>{{ t('dataHealth.coverage') }}</th>
                <th>{{ t('dataHealth.notes') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in flaggedMacroSources" :key="item.factor_name">
                <td>
                  <div class="table-symbol">
                    <strong class="data-table__symbol">{{ prettifyToken(item.factor_name) }}</strong>
                    <span class="table-symbol__meta">{{ item.source }}</span>
                  </div>
                </td>
                <td>
                  <span class="pill" :class="`pill--${healthTone(item.status)}`">{{ prettifyToken(item.status) }}</span>
                </td>
                <td>
                  <span class="pill pill--outline">{{ item.transport }}</span>
                </td>
                <td>
                  <div class="table-stack">
                    <strong>{{ t('dataHealth.rows', { count: formatInteger(item.rows) }) }}</strong>
                    <span class="table-stack__meta">{{ formatDateRange(item.first_date, item.last_date) }}</span>
                  </div>
                </td>
                <td>
                  <ul v-if="item.notes?.length" class="note-list">
                    <li v-for="note in item.notes" :key="note">{{ note }}</li>
                  </ul>
                  <span v-else class="table-stack__meta">{{ t('dataHealth.noNotes') }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-else class="empty-state empty-state--compact">
          <p>{{ t('dataHealth.allMacroPrimary') }}</p>
        </div>
      </section>

      <!-- Review queue table -->
      <section class="data-health__review-block">
        <div class="panel__subheader">
          <p class="panel__section-title">{{ t('dataHealth.reviewQueue') }}</p>
          <span class="panel__meta">
            {{ flaggedSymbols.length
              ? t('dataHealth.symbolsFlagged', { count: formatInteger(flaggedSymbols.length) })
              : t('dataHealth.noSymbolsFlagged')
            }}
          </span>
        </div>

        <div v-if="flaggedSymbols.length" class="table-wrap">
          <table class="data-table data-table--compact">
            <thead>
              <tr>
                <th>{{ t('dataHealth.symbol') }}</th>
                <th>{{ t('dataHealth.status') }}</th>
                <th>{{ t('dataHealth.coverage') }}</th>
                <th>{{ t('dataHealth.checks') }}</th>
                <th>{{ t('dataHealth.notes') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in flaggedSymbols" :key="item.symbol">
                <td>
                  <div class="table-symbol">
                    <strong class="data-table__symbol">{{ item.display_symbol || item.symbol }}</strong>
                    <span class="table-symbol__meta">
                      {{ item.name }}{{ item.display_symbol ? ` · ${item.symbol}` : '' }}
                    </span>
                  </div>
                </td>
                <td>
                  <span class="pill" :class="`pill--${healthTone(item.status)}`">{{ prettifyToken(item.status) }}</span>
                </td>
                <td>
                  <div class="table-stack">
                    <strong>{{ t('dataHealth.rows', { count: formatInteger(item.rows) }) }}</strong>
                    <span class="table-stack__meta">{{ formatDateRange(item.first_date, item.last_date) }}</span>
                  </div>
                </td>
                <td>
                  <div class="table-flags">
                    <span>{{ item.primary_provider_ok ? t('dataHealth.primaryOk') : t('dataHealth.primaryDown') }} · {{ formatFallbackState(item.fallback_provider_ok) }}</span>
                    <span>{{ t('dataHealth.gaps', { count: formatInteger(item.gap_count), days: formatInteger(item.max_gap_days) }) }}</span>
                    <span>{{ t('dataHealth.jumps', { count: formatInteger(item.suspicious_jump_count), pct: formatNumber(item.max_abs_daily_return_pct, 1) }) }}</span>
                    <span>{{ t('dataHealth.turnoverMissing', { count: formatInteger(item.missing_turnover_rows) }) }}</span>
                  </div>
                </td>
                <td>
                  <ul v-if="item.notes?.length" class="note-list">
                    <li v-for="note in item.notes" :key="note">{{ note }}</li>
                  </ul>
                  <span v-else class="table-stack__meta">{{ t('dataHealth.noNotes') }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-else class="empty-state empty-state--compact">
          <p>{{ t('dataHealth.allPassed') }}</p>
        </div>
      </section>
    </template>
  </article>
</template>

<style scoped>
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
  flex-wrap: wrap;
}

.panel__lede {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  margin-top: var(--space-1);
}

.panel__actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.panel__meta-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
  margin-bottom: var(--space-4);
}

.panel__meta {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.panel__subheader {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
  margin-bottom: var(--space-3);
}

.panel__section-title {
  margin: 0;
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  font-weight: 600;
}

.mini-metrics {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-4);
  margin-bottom: var(--space-4);
}

.data-health__review-block {
  margin-top: var(--space-5);
}

.table-wrap {
  overflow-x: auto;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
}

.data-table th,
.data-table td {
  padding: 0.9rem 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  text-align: left;
}

.data-table th {
  color: var(--text-secondary);
  font-size: var(--font-size-label);
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.data-table--compact th,
.data-table--compact td {
  vertical-align: top;
}

.data-table__symbol {
  color: var(--text-primary);
  font-weight: 600;
}

.table-symbol,
.table-stack,
.table-flags {
  display: grid;
  gap: var(--space-1);
}

.table-symbol__meta,
.table-stack__meta {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
}

.table-flags {
  color: var(--text-secondary);
}

.note-list {
  margin: 0;
  padding-left: var(--space-4);
  color: var(--text-secondary);
}

.note-list li + li {
  margin-top: var(--space-1);
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

.pill--positive {
  background: var(--tone-positive-bg);
  color: var(--tone-positive);
}

.pill--negative {
  background: var(--tone-negative-bg);
  color: var(--tone-negative);
}

.pill--neutral {
  background: var(--tone-neutral-bg);
  color: var(--tone-neutral);
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

@media (max-width: 1080px) {
  .mini-metrics {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .mini-metrics {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>

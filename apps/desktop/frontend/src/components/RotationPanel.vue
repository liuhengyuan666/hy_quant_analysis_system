<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatNumber } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const snapshot = computed(() => dashboardStore.snapshot);
const topRotation = computed(() => snapshot.value?.top_rotation || []);
const bottomRotation = computed(() => snapshot.value?.bottom_rotation || []);
const hasRotation = computed(() => topRotation.value.length > 0);
const symbolNames = computed(() => snapshot.value?.symbol_names || {});
</script>

<template>
  <article class="panel">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('rotation.eyebrow') }}</p>
        <h2>{{ t('rotation.title') }}</h2>
        <p v-if="hasRotation" class="panel__lede">
          {{ t('rotation.lede') }}
        </p>
      </div>
      <span v-if="hasRotation" class="panel__meta">{{ t('rotation.momentumRanked') }}</span>
    </div>

    <div v-if="!hasRotation" class="empty-state">
      <p>{{ t('rotation.noLeaders') }}</p>
    </div>

    <div v-else class="rotation-dual-grid">
      <section>
        <div class="panel__subheader">
          <p class="panel__section-title">{{ t('rotation.leaders') }}</p>
          <span class="panel__meta">{{ t('rotation.top5Strength') }}</span>
        </div>
        <div class="table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th>{{ t('rotation.rank') }}</th>
                <th>{{ t('rotation.symbol') }}</th>
                <th>{{ t('rotation.momentum') }}</th>
                <th>{{ t('rotation.rs20') }}</th>
                <th>{{ t('rotation.rs60') }}</th>
                <th>{{ t('rotation.rs120') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in topRotation" :key="item.rank">
                <td>#{{ item.rank }}</td>
                <td class="data-table__symbol tooltip-container">
                  {{ item.symbol }}
                  <span v-if="symbolNames[item.symbol]" class="tooltip">{{ symbolNames[item.symbol] }}</span>
                </td>
                <td>{{ formatNumber(item.momentum_score, 2) }}</td>
                <td>{{ formatNumber(item.rs_20, 2) }}</td>
                <td>{{ formatNumber(item.rs_60, 2) }}</td>
                <td>{{ formatNumber(item.rs_120, 2) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section>
        <div class="panel__subheader">
          <p class="panel__section-title">{{ t('rotation.laggards') }}</p>
          <span class="panel__meta">{{ t('rotation.bottom5Momentum') }}</span>
        </div>
        <div class="table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th>{{ t('rotation.rank') }}</th>
                <th>{{ t('rotation.symbol') }}</th>
                <th>{{ t('rotation.momentum') }}</th>
                <th>{{ t('rotation.rs20') }}</th>
                <th>{{ t('rotation.rs60') }}</th>
                <th>{{ t('rotation.rs120') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in bottomRotation" :key="item.rank">
                <td>#{{ item.rank }}</td>
                <td class="data-table__symbol tooltip-container">
                  {{ item.symbol }}
                  <span v-if="symbolNames[item.symbol]" class="tooltip">{{ symbolNames[item.symbol] }}</span>
                </td>
                <td>{{ formatNumber(item.momentum_score, 2) }}</td>
                <td>{{ formatNumber(item.rs_20, 2) }}</td>
                <td>{{ formatNumber(item.rs_60, 2) }}</td>
                <td>{{ formatNumber(item.rs_120, 2) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>
  </article>
</template>

<style scoped>
.panel {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--panel-padding);
}

.panel__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--space-4);
}

.panel__lede {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  margin-top: var(--space-1);
}

.panel__meta {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

.panel__subheader {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-3);
}

.panel__section-title {
  font-weight: 600;
  color: var(--text-primary);
}

.rotation-dual-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-5);
}

@media (max-width: 720px) {
  .rotation-dual-grid {
    grid-template-columns: 1fr;
  }
}

.table-wrap {
  overflow-x: auto;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-meta);
}

.data-table th {
  text-align: left;
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--panel-border);
  color: var(--text-secondary);
  font-weight: 500;
  font-size: var(--font-size-label);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.data-table td {
  padding: var(--space-2) var(--space-3);
  border-bottom: 1px solid var(--panel-border);
  color: var(--text-primary);
}

.data-table__symbol {
  font-weight: 600;
}

.tooltip-container {
  position: relative;
  cursor: help;
}

.tooltip-container .tooltip {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--panel-radius);
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  color: var(--text-primary);
  font-size: var(--font-size-meta);
  font-weight: 500;
  white-space: nowrap;
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transition: opacity var(--transition-base), visibility var(--transition-base);
  z-index: 10;
  box-shadow: var(--shadow-soft);
}

.tooltip-container:hover .tooltip {
  opacity: 1;
  visibility: visible;
}

.eyebrow {
  font-size: var(--font-size-label);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-secondary);
}

h2 {
  margin: var(--space-1) 0 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.empty-state {
  padding: var(--space-5);
  text-align: center;
  color: var(--text-secondary);
}
</style>

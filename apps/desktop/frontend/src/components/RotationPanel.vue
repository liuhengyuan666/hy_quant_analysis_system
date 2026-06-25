<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const snapshot = computed(() => dashboardStore.snapshot);
const topRotation = computed(() => snapshot.value?.top_rotation || []);
const bottomRotation = computed(() => snapshot.value?.bottom_rotation || []);
const symbolNames = computed(() => snapshot.value?.symbol_names || {});

const hasRotation = computed(() => topRotation.value.length > 0 || bottomRotation.value.length > 0);

const maxMomentum = computed(() => {
  const all = [...topRotation.value, ...bottomRotation.value];
  const datasetMax = Math.max(...all.map(i => Math.abs(i.momentum_score || 0)));
  // 保底 100，确保 45/100 的真实比例显示；若数据超出 100 则自动扩展
  return Math.max(datasetMax, 100);
});

const leadersData = computed(() => {
  return [...topRotation.value]
    .sort((a, b) => (b.momentum_score || 0) - (a.momentum_score || 0))
    .map(item => ({
      symbol: item.symbol,
      name: symbolNames.value[item.symbol] || item.symbol,
      value: item.momentum_score || 0,
      rs20: item.rs_20,
      rs60: item.rs_60,
      rs120: item.rs_120,
      percent: Math.min((Math.abs(item.momentum_score || 0) / maxMomentum.value) * 100, 100),
    }));
});

const laggardsData = computed(() => {
  return [...bottomRotation.value]
    .sort((a, b) => (a.momentum_score || 0) - (b.momentum_score || 0))
    .map(item => ({
      symbol: item.symbol,
      name: symbolNames.value[item.symbol] || item.symbol,
      value: item.momentum_score || 0,
      rs20: item.rs_20,
      rs60: item.rs_60,
      rs120: item.rs_120,
      percent: Math.min((Math.abs(item.momentum_score || 0) / maxMomentum.value) * 100, 100),
    }));
});

function buildTooltip(item) {
  return `${item.name} (${item.symbol})\nMomentum: ${item.value > 0 ? '+' : ''}${item.value.toFixed(1)}\nRS20: ${item.rs20?.toFixed(1) || '-'} | RS60: ${item.rs60?.toFixed(1) || '-'} | RS120: ${item.rs120?.toFixed(1) || '-'}`;
}
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

    <div v-else class="rotation-dual-charts">
      <section class="rotation-chart-section">
        <div class="chart-header">
          <span class="chart-header__icon">▲</span>
          <span class="chart-header__title">{{ t('rotation.leaders') }}</span>
          <span class="chart-header__meta">{{ t('rotation.top5Strength') }}</span>
        </div>
        <div class="rotation-list">
          <div v-for="item in leadersData" :key="item.symbol" class="rotation-item">
            <div class="rotation-item__left">
              <span class="rotation-item__code">{{ item.symbol }}</span>
              <span class="rotation-item__name">{{ item.name }}</span>
            </div>
            <div class="rotation-item__bar-wrap">
              <div class="rotation-item__bar-track">
                <div class="rotation-item__bar-fill rotation-item__bar-fill--positive" :style="{ width: item.percent + '%' }"></div>
              </div>
            </div>
            <div class="rotation-item__rs">
              <span class="rs-label">20</span>
              <span class="rs-value" :class="{ 'rs-value--strong': item.rs20 >= 50 }">{{ item.rs20?.toFixed(1) || '-' }}</span>
              <span class="rs-label">60</span>
              <span class="rs-value" :class="{ 'rs-value--strong': item.rs60 >= 50 }">{{ item.rs60?.toFixed(1) || '-' }}</span>
              <span class="rs-label">120</span>
              <span class="rs-value" :class="{ 'rs-value--strong': item.rs120 >= 50 }">{{ item.rs120?.toFixed(1) || '-' }}</span>
            </div>
            <div class="rotation-item__score">{{ item.value.toFixed(1) }}</div>
            <div class="custom-tooltip">
              <div class="tooltip-title">{{ item.name }} <span class="tooltip-symbol">({{ item.symbol }})</span></div>
              <div class="tooltip-divider"></div>
              <div class="tooltip-row">
                <span class="tooltip-key">Momentum</span>
                <span class="tooltip-value" :class="{ 'tooltip-value--positive': item.value > 0 }">
                  {{ item.value > 0 ? '+' : '' }}{{ item.value.toFixed(1) }}
                </span>
              </div>
              <div class="tooltip-row">
                <span class="tooltip-key">RS20</span>
                <span class="tooltip-value">{{ item.rs20?.toFixed(1) || '-' }}</span>
              </div>
              <div class="tooltip-row">
                <span class="tooltip-key">RS60</span>
                <span class="tooltip-value">{{ item.rs60?.toFixed(1) || '-' }}</span>
              </div>
              <div class="tooltip-row">
                <span class="tooltip-key">RS120</span>
                <span class="tooltip-value">{{ item.rs120?.toFixed(1) || '-' }}</span>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section class="rotation-chart-section">
        <div class="chart-header">
          <span class="chart-header__icon">▼</span>
          <span class="chart-header__title">{{ t('rotation.laggards') }}</span>
          <span class="chart-header__meta">{{ t('rotation.bottom5Momentum') }}</span>
        </div>
        <div class="rotation-list">
          <div v-for="item in laggardsData" :key="item.symbol" class="rotation-item">
            <div class="rotation-item__left">
              <span class="rotation-item__code">{{ item.symbol }}</span>
              <span class="rotation-item__name">{{ item.name }}</span>
            </div>
            <div class="rotation-item__bar-wrap">
              <div class="rotation-item__bar-track">
                <div class="rotation-item__bar-fill rotation-item__bar-fill--negative" :style="{ width: item.percent + '%' }"></div>
              </div>
            </div>
            <div class="rotation-item__rs">
              <span class="rs-label">20</span>
              <span class="rs-value" :class="{ 'rs-value--weak': item.rs20 <= 30 }">{{ item.rs20?.toFixed(1) || '-' }}</span>
              <span class="rs-label">60</span>
              <span class="rs-value" :class="{ 'rs-value--weak': item.rs60 <= 30 }">{{ item.rs60?.toFixed(1) || '-' }}</span>
              <span class="rs-label">120</span>
              <span class="rs-value" :class="{ 'rs-value--weak': item.rs120 <= 30 }">{{ item.rs120?.toFixed(1) || '-' }}</span>
            </div>
            <div class="rotation-item__score">{{ item.value.toFixed(1) }}</div>
            <div class="custom-tooltip">
              <div class="tooltip-title">{{ item.name }} <span class="tooltip-symbol">({{ item.symbol }})</span></div>
              <div class="tooltip-divider"></div>
              <div class="tooltip-row">
                <span class="tooltip-key">Momentum</span>
                <span class="tooltip-value" :class="{ 'tooltip-value--negative': item.value < 0 }">
                  {{ item.value > 0 ? '+' : '' }}{{ item.value.toFixed(1) }}
                </span>
              </div>
              <div class="tooltip-row">
                <span class="tooltip-key">RS20</span>
                <span class="tooltip-value">{{ item.rs20?.toFixed(1) || '-' }}</span>
              </div>
              <div class="tooltip-row">
                <span class="tooltip-key">RS60</span>
                <span class="tooltip-value">{{ item.rs60?.toFixed(1) || '-' }}</span>
              </div>
              <div class="tooltip-row">
                <span class="tooltip-key">RS120</span>
                <span class="tooltip-value">{{ item.rs120?.toFixed(1) || '-' }}</span>
              </div>
            </div>
          </div>
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

.rotation-dual-charts {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-5);
}

.rotation-chart-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  min-width: 0;
}

.chart-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding-bottom: var(--space-2);
  border-bottom: 1px solid var(--panel-border);
}

.chart-header__icon {
  font-size: 0.7rem;
  color: var(--text-secondary);
}

.chart-header__title {
  font-weight: 600;
  color: var(--text-primary);
  font-size: var(--font-size-meta);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.chart-header__meta {
  margin-left: auto;
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

.rotation-list {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.rotation-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.3rem 0.5rem;
  border-radius: var(--radius-sm);
  transition: background-color var(--transition-base);
  cursor: default;
}

.rotation-item:hover {
  background: rgba(255, 255, 255, 0.04);
}

.rotation-item__left {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  min-width: 0;
  flex: 0 0 140px;
  overflow: hidden;
}

.rotation-item__code {
  font-family: var(--font-mono);
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-primary);
  flex-shrink: 0;
}

.rotation-item__name {
  font-size: 0.8rem;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rotation-item__bar-wrap {
  flex: 1 1 auto;
  min-width: 0;
}

.rotation-item__bar-track {
  height: 8px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.06);
  overflow: hidden;
}

.rotation-item__bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.5s ease;
}

.rotation-item__bar-fill--positive {
  background: var(--color-negative);
}

.rotation-item__bar-fill--negative {
  background: var(--color-positive);
}

.rotation-item__rs {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex: 0 0 auto;
  font-family: var(--font-mono);
  font-size: 0.75rem;
}

.rs-label {
  color: var(--text-secondary);
  opacity: 0.5;
  font-size: 0.65rem;
}

.rs-value {
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
  min-width: 2.8ch;
  text-align: right;
}

.rs-value--strong {
  color: var(--color-negative);
  font-weight: 600;
}

.rs-value--weak {
  color: var(--color-positive);
  font-weight: 600;
}

.rotation-item__score {
  flex: 0 0 48px;
  text-align: right;
  font-family: var(--font-mono);
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
}

.empty-state {
  padding: var(--space-5);
  text-align: center;
  color: var(--text-secondary);
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

.custom-tooltip {
  visibility: hidden;
  opacity: 0;
  position: absolute;
  z-index: 100;
  bottom: calc(100% + 0.5rem);
  left: 50%;
  transform: translateX(-50%);
  transition: opacity 0.15s ease-in-out;
  background: var(--color-surface-strong);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.6rem 0.85rem;
  width: 12rem;
  box-shadow: var(--shadow-strong);
  pointer-events: none;
}

.rotation-item:hover .custom-tooltip {
  visibility: visible;
  opacity: 1;
}

.tooltip-title {
  color: var(--text-primary);
  font-size: 1.05rem;
  font-weight: 600;
  margin-bottom: 0.15rem;
}

.tooltip-symbol {
  color: var(--text-secondary);
  font-weight: 400;
  font-size: 0.95rem;
}

.tooltip-divider {
  height: 1px;
  background: var(--color-border);
  margin: 0.4rem 0;
}

.tooltip-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 1rem;
  line-height: 1.7;
}

.tooltip-key {
  color: var(--text-secondary);
  font-size: 0.95rem;
}

.tooltip-value {
  font-family: var(--font-mono);
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.tooltip-value--positive {
  color: var(--color-negative);
}

.tooltip-value--negative {
  color: var(--color-positive);
}

@media (max-width: 1080px) {
  .rotation-dual-charts {
    grid-template-columns: 1fr;
  }
}
</style>

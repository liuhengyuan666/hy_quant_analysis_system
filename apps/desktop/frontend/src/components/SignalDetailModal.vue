<script setup>
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatNumber, prettifyToken, signalTone } from '../lib/dashboard-utils.js';

const { t } = useI18n();

const hoveredSegment = ref(null);

const props = defineProps({
  signal: {
    type: Object,
    default: null,
  },
});

const emit = defineEmits(['close']);

const reason = computed(() => props.signal?.reason || {});
const label = computed(() => props.signal?.signal_label || reason.value.label || 'N/A');
const finalScore = computed(() => props.signal?.final_score ?? reason.value.final_score ?? 0);
const symbolName = computed(() => props.signal?.name || props.signal?.symbol || 'Unknown');

const alignedStrategies = computed(() => {
  if (!Array.isArray(reason.value.aligned_strategies) || !reason.value.aligned_strategies.length) {
    return [];
  }
  return reason.value.aligned_strategies;
});

const alignmentCount = computed(() => {
  if (reason.value.alignment === null || reason.value.alignment === undefined) return 0;
  return reason.value.alignment;
});

const rotationRank = computed(() => {
  if (reason.value.rotation?.rank === null || reason.value.rotation?.rank === undefined) return 'N/A';
  return `#${reason.value.rotation.rank}`;
});

// Dimension data for stacked bar and cards
const strategy = computed(() => ({
  weight: 0.45,
  rawScore: reason.value.strategy_score ?? 0,
  contribution: reason.value.strategy_contribution ?? 0,
  bestStrategy: reason.value.best_strategy ?? 'N/A',
}));

const alignment = computed(() => ({
  weight: 0.15,
  rawScore: (reason.value.alignment ?? 0) * 20, // alignment * 20 = alignment_score
  contribution: reason.value.alignment_contribution ?? 0,
  count: alignmentCount.value,
  aligned: alignedStrategies.value,
}));

const regime = computed(() => ({
  weight: 0.20,
  trendScore: reason.value.regime?.trend_score ?? 0,
  riskScore: reason.value.regime?.risk_score ?? 0,
  combinedScore: reason.value.regime?.combined_score ?? 0,
  contribution: reason.value.regime?.contribution ?? 0,
}));

const rotation = computed(() => ({
  weight: 0.20,
  momentumScore: reason.value.rotation?.momentum_score ?? 0,
  rank: rotationRank.value,
  combinedScore: reason.value.rotation?.combined_score ?? 0,
  contribution: reason.value.rotation?.contribution ?? 0,
}));

const totalContribution = computed(() =>
  strategy.value.contribution +
  alignment.value.contribution +
  regime.value.contribution +
  rotation.value.contribution
);

const stackedBar = computed(() => {
  const total = totalContribution.value || 1;
  return [
    { label: 'Strategy', value: strategy.value.contribution, pct: (strategy.value.contribution / total) * 100, color: '#4f8cff' },
    { label: 'Alignment', value: alignment.value.contribution, pct: (alignment.value.contribution / total) * 100, color: '#16c784' },
    { label: 'Regime', value: regime.value.contribution, pct: (regime.value.contribution / total) * 100, color: '#f5b041' },
    { label: 'Rotation', value: rotation.value.contribution, pct: (rotation.value.contribution / total) * 100, color: '#5ce1e6' },
  ].filter(s => s.value > 0.01);
});

function handleBarHover(idx) { hoveredSegment.value = idx; }
function handleBarLeave() { hoveredSegment.value = null; }

const activeCard = computed(() => {
  if (hoveredSegment.value === null) return null;
  const segments = ['strategy', 'alignment', 'regime', 'rotation'];
  return segments[hoveredSegment.value] ?? null;
});
</script>

<template>
  <div class="signal-detail" role="dialog" aria-modal="true" aria-labelledby="signalDetailTitle">
    <button class="signal-detail__backdrop" type="button" :aria-label="t('signalDetail.closeSignalDetail')" @click="emit('close')"></button>

    <article class="signal-detail__panel">
      <!-- Header -->
      <header class="detail-header">
        <div class="detail-header__top">
          <div class="detail-header__symbol">
            <span class="detail-header__code">{{ signal.symbol }}</span>
            <span class="detail-header__name">{{ symbolName }}</span>
          </div>
          <span class="detail-header__badge" :class="`badge--${signalTone(label)}`">
            {{ prettifyToken(label) }}
          </span>
        </div>

        <div class="detail-header__score">
          <span class="detail-header__score-label">最终得分</span>
          <span class="detail-header__score-value">{{ formatNumber(finalScore, 2) }}</span>
        </div>

        <div class="stacked-bar">
          <div class="stacked-bar__track">
            <div
              v-for="(seg, idx) in stackedBar"
              :key="seg.label"
              class="stacked-bar__segment"
              :style="{ width: seg.pct + '%', background: seg.color }"
              @mouseenter="handleBarHover(idx)"
              @mouseleave="handleBarLeave"
            ></div>
          </div>
          <div class="stacked-bar__legend">
            <span v-for="(seg, idx) in stackedBar" :key="seg.label" class="stacked-bar__legend-item" :class="{ 'stacked-bar__legend-item--active': hoveredSegment === idx }">
              <span class="stacked-bar__dot" :style="{ background: seg.color }"></span>
              {{ seg.label }} {{ formatNumber(seg.value, 2) }}
            </span>
          </div>
        </div>
      </header>

      <!-- Dimension Cards -->
      <div class="detail-cards">
        <!-- Strategy Card -->
        <section class="dim-card" :class="{ 'dim-card--active': activeCard === 'strategy' }">
          <div class="dim-card__header">
            <div class="dim-card__title">
              <span class="dim-card__name">策略层</span>
              <span class="dim-card__weight">45% 权重</span>
            </div>
            <span class="dim-card__contribution" style="color: #4f8cff;">+{{ formatNumber(strategy.contribution, 2) }}</span>
          </div>
          <div class="dim-card__gauge">
            <div class="gauge-track">
              <div class="gauge-fill" :style="{ width: strategy.rawScore + '%' }"></div>
            </div>
            <span class="gauge-value">{{ formatNumber(strategy.rawScore, 2) }}</span>
          </div>
          <div class="dim-card__subgrid">
            <div class="submetric">
              <span class="submetric__label">最佳策略</span>
              <span class="submetric__value">{{ prettifyToken(strategy.bestStrategy) }}</span>
            </div>
            <div class="submetric">
              <span class="submetric__label">策略得分</span>
              <span class="submetric__value">{{ formatNumber(strategy.rawScore, 2) }}</span>
            </div>
          </div>
        </section>

        <!-- Alignment Card -->
        <section class="dim-card" :class="{ 'dim-card--active': activeCard === 'alignment' }">
          <div class="dim-card__header">
            <div class="dim-card__title">
              <span class="dim-card__name">模式层</span>
              <span class="dim-card__weight">15% 权重</span>
            </div>
            <span class="dim-card__contribution" style="color: #16c784;">+{{ formatNumber(alignment.contribution, 2) }}</span>
          </div>
          <div class="dim-card__gauge">
            <div class="gauge-track">
              <div class="gauge-fill" :style="{ width: alignment.rawScore + '%' }"></div>
            </div>
            <span class="gauge-value">{{ formatNumber(alignment.rawScore, 2) }}</span>
          </div>
          <div class="dim-card__subgrid">
            <div class="submetric">
              <span class="submetric__label">对齐数量</span>
              <span class="submetric__value">{{ alignment.count }}</span>
            </div>
            <div class="submetric submetric--full">
              <span class="submetric__label">对齐策略</span>
              <div class="strategy-badges">
                <span v-for="s in alignment.aligned" :key="s" class="strategy-badge">
                  {{ prettifyToken(s) }}
                </span>
                <span v-if="!alignment.aligned.length" class="submetric__value--muted">无</span>
              </div>
            </div>
          </div>
        </section>

        <!-- Regime Card -->
        <section class="dim-card" :class="{ 'dim-card--active': activeCard === 'regime' }">
          <div class="dim-card__header">
            <div class="dim-card__title">
              <span class="dim-card__name">体制层</span>
              <span class="dim-card__weight">20% 权重</span>
            </div>
            <span class="dim-card__contribution" style="color: #f5b041;">+{{ formatNumber(regime.contribution, 2) }}</span>
          </div>
          <div class="dim-card__gauge">
            <div class="gauge-track">
              <div class="gauge-fill" :style="{ width: regime.combinedScore + '%', background: '#f5b041' }"></div>
            </div>
            <span class="gauge-value">{{ formatNumber(regime.combinedScore, 2) }}</span>
          </div>
          <div class="dim-card__subgrid">
            <div class="submetric">
              <span class="submetric__label">趋势分数</span>
              <span class="submetric__value">{{ formatNumber(regime.trendScore, 2) }}</span>
            </div>
            <div class="submetric">
              <span class="submetric__label">风险分数</span>
              <span class="submetric__value">{{ formatNumber(regime.riskScore, 2) }}</span>
            </div>
          </div>
        </section>

        <!-- Rotation Card -->
        <section class="dim-card" :class="{ 'dim-card--active': activeCard === 'rotation' }">
          <div class="dim-card__header">
            <div class="dim-card__title">
              <span class="dim-card__name">轮动层</span>
              <span class="dim-card__weight">20% 权重</span>
            </div>
            <span class="dim-card__contribution" style="color: #5ce1e6;">+{{ formatNumber(rotation.contribution, 2) }}</span>
          </div>
          <div class="dim-card__gauge">
            <div class="gauge-track">
              <div class="gauge-fill" :style="{ width: rotation.combinedScore + '%', background: '#5ce1e6' }"></div>
            </div>
            <span class="gauge-value">{{ formatNumber(rotation.combinedScore, 2) }}</span>
          </div>
          <div class="dim-card__subgrid">
            <div class="submetric">
              <span class="submetric__label">动量分数</span>
              <span class="submetric__value">{{ formatNumber(rotation.momentumScore, 2) }}</span>
            </div>
            <div class="submetric">
              <span class="submetric__label">排名</span>
              <span class="submetric__value submetric__value--rank">{{ rotation.rank }}</span>
            </div>
          </div>
        </section>
      </div>

      <!-- Footer Formula -->
      <footer class="detail-footer">
        <div class="detail-footer__divider"></div>
        <p class="detail-footer__formula">
          {{ formatNumber(strategy.contribution, 2) }} (策略) +
          {{ formatNumber(alignment.contribution, 2) }} (模式) +
          {{ formatNumber(regime.contribution, 2) }} (体制) +
          {{ formatNumber(rotation.contribution, 2) }} (轮动)
          = {{ formatNumber(totalContribution, 2) }} (最终分)
        </p>
      </footer>
    </article>
  </div>
</template>

<style scoped>
.signal-detail {
  position: fixed;
  inset: 0;
  z-index: var(--layer-modal, 40);
  display: flex;
  justify-content: flex-end;
}

.signal-detail__backdrop {
  position: absolute;
  inset: 0;
  background: var(--color-overlay);
  border: none;
  cursor: pointer;
}

.signal-detail__panel {
  position: relative;
  width: 440px;
  max-width: 90vw;
  height: 100vh;
  overflow-y: auto;
  background: #0f1117;
  border-left: 1px solid #252938;
  box-shadow: -8px 0 40px rgba(0, 0, 0, 0.6);
  display: flex;
  flex-direction: column;
  padding: var(--space-5);
  font-family: var(--font-mono);
}

/* Header */
.detail-header {
  margin-bottom: var(--space-5);
  padding-bottom: var(--space-4);
  border-bottom: 1px solid #252938;
}

.detail-header__top {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--space-4);
}

.detail-header__symbol {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

.detail-header__code {
  font-size: 1.35rem;
  font-weight: 700;
  color: #d7dae0;
  letter-spacing: -0.02em;
}

.detail-header__name {
  font-size: 0.85rem;
  color: #8a909a;
}

.detail-header__badge {
  padding: 0.35rem 0.9rem;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.badge--positive {
  background: rgba(22, 199, 132, 0.15);
  color: #16c784;
  border: 1px solid rgba(22, 199, 132, 0.25);
}

.badge--negative {
  background: rgba(255, 92, 92, 0.15);
  color: #ff5c5c;
  border: 1px solid rgba(255, 92, 92, 0.25);
}

.badge--neutral {
  background: rgba(245, 176, 65, 0.15);
  color: #f5b041;
  border: 1px solid rgba(245, 176, 65, 0.25);
}

.badge--warning {
  background: rgba(245, 176, 65, 0.15);
  color: #f5b041;
  border: 1px solid rgba(245, 176, 65, 0.25);
}

.detail-header__score {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}

.detail-header__score-label {
  font-size: 0.8rem;
  color: #8a909a;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.detail-header__score-value {
  font-size: 2.2rem;
  font-weight: 700;
  color: #d7dae0;
  line-height: 1;
}

/* Stacked Bar */
.stacked-bar {
  margin-bottom: var(--space-2);
}

.stacked-bar__track {
  display: flex;
  height: 6px;
  border-radius: 3px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.06);
  margin-bottom: var(--space-2);
}

.stacked-bar__segment {
  height: 100%;
  transition: opacity 0.2s ease;
  cursor: pointer;
}

.stacked-bar__segment:hover {
  opacity: 0.85;
}

.stacked-bar__legend {
  display: flex;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.stacked-bar__legend-item {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 0.7rem;
  color: #8a909a;
  transition: color 0.2s ease;
}

.stacked-bar__legend-item--active {
  color: #d7dae0;
}

.stacked-bar__dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* Dimension Cards */
.detail-cards {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  flex: 1;
}

.dim-card {
  background: #171923;
  border: 1px solid #252938;
  border-radius: 6px;
  padding: 1rem;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.dim-card--active {
  border-color: #4f8cff;
  box-shadow: 0 0 0 1px rgba(79, 140, 255, 0.15);
}

.dim-card__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.75rem;
}

.dim-card__title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.dim-card__name {
  font-size: 0.85rem;
  font-weight: 600;
  color: #d7dae0;
}

.dim-card__weight {
  font-size: 0.65rem;
  color: #5a6270;
  background: rgba(255, 255, 255, 0.06);
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
}

.dim-card__contribution {
  font-size: 1rem;
  font-weight: 700;
}

/* Gauge */
.dim-card__gauge {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.75rem;
}

.gauge-track {
  flex: 1;
  height: 4px;
  border-radius: 2px;
  background: rgba(255, 255, 255, 0.06);
  overflow: hidden;
}

.gauge-fill {
  height: 100%;
  border-radius: 2px;
  background: #4f8cff;
  transition: width 0.5s ease;
}

.gauge-value {
  font-size: 0.8rem;
  color: #d7dae0;
  font-weight: 600;
  min-width: 3.5ch;
  text-align: right;
}

/* Sub-metrics Grid */
.dim-card__subgrid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0.5rem 1rem;
}

.submetric {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
}

.submetric--full {
  grid-column: 1 / -1;
}

.submetric__label {
  font-size: 0.75rem;
  color: #5a6270;
}

.submetric__value {
  font-size: 0.8rem;
  color: #d7dae0;
  font-weight: 500;
}

.submetric__value--muted {
  font-size: 0.8rem;
  color: #5a6270;
}

.submetric__value--rank {
  color: #4f8cff;
  font-weight: 700;
}

/* Strategy Badges */
.strategy-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
  justify-content: flex-end;
}

.strategy-badge {
  font-size: 0.65rem;
  color: #9fb1c7;
  background: rgba(255, 255, 255, 0.08);
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  white-space: nowrap;
}

/* Footer */
.detail-footer {
  margin-top: auto;
  padding-top: var(--space-4);
}

.detail-footer__divider {
  height: 1px;
  background: #252938;
  margin-bottom: var(--space-3);
}

.detail-footer__formula {
  margin: 0;
  font-size: 0.75rem;
  color: #5a6270;
  text-align: center;
  letter-spacing: 0.02em;
}

/* Scrollbar */
.signal-detail__panel::-webkit-scrollbar {
  width: 4px;
}

.signal-detail__panel::-webkit-scrollbar-track {
  background: transparent;
}

.signal-detail__panel::-webkit-scrollbar-thumb {
  background: #252938;
  border-radius: 2px;
}
</style>

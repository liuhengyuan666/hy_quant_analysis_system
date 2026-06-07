<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatInteger, formatNumber, prettifyToken, signalTone } from '../lib/dashboard-utils.js';

const { t } = useI18n();

const props = defineProps({
  signal: {
    type: Object,
    default: null,
  },
});

const emit = defineEmits(['close']);

const reason = computed(() => props.signal?.reason || {});
const label = computed(() => props.signal?.signal_label || reason.value.label || 'N/A');
const finalScore = computed(() => props.signal?.final_score ?? reason.value.final_score);

const alignedStrategies = computed(() => {
  if (!Array.isArray(reason.value.aligned_strategies) || !reason.value.aligned_strategies.length) {
    return [];
  }
  return reason.value.aligned_strategies;
});

const alignmentCount = computed(() => {
  if (reason.value.alignment === null || reason.value.alignment === undefined) return 'N/A';
  return formatInteger(reason.value.alignment);
});

const rotationRank = computed(() => {
  if (reason.value.rotation?.rank === null || reason.value.rotation?.rank === undefined) return 'N/A';
  return `#${formatInteger(reason.value.rotation.rank)}`;
});
</script>

<template>
  <div class="signal-detail" role="dialog" aria-modal="true" aria-labelledby="signalDetailTitle">
    <button class="signal-detail__backdrop" type="button" :aria-label="t('signalDetail.closeSignalDetail')" @click="emit('close')"></button>
    <article class="signal-detail__panel panel">
      <div class="panel__header signal-detail__header">
        <div>
          <p class="eyebrow">{{ t('signalDetail.eyebrow') }}</p>
          <h2 id="signalDetailTitle">{{ signal.symbol || t('signalDetail.unknownSymbol') }}</h2>
          <p class="panel__lede">{{ reason.summary || t('signalDetail.noSummary') }}</p>
        </div>
        <div class="panel__actions signal-detail__actions">
          <span class="pill" :class="`pill--${signalTone(label)}`">{{ prettifyToken(label) }}</span>
          <span class="pill pill--outline">{{ t('signalDetail.score', { score: formatNumber(finalScore, 2) }) }}</span>
          <button class="signal-detail__close" type="button" :aria-label="t('signalDetail.closeSignalDetail')" @click="emit('close')">×</button>
        </div>
      </div>

      <div class="signal-detail__sections">
        <section class="signal-detail__section">
          <h3>{{ t('signalDetail.strategyWeight') }}</h3>
          <dl>
            <dt>{{ t('signalDetail.bestStrategy') }}</dt>
            <dd>{{ prettifyToken(reason.best_strategy || 'N/A') }}</dd>
            <dt>{{ t('signalDetail.strategyScore') }}</dt>
            <dd>{{ formatNumber(reason.strategy_score, 2) }}</dd>
            <dt>{{ t('signalDetail.contribution') }}</dt>
            <dd>{{ formatNumber(reason.strategy_contribution, 2) }}</dd>
          </dl>
        </section>

        <section class="signal-detail__section">
          <h3>{{ t('signalDetail.alignmentWeight') }}</h3>
          <dl>
            <dt>{{ t('signalDetail.alignmentCount') }}</dt>
            <dd>{{ alignmentCount }}</dd>
            <dt>{{ t('signalDetail.alignedStrategies') }}</dt>
            <dd>
              <div v-if="alignedStrategies.length" class="signal-detail__pill-row">
                <span v-for="strategy in alignedStrategies" :key="strategy" class="pill pill--neutral">
                  {{ prettifyToken(strategy) }}
                </span>
              </div>
              <span v-else class="panel__meta">{{ t('signalDetail.noAlignedStrategies') }}</span>
            </dd>
            <dt>{{ t('signalDetail.contribution') }}</dt>
            <dd>{{ formatNumber(reason.alignment_contribution, 2) }}</dd>
          </dl>
        </section>

        <section class="signal-detail__section">
          <h3>{{ t('signalDetail.regimeWeight') }}</h3>
          <dl>
            <dt>{{ t('signalDetail.trendScore') }}</dt>
            <dd>{{ formatNumber(reason.regime?.trend_score, 2) }}</dd>
            <dt>{{ t('signalDetail.riskScore') }}</dt>
            <dd>{{ formatNumber(reason.regime?.risk_score, 2) }}</dd>
            <dt>{{ t('signalDetail.combinedScore') }}</dt>
            <dd>{{ formatNumber(reason.regime?.combined_score, 2) }}</dd>
            <dt>{{ t('signalDetail.contribution') }}</dt>
            <dd>{{ formatNumber(reason.regime?.contribution, 2) }}</dd>
          </dl>
        </section>

        <section class="signal-detail__section">
          <h3>{{ t('signalDetail.rotationWeight') }}</h3>
          <dl>
            <dt>{{ t('signalDetail.momentumScore') }}</dt>
            <dd>{{ formatNumber(reason.rotation?.momentum_score, 2) }}</dd>
            <dt>{{ t('signalDetail.rank') }}</dt>
            <dd>{{ rotationRank }}</dd>
            <dt>{{ t('signalDetail.combinedScore') }}</dt>
            <dd>{{ formatNumber(reason.rotation?.combined_score, 2) }}</dd>
            <dt>{{ t('signalDetail.contribution') }}</dt>
            <dd>{{ formatNumber(reason.rotation?.contribution, 2) }}</dd>
          </dl>
        </section>
      </div>
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
  width: 400px;
  max-width: 90vw;
  height: 100vh;
  overflow-y: auto;
  background: var(--panel-bg);
  border-left: 1px solid var(--panel-border);
  box-shadow: var(--shadow-strong);
  transform: translateX(0);
  transition: transform 0.3s ease;
}

.signal-detail__header {
  margin-bottom: var(--space-5);
  position: sticky;
  top: 0;
  background: var(--panel-bg);
  z-index: 1;
  padding-bottom: var(--space-4);
  border-bottom: 1px solid var(--panel-border);
}

.signal-detail__actions {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}

.signal-detail__close {
  background: none;
  border: none;
  font-size: 1.5rem;
  color: var(--text-secondary);
  cursor: pointer;
  padding: var(--space-1) var(--space-2);
  line-height: 1;
}

.signal-detail__close:hover {
  color: var(--text-primary);
}

.signal-detail__sections {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.signal-detail__section {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--space-4);
}

.signal-detail__section h3 {
  font-size: var(--font-size-meta);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 var(--space-3);
}

.signal-detail__section dl {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--space-2) var(--space-3);
  margin: 0;
}

.signal-detail__section dt {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

.signal-detail__section dd {
  font-size: var(--font-size-meta);
  color: var(--text-primary);
  margin: 0;
  text-align: right;
}

.signal-detail__pill-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
  justify-content: flex-end;
}

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

.pill--warning {
  background: var(--color-warning-soft);
  color: var(--color-warning);
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
</style>

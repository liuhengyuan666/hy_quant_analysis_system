<script setup>
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatNumber, prettifyToken, signalTone } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const snapshot = computed(() => dashboardStore.snapshot);
const topSignals = computed(() => snapshot.value?.top_signals || []);
const bullishSignals = computed(() => snapshot.value?.bullish_signals || []);
const defensiveSignals = computed(() => snapshot.value?.defensive_signals || []);
const symbolNames = computed(() => snapshot.value?.symbol_names || {});

const mergedSignals = computed(() => {
  const map = new Map();
  [...topSignals.value, ...bullishSignals.value].forEach(item => {
    const existing = map.get(item.symbol);
    if (!existing || (item.final_score || 0) > (existing.final_score || 0)) {
      map.set(item.symbol, item);
    }
  });
  return Array.from(map.values());
});

const hasSignals = computed(() => mergedSignals.value.length > 0 || defensiveSignals.value.length > 0);

const signalDistribution = computed(() => {
  const counts = { StrongBuy: 0, Buy: 0, Other: 0 };
  mergedSignals.value.forEach(item => {
    const label = item.signal_label || '';
    if (label === 'StrongBuy') counts.StrongBuy++;
    else if (label === 'Buy') counts.Buy++;
    else counts.Other++;
  });
  return counts;
});

const signalBasis = computed(() => {
  if (!snapshot.value || mergedSignals.value.length === 0) return null;
  const signal = mergedSignals.value[0];
  const analysisScope = String(signal.analysis_scope || 'GLOBAL').toUpperCase();
  const regimeBasisScope = String(signal.regime_basis_scope || 'GLOBAL').toUpperCase();
  const snapshotScope = String(snapshot.value.scope || 'GLOBAL').toUpperCase();
  return { analysisScope, regimeBasisScope, snapshotScope, mismatched: regimeBasisScope !== snapshotScope };
});

const hoveredSignal = ref(null);
function setHovered(item) { hoveredSignal.value = item; }
function clearHovered() { hoveredSignal.value = null; }

function getHoveredSegment(item) {
  if (!hoveredSignal.value || hoveredSignal.value.symbol !== item.symbol) return null;
  return hoveredSignal.value.segment ?? null;
}

function handleBarHover(item, idx) { hoveredSignal.value = { ...item, segment: idx }; }
function handleBarLeave(item) { if (hoveredSignal.value?.symbol === item.symbol) hoveredSignal.value = null; }

function getStackedBar(item) {
  const r = item.reason || {};
  const sc = r.strategy_contribution ?? 0;
  const ac = r.alignment_contribution ?? 0;
  const rc = r.regime?.contribution ?? 0;
  const rot = r.rotation?.contribution ?? 0;
  const total = sc + ac + rc + rot || 1;
  return [
    { label: 'Strategy', value: sc, pct: (sc / total) * 100, color: '#4f8cff' },
    { label: 'Alignment', value: ac, pct: (ac / total) * 100, color: '#16c784' },
    { label: 'Regime', value: rc, pct: (rc / total) * 100, color: '#f5b041' },
    { label: 'Rotation', value: rot, pct: (rot / total) * 100, color: '#5ce1e6' },
  ].filter(s => s.value > 0.01);
}

function getStrategy(item) {
  const r = item.reason || {};
  return { rawScore: r.strategy_score ?? 0, contribution: r.strategy_contribution ?? 0, bestStrategy: r.best_strategy ?? 'N/A' };
}

function getAlignment(item) {
  const r = item.reason || {};
  const aligned = r.aligned_strategies || [];
  return { rawScore: (r.alignment ?? 0) * 20, contribution: r.alignment_contribution ?? 0, count: r.alignment ?? 0, aligned };
}

function getRegime(item) {
  const r = item.reason || {};
  return { trendScore: r.regime?.trend_score ?? 0, riskScore: r.regime?.risk_score ?? 0, combinedScore: r.regime?.combined_score ?? 0, contribution: r.regime?.contribution ?? 0 };
}

function getRotation(item) {
  const r = item.reason || {};
  return { momentumScore: r.rotation?.momentum_score ?? 0, rank: r.rotation?.rank ? `#${r.rotation.rank}` : 'N/A', combinedScore: r.rotation?.combined_score ?? 0, contribution: r.rotation?.contribution ?? 0 };
}
</script>

<template>
  <article class="panel">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('signals.eyebrow') }}</p>
        <h2>{{ t('signals.title') }}</h2>
        <p v-if="hasSignals" class="panel__lede">{{ t('signals.lede') }}</p>
      </div>
      <div v-if="hasSignals" class="panel__actions">
        <span class="panel__meta">{{ t('signals.groupedView', { date: snapshot?.report_date }) }}</span>
      </div>
    </div>

    <div v-if="signalBasis" class="panel__meta-row">
      <span class="panel__meta">{{ t('signals.dashboardScope', { scope: signalBasis.snapshotScope }) }}</span>
      <span class="panel__meta">{{ t('signals.analysisScope', { scope: signalBasis.analysisScope }) }}</span>
      <span class="panel__meta">{{ t('signals.regimeBasis', { scope: signalBasis.regimeBasisScope }) }}</span>
    </div>

    <section v-if="signalBasis?.mismatched" class="staleness-banner staleness-banner--warning" aria-label="Signal provenance notice">
      <strong>{{ t('signals.basisDiffers') }}</strong>
      <p>{{ t('signals.showingSignals', { scope: signalBasis.snapshotScope, analysis: signalBasis.analysisScope, regime: signalBasis.regimeBasisScope }) }}</p>
    </section>

    <div v-if="!hasSignals" class="empty-state">
      <p>{{ t('signals.noSignals') }}</p>
    </div>

    <template v-else>
      <div class="signal-distribution">
        <div class="signal-distribution__item">
          <span class="signal-distribution__count signal-distribution__count--strong">{{ signalDistribution.StrongBuy }}</span>
          <span class="signal-distribution__label">StrongBuy</span>
        </div>
        <div class="signal-distribution__item">
          <span class="signal-distribution__count signal-distribution__count--buy">{{ signalDistribution.Buy }}</span>
          <span class="signal-distribution__label">Buy</span>
        </div>
        <div v-if="signalDistribution.Other > 0" class="signal-distribution__item">
          <span class="signal-distribution__count signal-distribution__count--watch">{{ signalDistribution.Other }}</span>
          <span class="signal-distribution__label">Other</span>
        </div>
      </div>

      <div class="signal-groups-grid">
        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">{{ t('signals.bullishOpportunities') }}</p>
            <span class="panel__meta">{{ t('signals.strongBuyBuy') }}</span>
          </div>
          <div v-if="mergedSignals.length" class="signal-list">
            <div
              v-for="(item) in mergedSignals"
              :key="item.symbol"
              class="signal-card"
              :class="`signal-card--${signalTone(item.signal_label)}`"
              @mouseenter="setHovered(item)"
              @mouseleave="clearHovered"
            >
              <div class="signal-card__header">
                <div>
                  <strong class="signal-card__symbol">{{ item.symbol }}</strong>
                  <span v-if="symbolNames[item.symbol]" class="signal-card__name">{{ symbolNames[item.symbol] }}</span>
                  <p class="signal-card__score">{{ t('signals.score', { score: formatNumber(item.final_score, 2) }) }}</p>
                </div>
                <span class="pill" :class="`pill--${signalTone(item.signal_label)}`">{{ prettifyToken(item.signal_label) }}</span>
              </div>
              <p v-if="item.reason" class="signal-card__reason">{{ item.reason?.summary || '' }}</p>

              <!-- Hover Detail Card -->
              <div class="signal-detail-card">
                <div class="detail-header">
                  <div class="detail-header__top">
                    <div class="detail-header__symbol">
                      <span class="detail-header__code">{{ item.symbol }}</span>
                      <span class="detail-header__name">{{ symbolNames[item.symbol] || '' }}</span>
                    </div>
                    <span class="detail-header__badge" :class="`badge--${signalTone(item.signal_label)}`">{{ prettifyToken(item.signal_label) }}</span>
                  </div>
                  <div class="detail-header__score">
                    <span class="detail-header__score-label">最终得分</span>
                    <span class="detail-header__score-value">{{ formatNumber(item.final_score, 2) }}</span>
                  </div>
                  <div class="stacked-bar">
                    <div class="stacked-bar__track">
                      <div
                        v-for="(seg, idx) in getStackedBar(item)"
                        :key="seg.label"
                        class="stacked-bar__segment"
                        :style="{ width: seg.pct + '%', background: seg.color }"
                        @mouseenter="handleBarHover(item, idx)"
                        @mouseleave="handleBarLeave(item)"
                      ></div>
                    </div>
                    <div class="stacked-bar__legend">
                      <span v-for="(seg, idx) in getStackedBar(item)" :key="seg.label" class="stacked-bar__legend-item" :class="{ 'stacked-bar__legend-item--active': getHoveredSegment(item) === idx }">
                        <span class="stacked-bar__dot" :style="{ background: seg.color }"></span>
                        {{ seg.label }} {{ formatNumber(seg.value, 2) }}
                      </span>
                    </div>
                  </div>
                </div>

                <div class="detail-cards">
                  <section class="dim-card" :class="{ 'dim-card--active': getHoveredSegment(item) === 0 }">
                    <div class="dim-card__header">
                      <div class="dim-card__title">
                        <span class="dim-card__name">策略层</span>
                        <span class="dim-card__weight">45% 权重</span>
                      </div>
                      <span class="dim-card__contribution" style="color: #4f8cff;">+{{ formatNumber(getStrategy(item).contribution, 2) }}</span>
                    </div>
                    <div class="dim-card__gauge">
                      <div class="gauge-track"><div class="gauge-fill" :style="{ width: getStrategy(item).rawScore + '%' }"></div></div>
                      <span class="gauge-value">{{ formatNumber(getStrategy(item).rawScore, 2) }}</span>
                    </div>
                    <div class="dim-card__subgrid">
                      <div class="submetric"><span class="submetric__label">最佳策略</span><span class="submetric__value">{{ prettifyToken(getStrategy(item).bestStrategy) }}</span></div>
                      <div class="submetric"><span class="submetric__label">策略得分</span><span class="submetric__value">{{ formatNumber(getStrategy(item).rawScore, 2) }}</span></div>
                    </div>
                  </section>

                  <section class="dim-card" :class="{ 'dim-card--active': getHoveredSegment(item) === 1 }">
                    <div class="dim-card__header">
                      <div class="dim-card__title">
                        <span class="dim-card__name">模式层</span>
                        <span class="dim-card__weight">15% 权重</span>
                      </div>
                      <span class="dim-card__contribution" style="color: #16c784;">+{{ formatNumber(getAlignment(item).contribution, 2) }}</span>
                    </div>
                    <div class="dim-card__gauge">
                      <div class="gauge-track"><div class="gauge-fill" :style="{ width: getAlignment(item).rawScore + '%', background: '#16c784' }"></div></div>
                      <span class="gauge-value">{{ formatNumber(getAlignment(item).rawScore, 2) }}</span>
                    </div>
                    <div class="dim-card__subgrid">
                      <div class="submetric"><span class="submetric__label">对齐数量</span><span class="submetric__value">{{ getAlignment(item).count }}</span></div>
                      <div class="submetric submetric--full">
                        <span class="submetric__label">对齐策略</span>
                        <div class="strategy-badges">
                          <span v-for="s in getAlignment(item).aligned" :key="s" class="strategy-badge">{{ prettifyToken(s) }}</span>
                          <span v-if="!getAlignment(item).aligned.length" class="submetric__value--muted">无</span>
                        </div>
                      </div>
                    </div>
                  </section>

                  <section class="dim-card" :class="{ 'dim-card--active': getHoveredSegment(item) === 2 }">
                    <div class="dim-card__header">
                      <div class="dim-card__title">
                        <span class="dim-card__name">体制层</span>
                        <span class="dim-card__weight">20% 权重</span>
                      </div>
                      <span class="dim-card__contribution" style="color: #f5b041;">+{{ formatNumber(getRegime(item).contribution, 2) }}</span>
                    </div>
                    <div class="dim-card__gauge">
                      <div class="gauge-track"><div class="gauge-fill" :style="{ width: getRegime(item).combinedScore + '%', background: '#f5b041' }"></div></div>
                      <span class="gauge-value">{{ formatNumber(getRegime(item).combinedScore, 2) }}</span>
                    </div>
                    <div class="dim-card__subgrid">
                      <div class="submetric"><span class="submetric__label">趋势分数</span><span class="submetric__value">{{ formatNumber(getRegime(item).trendScore, 2) }}</span></div>
                      <div class="submetric"><span class="submetric__label">风险分数</span><span class="submetric__value">{{ formatNumber(getRegime(item).riskScore, 2) }}</span></div>
                    </div>
                  </section>

                  <section class="dim-card" :class="{ 'dim-card--active': getHoveredSegment(item) === 3 }">
                    <div class="dim-card__header">
                      <div class="dim-card__title">
                        <span class="dim-card__name">轮动层</span>
                        <span class="dim-card__weight">20% 权重</span>
                      </div>
                      <span class="dim-card__contribution" style="color: #5ce1e6;">+{{ formatNumber(getRotation(item).contribution, 2) }}</span>
                    </div>
                    <div class="dim-card__gauge">
                      <div class="gauge-track"><div class="gauge-fill" :style="{ width: getRotation(item).combinedScore + '%', background: '#5ce1e6' }"></div></div>
                      <span class="gauge-value">{{ formatNumber(getRotation(item).combinedScore, 2) }}</span>
                    </div>
                    <div class="dim-card__subgrid">
                      <div class="submetric"><span class="submetric__label">动量分数</span><span class="submetric__value">{{ formatNumber(getRotation(item).momentumScore, 2) }}</span></div>
                      <div class="submetric"><span class="submetric__label">排名</span><span class="submetric__value submetric__value--rank">{{ getRotation(item).rank }}</span></div>
                    </div>
                  </section>
                </div>

                <div class="detail-footer">
                  <div class="detail-footer__divider"></div>
                  <p class="detail-footer__formula">
                    {{ formatNumber(getStrategy(item).contribution, 2) }} (策略) + {{ formatNumber(getAlignment(item).contribution, 2) }} (模式) + {{ formatNumber(getRegime(item).contribution, 2) }} (体制) + {{ formatNumber(getRotation(item).contribution, 2) }} (轮动) = {{ formatNumber(item.final_score, 2) }} (最终分)
                  </p>
                </div>
              </div>
            </div>
          </div>
          <div v-else class="empty-state empty-state--compact"><p>{{ t('signals.noBullish') }}</p></div>
        </section>

        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">{{ t('signals.defensiveSell') }}</p>
            <span class="panel__meta">{{ t('signals.watchHoldReduceSell') }}</span>
          </div>
          <div v-if="defensiveSignals.length" class="signal-list">
            <div
              v-for="(item) in defensiveSignals"
              :key="item.symbol"
              class="signal-card signal-card--defensive"
              @mouseenter="setHovered(item)"
              @mouseleave="clearHovered"
            >
              <div class="signal-card__header">
                <div>
                  <strong class="signal-card__symbol">{{ item.symbol }}</strong>
                  <span v-if="symbolNames[item.symbol]" class="signal-card__name">{{ symbolNames[item.symbol] }}</span>
                  <p class="signal-card__score">{{ t('signals.score', { score: formatNumber(item.final_score, 2) }) }}</p>
                </div>
                <span class="pill" :class="`pill--${signalTone(item.signal_label)}`">{{ prettifyToken(item.signal_label) }}</span>
              </div>
              <p v-if="item.reason" class="signal-card__reason">{{ item.reason?.summary || '' }}</p>

              <div class="signal-detail-card">
                <div class="detail-header">
                  <div class="detail-header__top">
                    <div class="detail-header__symbol">
                      <span class="detail-header__code">{{ item.symbol }}</span>
                      <span class="detail-header__name">{{ symbolNames[item.symbol] || '' }}</span>
                    </div>
                    <span class="detail-header__badge" :class="`badge--${signalTone(item.signal_label)}`">{{ prettifyToken(item.signal_label) }}</span>
                  </div>
                  <div class="detail-header__score">
                    <span class="detail-header__score-label">最终得分</span>
                    <span class="detail-header__score-value">{{ formatNumber(item.final_score, 2) }}</span>
                  </div>
                  <div class="stacked-bar">
                    <div class="stacked-bar__track">
                      <div v-for="(seg, idx) in getStackedBar(item)" :key="seg.label" class="stacked-bar__segment" :style="{ width: seg.pct + '%', background: seg.color }" @mouseenter="handleBarHover(item, idx)" @mouseleave="handleBarLeave(item)"></div>
                    </div>
                    <div class="stacked-bar__legend">
                      <span v-for="(seg, idx) in getStackedBar(item)" :key="seg.label" class="stacked-bar__legend-item" :class="{ 'stacked-bar__legend-item--active': getHoveredSegment(item) === idx }">
                        <span class="stacked-bar__dot" :style="{ background: seg.color }"></span>
                        {{ seg.label }} {{ formatNumber(seg.value, 2) }}
                      </span>
                    </div>
                  </div>
                </div>
                <div class="detail-cards">
                  <section class="dim-card" :class="{ 'dim-card--active': getHoveredSegment(item) === 0 }">
                    <div class="dim-card__header">
                      <div class="dim-card__title"><span class="dim-card__name">策略层</span><span class="dim-card__weight">45% 权重</span></div>
                      <span class="dim-card__contribution" style="color: #4f8cff;">+{{ formatNumber(getStrategy(item).contribution, 2) }}</span>
                    </div>
                    <div class="dim-card__gauge">
                      <div class="gauge-track"><div class="gauge-fill" :style="{ width: getStrategy(item).rawScore + '%' }"></div></div>
                      <span class="gauge-value">{{ formatNumber(getStrategy(item).rawScore, 2) }}</span>
                    </div>
                    <div class="dim-card__subgrid">
                      <div class="submetric"><span class="submetric__label">最佳策略</span><span class="submetric__value">{{ prettifyToken(getStrategy(item).bestStrategy) }}</span></div>
                      <div class="submetric"><span class="submetric__label">策略得分</span><span class="submetric__value">{{ formatNumber(getStrategy(item).rawScore, 2) }}</span></div>
                    </div>
                  </section>
                  <section class="dim-card" :class="{ 'dim-card--active': getHoveredSegment(item) === 1 }">
                    <div class="dim-card__header">
                      <div class="dim-card__title"><span class="dim-card__name">模式层</span><span class="dim-card__weight">15% 权重</span></div>
                      <span class="dim-card__contribution" style="color: #16c784;">+{{ formatNumber(getAlignment(item).contribution, 2) }}</span>
                    </div>
                    <div class="dim-card__gauge">
                      <div class="gauge-track"><div class="gauge-fill" :style="{ width: getAlignment(item).rawScore + '%', background: '#16c784' }"></div></div>
                      <span class="gauge-value">{{ formatNumber(getAlignment(item).rawScore, 2) }}</span>
                    </div>
                    <div class="dim-card__subgrid">
                      <div class="submetric"><span class="submetric__label">对齐数量</span><span class="submetric__value">{{ getAlignment(item).count }}</span></div>
                      <div class="submetric submetric--full">
                        <span class="submetric__label">对齐策略</span>
                        <div class="strategy-badges">
                          <span v-for="s in getAlignment(item).aligned" :key="s" class="strategy-badge">{{ prettifyToken(s) }}</span>
                          <span v-if="!getAlignment(item).aligned.length" class="submetric__value--muted">无</span>
                        </div>
                      </div>
                    </div>
                  </section>
                  <section class="dim-card" :class="{ 'dim-card--active': getHoveredSegment(item) === 2 }">
                    <div class="dim-card__header">
                      <div class="dim-card__title"><span class="dim-card__name">体制层</span><span class="dim-card__weight">20% 权重</span></div>
                      <span class="dim-card__contribution" style="color: #f5b041;">+{{ formatNumber(getRegime(item).contribution, 2) }}</span>
                    </div>
                    <div class="dim-card__gauge">
                      <div class="gauge-track"><div class="gauge-fill" :style="{ width: getRegime(item).combinedScore + '%', background: '#f5b041' }"></div></div>
                      <span class="gauge-value">{{ formatNumber(getRegime(item).combinedScore, 2) }}</span>
                    </div>
                    <div class="dim-card__subgrid">
                      <div class="submetric"><span class="submetric__label">趋势分数</span><span class="submetric__value">{{ formatNumber(getRegime(item).trendScore, 2) }}</span></div>
                      <div class="submetric"><span class="submetric__label">风险分数</span><span class="submetric__value">{{ formatNumber(getRegime(item).riskScore, 2) }}</span></div>
                    </div>
                  </section>
                  <section class="dim-card" :class="{ 'dim-card--active': getHoveredSegment(item) === 3 }">
                    <div class="dim-card__header">
                      <div class="dim-card__title"><span class="dim-card__name">轮动层</span><span class="dim-card__weight">20% 权重</span></div>
                      <span class="dim-card__contribution" style="color: #5ce1e6;">+{{ formatNumber(getRotation(item).contribution, 2) }}</span>
                    </div>
                    <div class="dim-card__gauge">
                      <div class="gauge-track"><div class="gauge-fill" :style="{ width: getRotation(item).combinedScore + '%', background: '#5ce1e6' }"></div></div>
                      <span class="gauge-value">{{ formatNumber(getRotation(item).combinedScore, 2) }}</span>
                    </div>
                    <div class="dim-card__subgrid">
                      <div class="submetric"><span class="submetric__label">动量分数</span><span class="submetric__value">{{ formatNumber(getRotation(item).momentumScore, 2) }}</span></div>
                      <div class="submetric"><span class="submetric__label">排名</span><span class="submetric__value submetric__value--rank">{{ getRotation(item).rank }}</span></div>
                    </div>
                  </section>
                </div>
                <div class="detail-footer">
                  <div class="detail-footer__divider"></div>
                  <p class="detail-footer__formula">
                    {{ formatNumber(getStrategy(item).contribution, 2) }} (策略) + {{ formatNumber(getAlignment(item).contribution, 2) }} (模式) + {{ formatNumber(getRegime(item).contribution, 2) }} (体制) + {{ formatNumber(getRotation(item).contribution, 2) }} (轮动) = {{ formatNumber(item.final_score, 2) }} (最终分)
                  </p>
                </div>
              </div>
            </div>
          </div>
          <div v-else class="empty-state empty-state--compact"><p>{{ t('signals.noDefensive') }}</p></div>
        </section>
      </div>
    </template>
  </article>
</template>

<style scoped>
.panel { background: var(--panel-bg); border: 1px solid var(--panel-border); border-radius: var(--panel-radius); padding: var(--panel-padding); }
.panel__header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: var(--space-4); }
.panel__actions { display: flex; gap: var(--space-2); align-items: center; }
.panel__lede { color: var(--text-secondary); font-size: var(--font-size-meta); margin-top: var(--space-1); }
.panel__meta { font-size: var(--font-size-label); color: var(--text-secondary); }
.panel__meta-row { display: flex; gap: var(--space-4); flex-wrap: wrap; margin-bottom: var(--space-4); }
.panel__subheader { display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-3); }
.panel__section-title { font-weight: 600; color: var(--text-primary); }

.signal-distribution { display: flex; gap: var(--space-4); margin-bottom: var(--space-4); padding: var(--space-3); background: var(--panel-bg-secondary); border: 1px solid var(--panel-border); border-radius: var(--panel-radius); }
.signal-distribution__item { display: flex; flex-direction: column; align-items: center; gap: var(--space-1); min-width: 4rem; }
.signal-distribution__count { font-family: var(--font-mono); font-size: 1.5rem; font-weight: 700; line-height: 1; }
.signal-distribution__count--strong { color: var(--tone-positive); }
.signal-distribution__count--buy { color: var(--color-accent); }
.signal-distribution__count--watch { color: var(--color-warning); }
.signal-distribution__label { font-size: var(--font-size-label); color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.05em; }

.signal-groups-grid { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-5); }
@media (max-width: 720px) { .signal-groups-grid { grid-template-columns: 1fr; } }

.signal-list { display: flex; flex-direction: column; gap: var(--space-2); }

.signal-card { position: relative; background: var(--panel-bg-secondary); border: 1px solid var(--panel-border); border-radius: var(--panel-radius); padding: var(--space-3); text-align: left; cursor: default; transition: border-color 0.2s ease; }
.signal-card:hover { border-color: var(--color-accent-border); }
.signal-card__header { display: flex; justify-content: space-between; align-items: flex-start; }
.signal-card__symbol { display: inline; font-size: 1rem; font-weight: 600; color: var(--text-primary); }
.signal-card__name { display: inline; margin-left: var(--space-2); font-size: var(--font-size-label); color: var(--text-secondary); }
.signal-card__score { font-size: var(--font-size-label); color: var(--text-secondary); margin: var(--space-1) 0 0; }
.signal-card__reason { font-size: var(--font-size-label); color: var(--text-secondary); margin: var(--space-2) 0 0; }

.pill { display: inline-flex; align-items: center; padding: var(--space-1) var(--space-3); border-radius: var(--space-3); font-size: var(--font-size-label); font-weight: 500; }
.pill--positive { background: var(--tone-positive-bg); color: var(--tone-positive); }
.pill--negative { background: var(--tone-negative-bg); color: var(--tone-negative); }
.pill--neutral { background: var(--tone-neutral-bg); color: var(--tone-neutral); }
.pill--warning { background: var(--color-warning-soft); color: var(--color-warning); }

.staleness-banner { padding: var(--space-3); border-radius: var(--panel-radius); margin-bottom: var(--space-4); }
.staleness-banner--warning { background: var(--color-warning-soft); border: 1px solid var(--color-warning); }
.staleness-banner strong { display: block; margin-bottom: var(--space-1); color: var(--color-warning); }
.staleness-banner p { margin: 0; font-size: var(--font-size-meta); color: var(--text-secondary); }

.signal-card--positive { border-color: rgba(118, 212, 159, 0.18); }
.signal-card--negative { border-color: rgba(240, 141, 126, 0.18); }
.signal-card--defensive { background: linear-gradient(180deg, rgba(245, 176, 65, 0.06), rgba(245, 176, 65, 0.02)); }

.eyebrow { font-size: var(--font-size-label); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--text-secondary); }
h2 { margin: var(--space-1) 0 0; font-size: 1.25rem; font-weight: 700; color: var(--text-primary); }
.empty-state { padding: var(--space-5); text-align: center; color: var(--text-secondary); }
.empty-state--compact { padding: var(--space-3); }

/* Hover Detail Card */
.signal-detail-card { visibility: hidden; opacity: 0; position: absolute; z-index: 100; bottom: calc(100% + 0.6rem); left: 50%; transform: translateX(-50%); transition: opacity 0.15s ease-in-out, visibility 0.15s ease-in-out; background: #0f1117; border: 1px solid #252938; border-radius: 8px; padding: 1.5rem; width: 34rem; max-width: 92vw; box-shadow: 0 8px 40px rgba(0, 0, 0, 0.6); pointer-events: none; font-family: var(--font-mono); }
.signal-card:hover .signal-detail-card { visibility: visible; opacity: 1; pointer-events: auto; }

.detail-header { margin-bottom: 1.25rem; padding-bottom: 1rem; border-bottom: 1px solid #252938; }
.detail-header__top { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1rem; }
.detail-header__symbol { display: flex; flex-direction: column; gap: 0.25rem; }
.detail-header__code { font-size: 1.4rem; font-weight: 700; color: #d7dae0; letter-spacing: -0.02em; }
.detail-header__name { font-size: 1rem; color: #8a909a; }
.detail-header__badge { padding: 0.35rem 0.9rem; border-radius: 999px; font-size: 0.9rem; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; }
.badge--positive { background: rgba(22, 199, 132, 0.15); color: #16c784; border: 1px solid rgba(22, 199, 132, 0.25); }
.badge--negative { background: rgba(255, 92, 92, 0.15); color: #ff5c5c; border: 1px solid rgba(255, 92, 92, 0.25); }
.badge--neutral { background: rgba(245, 176, 65, 0.15); color: #f5b041; border: 1px solid rgba(245, 176, 65, 0.25); }
.badge--warning { background: rgba(245, 176, 65, 0.15); color: #f5b041; border: 1px solid rgba(245, 176, 65, 0.25); }

.detail-header__score { display: flex; align-items: baseline; gap: 0.6rem; margin-bottom: 1rem; }
.detail-header__score-label { font-size: 0.95rem; color: #8a909a; text-transform: uppercase; letter-spacing: 0.08em; }
.detail-header__score-value { font-size: 2.2rem; font-weight: 700; color: #d7dae0; line-height: 1; }

.stacked-bar { margin-bottom: 0.35rem; }
.stacked-bar__track { display: flex; height: 7px; border-radius: 3.5px; overflow: hidden; background: rgba(255, 255, 255, 0.06); margin-bottom: 0.6rem; }
.stacked-bar__segment { height: 100%; transition: opacity 0.2s ease; cursor: pointer; }
.stacked-bar__segment:hover { opacity: 0.85; }
.stacked-bar__legend { display: flex; gap: 0.9rem; flex-wrap: wrap; }
.stacked-bar__legend-item { display: flex; align-items: center; gap: 0.35rem; font-size: 0.85rem; color: #8a909a; transition: color 0.2s ease; }
.stacked-bar__legend-item--active { color: #d7dae0; }
.stacked-bar__dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }

.detail-cards { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
.dim-card { background: #171923; border: 1px solid #252938; border-radius: 6px; padding: 1rem; transition: border-color 0.2s ease, box-shadow 0.2s ease; }
.dim-card--active { border-color: #4f8cff; box-shadow: 0 0 0 1px rgba(79, 140, 255, 0.15); }
.dim-card__header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.6rem; }
.dim-card__title { display: flex; align-items: center; gap: 0.4rem; }
.dim-card__name { font-size: 1rem; font-weight: 600; color: #d7dae0; }
.dim-card__weight { font-size: 0.8rem; color: #5a6270; background: rgba(255, 255, 255, 0.06); padding: 0.15rem 0.4rem; border-radius: 4px; }
.dim-card__contribution { font-size: 1.15rem; font-weight: 700; }

.dim-card__gauge { display: flex; align-items: center; gap: 0.6rem; margin-bottom: 0.6rem; }
.gauge-track { flex: 1; height: 5px; border-radius: 2.5px; background: rgba(255, 255, 255, 0.06); overflow: hidden; }
.gauge-fill { height: 100%; border-radius: 2.5px; background: #4f8cff; transition: width 0.5s ease; }
.gauge-value { font-size: 0.95rem; color: #d7dae0; font-weight: 600; min-width: 3.5ch; text-align: right; }

.dim-card__subgrid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.5rem 1rem; }
.submetric { display: flex; justify-content: space-between; align-items: center; gap: 0.4rem; }
.submetric--full { grid-column: 1 / -1; }
.submetric__label { font-size: 0.9rem; color: #5a6270; }
.submetric__value { font-size: 0.95rem; color: #d7dae0; font-weight: 500; }
.submetric__value--muted { font-size: 0.95rem; color: #5a6270; }
.submetric__value--rank { color: #4f8cff; font-weight: 700; }

.strategy-badges { display: flex; flex-wrap: wrap; gap: 0.3rem; justify-content: flex-end; }
.strategy-badge { font-size: 0.8rem; color: #9fb1c7; background: rgba(255, 255, 255, 0.08); padding: 0.2rem 0.5rem; border-radius: 4px; white-space: nowrap; }

.detail-footer { margin-top: 1rem; padding-top: 1rem; }
.detail-footer__divider { height: 1px; background: #252938; margin-bottom: 0.6rem; }
.detail-footer__formula { margin: 0; font-size: 0.9rem; color: #5a6270; text-align: center; letter-spacing: 0.02em; }
</style>

<script setup>
import { computed } from 'vue';
import { formatNumber, prettifyToken, signalTone } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const snapshot = computed(() => dashboardStore.snapshot);
const topSignals = computed(() => snapshot.value?.top_signals || []);
const bullishSignals = computed(() => snapshot.value?.bullish_signals || []);
const defensiveSignals = computed(() => snapshot.value?.defensive_signals || []);
const hasSignals = computed(() => topSignals.value.length > 0 || bullishSignals.value.length > 0 || defensiveSignals.value.length > 0);

const signalBasis = computed(() => {
  if (!snapshot.value) return null;
  const signal = topSignals.value[0] || bullishSignals.value[0] || defensiveSignals.value[0];
  if (!signal) return null;

  const analysisScope = String(signal.analysis_scope || 'GLOBAL').toUpperCase();
  const regimeBasisScope = String(signal.regime_basis_scope || 'GLOBAL').toUpperCase();
  const snapshotScope = String(snapshot.value.scope || 'GLOBAL').toUpperCase();

  return {
    analysisScope,
    regimeBasisScope,
    snapshotScope,
    mismatched: regimeBasisScope !== snapshotScope,
  };
});

const emit = defineEmits(['select-signal']);

function handleSignalClick(group, index) {
  const signals = group === 'top' ? topSignals.value
    : group === 'bullish' ? bullishSignals.value
    : defensiveSignals.value;
  emit('select-signal', signals[index]);
}
</script>

<template>
  <article class="panel">
    <div class="panel__header">
      <div>
        <p class="eyebrow">Signal stack</p>
        <h2>Buy & defensive groups</h2>
        <p v-if="hasSignals" class="panel__lede">
          Bullish opportunities separated from defensive or sell-side signals for the selected report date.
        </p>
      </div>
      <div v-if="hasSignals" class="panel__actions">
        <span class="panel__meta">Grouped signal view for {{ snapshot?.report_date }}</span>
      </div>
    </div>

    <div v-if="signalBasis" class="panel__meta-row">
      <span class="panel__meta">Dashboard scope · {{ signalBasis.snapshotScope }}</span>
      <span class="panel__meta">Signal analysis scope · {{ signalBasis.analysisScope }}</span>
      <span class="panel__meta">Signal regime basis · {{ signalBasis.regimeBasisScope }}</span>
    </div>

    <section v-if="signalBasis?.mismatched" class="staleness-banner staleness-banner--warning" aria-label="Signal provenance notice">
      <strong>Signal scoring basis differs from the selected scope</strong>
      <p>This {{ signalBasis.snapshotScope }} view is currently showing signals with analysis scope {{ signalBasis.analysisScope }} and regime basis {{ signalBasis.regimeBasisScope }}.</p>
    </section>

    <div v-if="!hasSignals" class="empty-state">
      <p>No signal candidates are available for the latest report date.</p>
    </div>

    <template v-else>
      <section v-if="topSignals.length" class="signal-focus-section">
        <div class="panel__subheader">
          <p class="panel__section-title">Top signals</p>
          <span class="panel__meta">Highest conviction across labels</span>
        </div>
        <div class="signal-list">
          <button
            v-for="(item, index) in topSignals"
            :key="`top-${index}`"
            class="signal-card signal-card--top signal-card--interactive"
            type="button"
            @click="handleSignalClick('top', index)"
          >
            <div class="signal-card__header">
              <div>
                <strong class="signal-card__symbol">{{ item.symbol }}</strong>
                <p class="signal-card__score">Score {{ formatNumber(item.final_score, 2) }}</p>
              </div>
              <span class="pill" :class="`pill--${signalTone(item.signal_label)}`">
                {{ prettifyToken(item.signal_label) }}
              </span>
            </div>
            <p v-if="item.reason" class="signal-card__reason">{{ item.reason?.summary || '' }}</p>
          </button>
        </div>
      </section>

      <div class="signal-groups-grid">
        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">Bullish opportunities</p>
            <span class="panel__meta">StrongBuy / Buy</span>
          </div>
          <div v-if="bullishSignals.length" class="signal-list">
            <button
              v-for="(item, index) in bullishSignals"
              :key="`bullish-${index}`"
              class="signal-card signal-card--bullish signal-card--interactive"
              type="button"
              @click="handleSignalClick('bullish', index)"
            >
              <div class="signal-card__header">
                <div>
                  <strong class="signal-card__symbol">{{ item.symbol }}</strong>
                  <p class="signal-card__score">Score {{ formatNumber(item.final_score, 2) }}</p>
                </div>
                <span class="pill" :class="`pill--${signalTone(item.signal_label)}`">
                  {{ prettifyToken(item.signal_label) }}
                </span>
              </div>
              <p v-if="item.reason" class="signal-card__reason">{{ item.reason?.summary || '' }}</p>
            </button>
          </div>
          <div v-else class="empty-state empty-state--compact">
            <p>No bullish signals for this date.</p>
          </div>
        </section>

        <section>
          <div class="panel__subheader">
            <p class="panel__section-title">Defensive / sell watch</p>
            <span class="panel__meta">Watch / Hold / Reduce / Sell</span>
          </div>
          <div v-if="defensiveSignals.length" class="signal-list">
            <button
              v-for="(item, index) in defensiveSignals"
              :key="`defensive-${index}`"
              class="signal-card signal-card--defensive signal-card--interactive"
              type="button"
              @click="handleSignalClick('defensive', index)"
            >
              <div class="signal-card__header">
                <div>
                  <strong class="signal-card__symbol">{{ item.symbol }}</strong>
                  <p class="signal-card__score">Score {{ formatNumber(item.final_score, 2) }}</p>
                </div>
                <span class="pill" :class="`pill--${signalTone(item.signal_label)}`">
                  {{ prettifyToken(item.signal_label) }}
                </span>
              </div>
              <p v-if="item.reason" class="signal-card__reason">{{ item.reason?.summary || '' }}</p>
            </button>
          </div>
          <div v-else class="empty-state empty-state--compact">
            <p>No defensive or sell-side signals for this date.</p>
          </div>
        </section>
      </div>
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

.panel__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--space-4);
}

.panel__actions {
  display: flex;
  gap: var(--space-2);
  align-items: center;
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

.panel__meta-row {
  display: flex;
  gap: var(--space-4);
  flex-wrap: wrap;
  margin-bottom: var(--space-4);
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

.signal-focus-section {
  margin-bottom: var(--space-5);
}

.signal-groups-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-5);
}

@media (max-width: 720px) {
  .signal-groups-grid {
    grid-template-columns: 1fr;
  }
}

.signal-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.signal-card {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--space-3);
  text-align: left;
  cursor: pointer;
  transition: border-color 0.2s ease;
}

.signal-card:hover {
  border-color: var(--color-accent-border);
}

.signal-card__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.signal-card__symbol {
  display: block;
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
}

.signal-card__score {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  margin: var(--space-1) 0 0;
}

.signal-card__reason {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  margin: var(--space-2) 0 0;
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: var(--space-1) var(--space-3);
  border-radius: var(--space-3);
  font-size: var(--font-size-label);
  font-weight: 500;
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

.staleness-banner {
  padding: var(--space-3);
  border-radius: var(--panel-radius);
  margin-bottom: var(--space-4);
}

.staleness-banner--warning {
  background: var(--color-warning-soft);
  border: 1px solid var(--color-warning);
}

.staleness-banner strong {
  display: block;
  margin-bottom: var(--space-1);
  color: var(--color-warning);
}

.staleness-banner p {
  margin: 0;
  font-size: var(--font-size-meta);
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

.empty-state {
  padding: var(--space-5);
  text-align: center;
  color: var(--text-secondary);
}

.empty-state--compact {
  padding: var(--space-3);
}
</style>

<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import { formatNumber } from '../lib/dashboard-utils.js';
import {
  dashboardStore,
  toggleStrategyPerspectives,
  loadStrategyScoreboard as bridgeLoadStrategyScoreboard,
  loadStrategyAttribution as bridgeLoadStrategyAttribution,
} from '../store.js';

const { t } = useI18n();

// Display-only pagination; backend order is preserved at all times.
const PAGE_SIZE = 20;

const PERSONA_DEFS = [
  { kind: 'ValueLeft', field: 'value_left_score', nameKey: 'perspectives.personas.valueLeft' },
  { kind: 'TrendPullback', field: 'trend_pullback_score', nameKey: 'perspectives.personas.trendPullback' },
  { kind: 'TrendBreakout', field: 'trend_breakout_score', nameKey: 'perspectives.personas.trendBreakout' },
  { kind: 'MomentumRight', field: 'momentum_right_score', nameKey: 'perspectives.personas.momentumRight' },
];

const PERSONA_NAME_KEYS = Object.fromEntries(PERSONA_DEFS.map((p) => [p.kind, p.nameKey]));

// Local reactive state
const showAll = ref(false);
const attributionOpen = ref(false);
const attributionSymbol = ref('');
const loadedKey = ref('');

const isOpen = computed(() => dashboardStore.showStrategyPerspectives);
const scoreboard = computed(() => dashboardStore.strategyScoreboard);
const loading = computed(() => dashboardStore.strategyScoreboardLoading);
const error = computed(() => dashboardStore.strategyScoreboardError);
const attribution = computed(() => dashboardStore.strategyAttribution);
const attributionLoading = computed(() => dashboardStore.strategyAttributionLoading);
const attributionError = computed(() => dashboardStore.strategyAttributionError);

const entries = computed(() => (Array.isArray(scoreboard.value?.entries) ? scoreboard.value.entries : []));
const dataDate = computed(() => scoreboard.value?.date || '');
const scopeLabel = computed(() => (dashboardStore.selectedScope || 'global').toUpperCase());

const visibleEntries = computed(() => (
  showAll.value ? entries.value : entries.value.slice(0, PAGE_SIZE)
));

const loadKey = computed(() => `${dashboardStore.selectedScope}|${dashboardStore.selectedReportDate || 'latest'}`);

const attributions = computed(() => (
  Array.isArray(attribution.value?.attributions) ? attribution.value.attributions : []
));

function personaName(kind) {
  const key = PERSONA_NAME_KEYS[kind];
  return key ? t(key) : kind;
}

function personasFor(entry) {
  return PERSONA_DEFS.map((def) => ({
    kind: def.kind,
    nameKey: def.nameKey,
    score: Number(entry?.[def.field]) || 0,
    isBest: entry?.best_strategy === def.kind,
  }));
}

function barWidth(score) {
  const value = Number(score);
  if (!Number.isFinite(value)) return '0%';
  return `${Math.max(0, Math.min(100, value))}%`;
}

// Body scroll lock + lazy load on open
watch(isOpen, (open) => {
  document.body.classList.toggle('body--perspectives-open', open);
  if (open && loadedKey.value !== loadKey.value && !loading.value) {
    loadedKey.value = loadKey.value;
    bridgeLoadStrategyScoreboard();
  }
});

// Keyboard: ESC closes attribution first, then the overlay
function handleKeydown(event) {
  if (event.key !== 'Escape') return;
  if (attributionOpen.value) {
    closeAttribution();
  } else if (isOpen.value) {
    close();
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown);
  document.body.classList.remove('body--perspectives-open');
});

function close() {
  attributionOpen.value = false;
  toggleStrategyPerspectives(false);
}

function reload() {
  loadedKey.value = loadKey.value;
  bridgeLoadStrategyScoreboard();
}

function openAttribution(symbol) {
  if (!symbol) return;
  attributionSymbol.value = symbol;
  attributionOpen.value = true;
  bridgeLoadStrategyAttribution(symbol);
}

function closeAttribution() {
  attributionOpen.value = false;
}
</script>

<template>
  <section
    class="perspectives"
    :class="{ 'perspectives--open': isOpen }"
    :aria-hidden="isOpen ? 'false' : 'true'"
  >
    <button
      class="perspectives__backdrop"
      type="button"
      :aria-label="t('common.close')"
      @click="close"
    ></button>

    <div class="perspectives__panel" role="dialog" aria-modal="true" aria-labelledby="strategyPerspectivesTitle">
      <div class="perspectives__header">
        <div>
          <p class="eyebrow">{{ t('perspectives.eyebrow') }}</p>
          <h2 id="strategyPerspectivesTitle">{{ t('perspectives.title') }}</h2>
          <p class="panel__lede">{{ t('perspectives.lede') }}</p>
        </div>
        <div class="panel__actions">
          <span v-if="dataDate" class="pill pill--outline">{{ t('perspectives.dataDate', { date: dataDate }) }}</span>
          <span class="pill pill--outline">{{ t('perspectives.scopeLabel', { scope: scopeLabel }) }}</span>
          <button
            class="button button--secondary button--compact"
            :disabled="loading"
            @click="reload"
          >
            {{ loading ? t('perspectives.refreshing') : t('perspectives.reload') }}
          </button>
          <button class="button button--secondary button--compact" @click="close">
            {{ t('common.close') }}
          </button>
        </div>
      </div>

      <div class="perspectives__body">
        <!-- Loading state -->
        <div v-if="loading && !entries.length" class="empty-state">
          <p>{{ t('perspectives.loading') }}</p>
        </div>

        <!-- Error state -->
        <section v-else-if="error" class="notice notice--error notice--inline">
          <div>
            <strong>{{ t('perspectives.errorTitle') }}</strong>
            <p>{{ error }}</p>
          </div>
        </section>

        <!-- Empty state -->
        <div v-else-if="!entries.length" class="empty-state">
          <p>{{ t('perspectives.empty') }}</p>
        </div>

        <!-- Symbol sections (backend order preserved) -->
        <template v-else>
          <section v-for="entry in visibleEntries" :key="entry.symbol" class="symbol-section">
            <header class="symbol-section__header">
              <div class="symbol-section__identity">
                <span class="symbol-section__code">{{ entry.symbol }}</span>
                <span class="symbol-section__name">{{ entry.name || entry.symbol }}</span>
              </div>
              <div class="symbol-section__meta">
                <span class="pill pill--outline">{{ t('perspectives.alignment', { count: entry.alignment ?? 0 }) }}</span>
                <span class="pill pill--outline">{{ t('perspectives.confidence', { value: formatNumber(entry.confidence ?? 0, 2) }) }}</span>
                <button
                  class="button button--secondary button--compact"
                  :disabled="attributionLoading"
                  @click="openAttribution(entry.symbol)"
                >
                  {{ t('perspectives.attribution') }}
                </button>
              </div>
            </header>

            <div class="persona-grid">
              <article
                v-for="persona in personasFor(entry)"
                :key="persona.kind"
                class="persona-card"
                :class="{ 'persona-card--best': persona.isBest }"
              >
                <header class="persona-card__header">
                  <span class="persona-card__name">{{ t(persona.nameKey) }}</span>
                  <span v-if="persona.isBest" class="persona-card__badge">{{ t('perspectives.bestBadge') }}</span>
                </header>
                <span class="persona-card__score">{{ formatNumber(persona.score, 1) }}</span>
                <div class="persona-card__bar">
                  <div class="persona-card__fill" :style="{ width: barWidth(persona.score) }"></div>
                </div>
              </article>
            </div>

            <div v-if="entry.scenario_scores?.length" class="scenario-chips">
              <span v-for="scenario in entry.scenario_scores" :key="scenario.key" class="scenario-chip">
                {{ scenario.label }} {{ formatNumber(scenario.score, 1) }}
              </span>
            </div>
          </section>

          <!-- Display-only expand toggle -->
          <div v-if="entries.length > PAGE_SIZE" class="perspectives__expand">
            <button class="button button--secondary button--compact" @click="showAll = !showAll">
              {{ showAll ? t('perspectives.showLess') : t('perspectives.showAll', { count: entries.length }) }}
            </button>
          </div>
        </template>
      </div>
    </div>

    <!-- Attribution right slide-over (explicit user click only) -->
    <div v-if="attributionOpen" class="attribution" role="dialog" aria-modal="true" aria-labelledby="strategyAttributionTitle">
      <button
        class="attribution__backdrop"
        type="button"
        :aria-label="t('common.close')"
        @click="closeAttribution"
      ></button>

      <article class="attribution__panel">
        <header class="attribution__header">
          <div>
            <p class="eyebrow">{{ t('perspectives.attributionTitle') }}</p>
            <h3 id="strategyAttributionTitle">{{ attributionSymbol }}</h3>
            <p class="panel__lede">{{ t('perspectives.attributionLede') }}</p>
          </div>
          <button class="button button--secondary button--compact" @click="closeAttribution">
            {{ t('common.close') }}
          </button>
        </header>

        <div v-if="attributionLoading" class="empty-state">
          <p>{{ t('perspectives.attributionLoading') }}</p>
        </div>

        <section v-else-if="attributionError" class="notice notice--error notice--inline">
          <div>
            <strong>{{ t('perspectives.attributionError') }}</strong>
            <p>{{ attributionError }}</p>
          </div>
        </section>

        <template v-else>
          <section v-for="attr in attributions" :key="attr.kind" class="attr-strategy">
            <header class="attr-strategy__header">
              <span class="attr-strategy__name">{{ personaName(attr.kind) }}</span>
              <span class="attr-strategy__scores">
                {{ t('perspectives.recomputed') }} {{ formatNumber(attr.recomputed_score, 2) }}
                · {{ t('perspectives.stored') }} {{ formatNumber(attr.stored_score, 2) }}
                · {{ t('perspectives.drift') }} {{ formatNumber(attr.drift, 2) }}
              </span>
            </header>

            <table v-if="attr.drivers?.length" class="attr-drivers">
              <thead>
                <tr>
                  <th>{{ t('perspectives.factor') }}</th>
                  <th>{{ t('perspectives.value') }}</th>
                  <th>{{ t('perspectives.contribution') }}</th>
                  <th>{{ t('perspectives.note') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="driver in attr.drivers" :key="driver.factor">
                  <td>{{ driver.factor }}</td>
                  <td>{{ formatNumber(driver.value, 2) }}</td>
                  <td>{{ formatNumber(driver.contribution, 2) }}</td>
                  <td>{{ driver.note }}</td>
                </tr>
              </tbody>
            </table>
            <p v-else class="attr-strategy__empty">{{ t('perspectives.noDrivers') }}</p>
          </section>
        </template>
      </article>
    </div>
  </section>
</template>

<style scoped>
.perspectives {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-5);
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transition:
    opacity 180ms ease,
    visibility 180ms ease;
  z-index: var(--layer-guide-viewer, 30);
}

.perspectives--open {
  opacity: 1;
  visibility: visible;
  pointer-events: auto;
}

.perspectives__backdrop {
  position: absolute;
  inset: 0;
  border: 0;
  padding: 0;
  background: linear-gradient(180deg, rgba(7, 10, 12, 0.56), var(--color-overlay));
  cursor: pointer;
}

.perspectives__panel {
  position: relative;
  width: min(100%, var(--guide-viewer-width, 76rem));
  max-height: var(--guide-viewer-height, calc(100vh - (var(--space-7) * 2)));
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  border: 1px solid var(--color-border-strong);
  border-radius: calc(1.5rem + 0.25rem);
  background:
    linear-gradient(135deg, rgba(79, 140, 255, 0.1), transparent 34%),
    linear-gradient(180deg, rgba(18, 22, 27, 0.98), rgba(14, 17, 21, 0.96));
  box-shadow: var(--shadow-strong);
  overflow: hidden;
}

.perspectives__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-4);
  padding: var(--space-5);
  border-bottom: 1px solid var(--color-border);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0));
}

.perspectives__body {
  min-height: 0;
  overflow-y: auto;
  padding: var(--space-5);
  display: grid;
  gap: var(--space-4);
  align-content: start;
}

.perspectives__expand {
  display: flex;
  justify-content: center;
  padding-top: var(--space-2);
}

/* Symbol sections */
.symbol-section {
  display: grid;
  gap: var(--space-3);
  padding: var(--space-4);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-lg);
  background: var(--panel-bg);
}

.symbol-section__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.symbol-section__identity {
  display: flex;
  align-items: baseline;
  gap: var(--space-3);
}

.symbol-section__code {
  font-family: var(--font-mono);
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.02em;
}

.symbol-section__name {
  font-size: 0.85rem;
  color: var(--text-secondary);
}

.symbol-section__meta {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

/* Persona cards */
.persona-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-3);
}

.persona-card {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-3);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-md);
  background: var(--color-surface-raised);
  transition: border-color 180ms ease, background-color 180ms ease;
}

.persona-card--best {
  border-color: var(--color-accent-border);
  background: var(--color-accent-soft);
}

.persona-card__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-2);
}

.persona-card__name {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-secondary);
}

.persona-card__badge {
  padding: 0 var(--space-2);
  border-radius: var(--radius-pill);
  background: var(--color-accent);
  color: var(--color-bg);
  font-size: 0.7rem;
  font-weight: 700;
  line-height: 1.6;
  white-space: nowrap;
}

.persona-card__score {
  font-family: var(--font-mono);
  font-size: 1.6rem;
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1;
}

.persona-card__bar {
  height: 4px;
  border-radius: 2px;
  background: var(--score-bar-bg);
  overflow: hidden;
}

.persona-card__fill {
  height: 100%;
  border-radius: 2px;
  background: var(--color-accent);
  transition: width 0.5s ease;
}

/* Scenario chips */
.scenario-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.scenario-chip {
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-pill);
  background: var(--color-surface-raised);
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  white-space: nowrap;
}

/* Attribution slide-over */
.attribution {
  position: fixed;
  inset: 0;
  z-index: var(--layer-modal, 40);
  display: flex;
  justify-content: flex-end;
}

.attribution__backdrop {
  position: absolute;
  inset: 0;
  border: 0;
  padding: 0;
  background: var(--color-overlay);
  cursor: pointer;
}

.attribution__panel {
  position: relative;
  width: 30rem;
  max-width: 92vw;
  height: 100vh;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-5);
  background: var(--panel-bg-secondary);
  border-left: 1px solid var(--color-border-strong);
  box-shadow: -8px 0 40px rgba(0, 0, 0, 0.6);
}

.attribution__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-3);
  padding-bottom: var(--space-4);
  border-bottom: 1px solid var(--panel-border);
}

.attribution__header h3 {
  margin: 0;
  font-family: var(--font-mono);
  font-size: 1.35rem;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.02em;
}

.attr-strategy {
  display: grid;
  gap: var(--space-3);
  padding: var(--space-3);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-md);
  background: var(--color-surface-raised);
}

.attr-strategy__header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.attr-strategy__name {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-primary);
}

.attr-strategy__scores {
  font-family: var(--font-mono);
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.attr-drivers {
  width: 100%;
  border-collapse: collapse;
  font-family: var(--font-mono);
  font-size: 0.75rem;
}

.attr-drivers th {
  padding: var(--space-1) var(--space-2);
  color: var(--text-secondary);
  font-weight: 600;
  text-align: left;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid var(--panel-border);
}

.attr-drivers td {
  padding: var(--space-1) var(--space-2);
  color: var(--text-primary);
  border-bottom: 1px solid var(--panel-border);
  overflow-wrap: anywhere;
}

.attr-drivers tr:last-child td {
  border-bottom: 0;
}

.attr-strategy__empty {
  margin: 0;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

/* Shared primitives (mirroring UsageGuidesPanel conventions) */
.panel__lede {
  margin: 0;
  color: var(--color-text-muted);
  line-height: 1.65;
}

.panel__actions {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: var(--space-1) var(--space-3);
  border-radius: var(--space-3);
  font-size: 0.8rem;
  font-weight: 500;
}

.pill--outline {
  border: 1px solid var(--pill-outline-border);
  color: var(--text-secondary);
  background: transparent;
}

.eyebrow {
  font-size: 0.8rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--color-secondary);
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

.notice {
  padding: var(--space-3) var(--space-4);
  border-radius: var(--panel-radius);
}

.notice--error {
  background: var(--tone-negative-bg);
  border: 1px solid var(--tone-negative);
}

.notice--inline {
  margin: 0;
}

.notice strong {
  display: block;
  margin-bottom: var(--space-1);
  color: var(--text-primary);
}

.notice p {
  margin: var(--space-1) 0;
  font-size: 0.95rem;
  color: var(--text-secondary);
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
  .perspectives {
    padding: var(--space-4);
  }

  .persona-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .perspectives {
    padding: var(--space-3);
  }

  .perspectives__panel {
    max-height: calc(100vh - (var(--space-3) * 2));
  }

  .perspectives__header,
  .perspectives__body {
    padding: var(--space-4);
  }

  .persona-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>

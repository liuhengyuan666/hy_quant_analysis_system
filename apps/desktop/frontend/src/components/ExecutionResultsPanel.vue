<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore } from '../store.js';

const { t } = useI18n();

const STATE_ORDER = ['INCREASE', 'MAINTAIN', 'AVOID', 'REDUCE', 'SKIP'];

const STATE_META = {
  INCREASE: { tone: 'positive', icon: '\u25B6' },
  MAINTAIN: { tone: 'neutral', icon: '\u25CB' },
  AVOID: { tone: 'warning', icon: '\u25B2' },
  REDUCE: { tone: 'negative', icon: '\u25BC' },
  SKIP: { tone: 'outline', icon: '\u2715' },
};

const STATE_I18N = {
  INCREASE: 'execution.stateIncrease',
  MAINTAIN: 'execution.stateMaintain',
  AVOID: 'execution.stateAvoid',
  REDUCE: 'execution.stateReduce',
  SKIP: 'execution.stateSkip',
};

const results = computed(() => dashboardStore.executionResults || []);
const snapshot = computed(() => dashboardStore.snapshot);

const hasResults = computed(() => results.value.length > 0);

const groupedResults = computed(() => {
  const groups = [];
  const byState = {};

  for (const item of results.value) {
    const state = item.state || 'SKIP';
    if (!byState[state]) byState[state] = [];
    byState[state].push(item);
  }

  for (const state of STATE_ORDER) {
    if (byState[state] && byState[state].length > 0) {
      groups.push({
        state,
        items: byState[state],
        count: byState[state].length,
        meta: STATE_META[state] || { tone: 'outline', icon: '\u2014' },
      });
    }
  }

  return groups;
});

const totalCount = computed(() => results.value.length);

function reasonLabel(reason) {
  // Convert ReasonTag strings like "StrongClose" to readable labels
  const name = String(reason ?? '')
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/_/g, ' ');
  const key = `execution.reason_${reason}`;
  // Fallback to formatted reason string if no i18n key exists
  return name;
}
</script>

<template>
  <article class="panel">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('execution.eyebrow') }}</p>
        <h2>{{ t('execution.title') }}</h2>
        <p v-if="hasResults" class="panel__lede">
          {{ t('execution.lede', { count: totalCount }) }}
        </p>
      </div>
      <div v-if="hasResults" class="panel__actions">
        <span class="panel__meta">{{ t('execution.candidates', { count: totalCount }) }}</span>
      </div>
    </div>

    <!-- Empty state -->
    <div v-if="!hasResults" class="empty-state">
      <div class="empty-state__log">
        <span class="empty-state__timestamp">{{ snapshot?.report_date || '-' }} 15:00:00</span>
        <span class="empty-state__level">INFO</span>
        <span class="empty-state__message">Engine ran successfully. No signal triggered for this session.</span>
      </div>
      <p class="empty-state__hint">{{ t('execution.noResultsHint') }}</p>
    </div>

    <!-- Timeline view -->
    <template v-else>
      <div class="execution-timeline">
        <div
          v-for="group in groupedResults"
          :key="group.state"
          class="execution-group"
        >
          <!-- Group header -->
          <div class="execution-group__header">
            <span
              class="execution-group__pill"
              :class="`execution-group__pill--${group.meta.tone}`"
            >
              <span class="execution-group__icon">{{ group.meta.icon }}</span>
              <span>{{ group.count }}</span>
            </span>
            <span class="execution-group__label">{{ t(STATE_I18N[group.state]) }}</span>
            <span class="execution-group__count">
              {{ t('execution.symbolsCount', { count: group.count }) }}
            </span>
          </div>

          <!-- Cards -->
          <div class="execution-cards">
            <div
              v-for="(item, idx) in group.items"
              :key="`${group.state}-${idx}`"
              class="execution-card"
              :class="`execution-card--${group.meta.tone}`"
            >
              <div class="execution-card__body">
                <div class="execution-card__main">
                  <strong class="execution-card__symbol">{{ item.symbol }}</strong>
                  <span class="execution-card__divider" aria-hidden="true">&middot;</span>
                  <span
                    class="execution-card__state"
                    :class="`execution-card__state--${group.meta.tone}`"
                  >
                    {{ t(STATE_I18N[group.state]) }}
                  </span>
                </div>
                <div v-if="item.reasons && item.reasons.length" class="execution-card__reasons">
                  <span
                    v-for="reason in item.reasons"
                    :key="reason"
                    class="execution-card__reason-tag"
                  >{{ reasonLabel(reason) }}</span>
                </div>
                <div v-else class="execution-card__reasons">
                  <span class="execution-card__reason-tag execution-card__reason-tag--empty">
                    {{ t('execution.noReasons') }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
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
  gap: var(--space-4);
  margin-bottom: var(--space-4);
  flex-wrap: wrap;
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
  text-transform: uppercase;
  letter-spacing: 0.08em;
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

.empty-state__log {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  font-family: var(--font-mono);
  font-size: var(--font-size-label);
  margin-bottom: var(--space-3);
  flex-wrap: wrap;
}

.empty-state__timestamp {
  color: var(--text-secondary);
  opacity: 0.7;
}

.empty-state__level {
  color: var(--color-accent);
  font-weight: 600;
}

.empty-state__message {
  color: var(--text-primary);
}

.empty-state__hint {
  margin-top: var(--space-2);
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  opacity: 0.7;
}

/* ── Timeline ─────────────────────────────────────────────── */

.execution-timeline {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

/* ── Group ────────────────────────────────────────────────── */

.execution-group__header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) 0;
  border-bottom: 1px solid var(--panel-border);
  margin-bottom: var(--space-2);
}

.execution-group__pill {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  min-width: 2.5rem;
  height: 1.5rem;
  padding: 0 var(--space-2);
  border-radius: var(--radius-pill);
  font-size: var(--font-size-label);
  font-weight: 600;
  font-family: var(--font-mono);
  justify-content: center;
}

.execution-group__pill--positive {
  background: var(--tone-positive-bg);
  color: var(--tone-positive);
  border: 1px solid var(--tone-positive);
}

.execution-group__pill--warning {
  background: var(--color-warning-soft);
  color: var(--color-warning);
  border: 1px solid var(--color-warning);
}

.execution-group__pill--neutral {
  background: var(--tone-neutral-bg);
  color: var(--tone-neutral);
  border: 1px solid var(--color-border-strong);
}

.execution-group__pill--negative {
  background: var(--tone-negative-bg);
  color: var(--tone-negative);
  border: 1px solid var(--tone-negative);
}

.execution-group__pill--outline {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--color-border-strong);
}

.execution-group__icon {
  font-size: 0.65rem;
  line-height: 1;
}

.execution-group__label {
  font-weight: 600;
  color: var(--text-primary);
  font-size: var(--font-size-meta);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.execution-group__count {
  margin-left: auto;
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

/* ── Cards ────────────────────────────────────────────────── */

.execution-cards {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.execution-card {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  flex: 1 1 14rem;
  min-width: 12rem;
  max-width: 20rem;
  transition: border-color 0.2s ease;
}

.execution-card:hover {
  border-color: var(--color-accent-border);
}

.execution-card--positive {
  border-left: 3px solid var(--tone-positive);
}

.execution-card--warning {
  border-left: 3px solid var(--color-warning);
}

.execution-card--neutral {
  border-left: 3px solid var(--color-neutral);
}

.execution-card--negative {
  border-left: 3px solid var(--tone-negative);
}

.execution-card--outline {
  border-left: 3px solid var(--color-border-strong);
}

.execution-card__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.execution-card__main {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.execution-card__symbol {
  font-family: var(--font-mono);
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
}

.execution-card__divider {
  color: var(--text-secondary);
  opacity: 0.4;
}

.execution-card__state {
  font-size: var(--font-size-label);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
}

.execution-card__state--positive {
  background: var(--tone-positive-bg);
  color: var(--tone-positive);
}

.execution-card__state--warning {
  background: var(--color-warning-soft);
  color: var(--color-warning);
}

.execution-card__state--neutral {
  background: var(--tone-neutral-bg);
  color: var(--tone-neutral);
}

.execution-card__state--negative {
  background: var(--tone-negative-bg);
  color: var(--tone-negative);
}

.execution-card__state--outline {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--color-border-strong);
}

.execution-card__reasons {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}

.execution-card__reason-tag {
  display: inline-block;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-surface-raised);
  border: 1px solid var(--color-border);
  color: var(--text-secondary);
  font-size: var(--font-size-label);
  font-family: var(--font-mono);
  letter-spacing: 0.02em;
  line-height: 1.3;
}

.execution-card__reason-tag--empty {
  font-family: var(--font-body);
  font-style: italic;
  opacity: 0.6;
  border-style: dashed;
}

/* ── Responsive ─────────────────────────────────────────── */

@media (max-width: 720px) {
  .execution-card {
    flex: 1 1 100%;
    max-width: none;
  }

  .execution-cards {
    flex-direction: column;
  }
}
</style>

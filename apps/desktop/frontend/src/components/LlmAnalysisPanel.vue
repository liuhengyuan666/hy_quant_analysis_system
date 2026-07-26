<script setup>
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  dashboardStore,
  updateLlmAnalysis,
  updateLlmLoading,
  updateLlmError,
} from '../store.js';
import { marked } from 'marked';
import { llmApi } from '../api/tauri.js';

const { t } = useI18n();

const activeAction = ref('');
const adversarialLevel = ref('standard');
const loading = computed(() => dashboardStore.llmLoading);
const error = computed(() => dashboardStore.llmError);
const analysis = computed(() => dashboardStore.llmAnalysis);

const actions = [
  { key: 'market_story', label: t('research.marketStory') },
  { key: 'explain_decision', label: t('research.explainDecision') },
  { key: 'preclose_review', label: t('research.precloseReview') },
  { key: 'risk_view', label: t('research.riskView') },
  { key: 'devils_advocate', label: t('research.devilsAdvocate') },
  { key: 'portfolio_review', label: t('research.portfolioReview') },
  { key: 'market_adversarial_lens', label: t('research.marketAdversarialLens') },
];

async function handleGenerate(action) {
  activeAction.value = action;
  updateLlmLoading(true);
  updateLlmError('');
  try {
    const result = await llmApi.analyzeWithLlm(
      dashboardStore.selectedScope,
      action,
      adversarialLevel.value
    );
    updateLlmAnalysis(result);
  } catch (err) {
    console.error('[ResearchPanel] Analysis failed:', err);
    updateLlmError(err?.toString?.() || t('research.analysisFailed'));
  } finally {
    updateLlmLoading(false);
  }
}

function renderMarkdown(text) {
  if (!text) return '';
  // Escape raw HTML in LLM output BEFORE parsing, so injected markup renders
  // as text while marked's own generated tags survive.
  const escaped = String(text)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');

  return marked.parse(escaped);
}
</script>

<template>
  <article class="llm-panel panel">
    <!-- Header -->
    <div class="llm-panel__header panel__header">
      <div>
        <p class="eyebrow">{{ t('research.eyebrow') }}</p>
        <h2>{{ t('research.panelTitle') }}</h2>
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="llm-panel__actions">
      <button
        v-for="act in actions"
        :key="act.key"
        class="llm-panel__action-btn"
        :class="{ 'llm-panel__action-btn--active': activeAction === act.key }"
        @click="handleGenerate(act.key)"
        :disabled="loading"
      >
        {{ act.label }}
      </button>
    </div>

    <!-- Adversarial Inject Level -->
    <div class="llm-panel__adversarial">
      <span class="llm-panel__adversarial-label">{{ t('research.adversarialLevel') }}</span>
      <div class="llm-panel__adversarial-options" role="radiogroup">
        <button
          v-for="level in ['full', 'standard', 'compact', 'none']"
          :key="level"
          class="llm-panel__adversarial-btn"
          :class="{ 'llm-panel__adversarial-btn--active': adversarialLevel === level }"
          role="radio"
          :aria-checked="adversarialLevel === level"
          @click="adversarialLevel = level"
        >
          {{ t(`research.adversarial${level.charAt(0).toUpperCase() + level.slice(1)}`) }}
        </button>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="llm-panel__loading">
      <div class="llm-panel__spinner"></div>
      <p>{{ t('research.analyzing') }}</p>
    </div>

    <!-- Error -->
    <div v-else-if="error" class="llm-panel__error">
      <p class="llm-panel__error-title">{{ t('common.error') }}</p>
      <p class="llm-panel__error-body">{{ error }}</p>
    </div>

    <!-- Content -->
    <div v-else-if="analysis" class="llm-panel__content">
      <div
        v-if="analysis.markdown"
        class="llm-panel__markdown"
        v-html="renderMarkdown(analysis.markdown)"
      ></div>
      <div v-else-if="analysis.placeholder" class="llm-panel__placeholder">
        {{ analysis.markdown }}
      </div>
    </div>

    <!-- Empty -->
    <div v-else class="llm-panel__empty">
      <p>{{ t('research.selectAction') }}</p>
    </div>
  </article>
</template>

<style scoped>
.llm-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow-y: auto;
}

.llm-panel__header {
  flex-shrink: 0;
  padding: var(--space-4) var(--space-4);
  border-bottom: 1px solid var(--panel-border);
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.llm-panel__actions {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--panel-border);
  flex-shrink: 0;
}

.llm-panel__action-btn {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-sm);
  padding: var(--space-2) var(--space-3);
  color: var(--text-primary);
  font-size: var(--font-size-label);
  font-weight: 500;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.llm-panel__action-btn:hover:not(:disabled) {
  background: var(--color-accent-soft);
  border-color: var(--color-accent);
}

.llm-panel__action-btn--active {
  background: var(--color-accent-soft);
  border-color: var(--color-accent);
  color: var(--color-accent);
}

.llm-panel__action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.llm-panel__adversarial {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--panel-border);
  flex-shrink: 0;
}

.llm-panel__adversarial-label {
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
  white-space: nowrap;
}

.llm-panel__adversarial-options {
  display: flex;
  gap: var(--space-1);
}

.llm-panel__adversarial-btn {
  background: transparent;
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-sm);
  padding: var(--space-1) var(--space-2);
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.llm-panel__adversarial-btn:hover {
  background: var(--color-accent-soft);
  border-color: var(--color-accent);
}

.llm-panel__adversarial-btn--active {
  background: var(--color-accent-soft);
  border-color: var(--color-accent);
  color: var(--color-accent);
}

.llm-panel__content {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-3) var(--space-4);
}

.llm-panel__markdown {
  font-size: var(--font-size-body);
  line-height: var(--line-height-body);
  color: var(--text-primary);
}

.llm-panel__markdown :deep(h1),
.llm-panel__markdown :deep(h2),
.llm-panel__markdown :deep(h3) {
  font-family: var(--font-display);
  margin: var(--space-4) 0 var(--space-2);
}

.llm-panel__markdown :deep(h1) {
  font-size: var(--font-size-title);
}

.llm-panel__markdown :deep(h2) {
  font-size: 1.15rem;
}

.llm-panel__markdown :deep(h3) {
  font-size: var(--font-size-meta);
}

.llm-panel__markdown :deep(code) {
  background: var(--panel-bg-secondary);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  font-size: 0.9em;
}

.llm-panel__markdown :deep(strong) {
  color: var(--color-accent);
}

.llm-panel__markdown :deep(p) {
  margin: var(--space-2) 0;
}

.llm-panel__markdown :deep(ul),
.llm-panel__markdown :deep(ol) {
  margin: var(--space-2) 0;
  padding-left: var(--space-5);
  color: var(--text-primary);
}

.llm-panel__markdown :deep(li) {
  margin: var(--space-1) 0;
}

.llm-panel__markdown :deep(li)::marker {
  color: var(--color-accent);
}

.llm-panel__markdown :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: var(--space-3) 0;
  font-size: var(--font-size-label);
}

.llm-panel__markdown :deep(th),
.llm-panel__markdown :deep(td) {
  border: 1px solid var(--panel-border);
  padding: var(--space-2) var(--space-3);
  text-align: left;
}

.llm-panel__markdown :deep(th) {
  background: var(--panel-bg-secondary);
  color: var(--text-primary);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.llm-panel__markdown :deep(td) {
  color: var(--text-secondary);
}

.llm-panel__markdown :deep(tbody tr:hover td) {
  background: var(--color-surface-raised);
}

.llm-panel__markdown :deep(blockquote) {
  margin: var(--space-3) 0;
  padding: var(--space-2) var(--space-4);
  border-left: 3px solid var(--color-accent);
  background: var(--color-accent-soft);
  color: var(--text-secondary);
  border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
}

.llm-panel__markdown :deep(blockquote p) {
  margin: 0;
}

.llm-panel__markdown :deep(pre) {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-sm);
  padding: var(--space-3) var(--space-4);
  margin: var(--space-3) 0;
  overflow-x: auto;
}

.llm-panel__markdown :deep(pre code) {
  background: transparent;
  padding: 0;
  border-radius: 0;
  font-family: var(--font-mono);
  font-size: var(--font-size-label);
  color: var(--text-primary);
}

.llm-panel__loading,
.llm-panel__error,
.llm-panel__empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-4);
  padding: var(--space-6);
  text-align: center;
  color: var(--text-secondary);
}

.llm-panel__spinner {
  width: 2.5rem;
  height: 2.5rem;
  border: 3px solid var(--color-accent-soft);
  border-top-color: var(--color-accent);
  border-radius: 50%;
  animation: llm-panel-spin 0.9s linear infinite;
}

@keyframes llm-panel-spin {
  to {
    transform: rotate(360deg);
  }
}

.llm-panel__error-title {
  color: var(--color-negative);
  font-weight: 600;
}

.llm-panel__error-body {
  font-size: var(--font-size-meta);
  max-width: 100%;
  word-break: break-word;
}

.llm-panel__placeholder {
  color: var(--text-secondary);
  font-style: italic;
  padding: var(--space-4);
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

<script setup>
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  dashboardStore,
  updateLlmAnalysis,
  updateLlmLoading,
  updateLlmError,
  updateLlmConfig,
  toggleLlmPanel,
} from '../store.js';
import { llmApi } from '../api/tauri.js';
import { formatNumber } from '../lib/dashboard-utils.js';
import SkillBadge from './SkillBadge.vue';

const { t } = useI18n();

const emit = defineEmits(['close', 'reanalyze']);

const activeTab = ref('conclusion');
const exporting = ref(false);
const configForm = ref({
  baseUrl: '',
  model: '',
  apiKey: '',
});

const analysis = computed(() => dashboardStore.llmAnalysis);
const loading = computed(() => dashboardStore.llmLoading);
const error = computed(() => dashboardStore.llmError);
const selectedAgent = computed(() => dashboardStore.selectedAgent);
const snapshot = computed(() => dashboardStore.snapshot);

const tabs = computed(() => [
  { key: 'conclusion', label: t('llm.conclusion') },
  { key: 'regime', label: t('llm.regime') },
  { key: 'execution', label: t('llm.execution') },
  { key: 'risk', label: t('llm.risk') },
]);

const skillName = computed(() => analysis.value?.skill || '');
const triggered = computed(() => analysis.value?.triggered ?? false);
const scope = computed(() => analysis.value?.scope || '');

const regimeAnalysis = computed(() => analysis.value?.regime_analysis || {});
const llmAnalysisText = computed(() => analysis.value?.llm_analysis || '');
const tokenUsage = computed(() => analysis.value?.token_usage || {});
const riskAssessment = computed(() => regimeAnalysis.value?.risk_assessment || {});
const isPlaceholder = computed(() => analysis.value?.placeholder === true);
const isConfigured = computed(() => dashboardStore.llmConfig?.configured ?? false);

const keyDrivers = computed(() => regimeAnalysis.value?.key_drivers || []);

// Pre-populate config form from existing llmConfig (never pre-fill apiKey)
watch(
  () => dashboardStore.llmConfig,
  (cfg) => {
    if (cfg) {
      configForm.value.baseUrl = cfg.base_url || '';
      configForm.value.model = cfg.model || '';
    }
  },
  { immediate: true }
);

function handleClose() {
  emit('close');
}

function handleReanalyze() {
  emit('reanalyze', selectedAgent.value);
}

async function handleExportMarkdown() {
  if (!analysis.value || exporting.value) return;
  exporting.value = true;
  try {
    const result = await llmApi.exportLlmAnalysis(
      dashboardStore.selectedScope,
      snapshot.value?.report_date || new Date().toISOString().slice(0, 10),
      analysis.value
    );
    updateLlmError('');
    alert(t('llm.markdownExported') + ': ' + result.output_path);
  } catch (err) {
    console.error('[LlmPanel] Export failed:', err);
    updateLlmError(err?.toString?.() || t('llm.exportFailed'));
  } finally {
    exporting.value = false;
  }
}

async function handleSaveConfig() {
  try {
    await llmApi.setLlmConfig(
      configForm.value.baseUrl,
      configForm.value.model,
      60
    );
    if (configForm.value.apiKey) {
      await llmApi.setLlmApiKey(configForm.value.apiKey);
    }
    alert(t('llm.configSaved'));
    // Refresh status
    const status = await llmApi.getStatus();
    updateLlmConfig(status);
  } catch (err) {
    console.error('[LlmPanel] Config save failed:', err);
    updateLlmError(err?.toString?.() || t('llm.configSaveFailed'));
  }
}

/**
 * Lightweight markdown-to-HTML renderer with XSS sanitization.
 * Two-layer defense: 1) escape raw HTML, 2) convert markdown, 3) strip dangerous tags.
 */
function renderMarkdown(text) {
  if (!text) return '';

  // Step 1: Escape raw HTML entities so any HTML in the LLM output is rendered as text
  let html = String(text)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');

  // Step 2: Convert markdown syntax to HTML (on already-escaped text)
  html = html
    .replace(/^# (.*$)/gim, '<h1>$1</h1>')
    .replace(/^## (.*$)/gim, '<h2>$1</h2>')
    .replace(/^### (.*$)/gim, '<h3>$1</h3>')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.*?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\n/g, '<br>');

  // Step 3: Defense-in-depth — strip any dangerous tags that might have slipped through
  const unsafePatterns = [
    /<script[^>]*>[\s\S]*?<\/script>/gi,
    /<iframe[^>]*>[\s\S]*?<\/iframe>/gi,
    /<object[^>]*>[\s\S]*?<\/object>/gi,
    /<embed[^>]*>/gi,
    /<form[^>]*>[\s\S]*?<\/form>/gi,
    /on\w+\s*=\s*["']?[^"'>]*["']?/gi,
    /javascript:/gi,
  ];
  unsafePatterns.forEach((pattern) => {
    html = html.replace(pattern, '');
  });

  return html;
}
</script>

<template>
  <div class="llm-panel" role="dialog" aria-modal="true">
    <button
      class="llm-panel__backdrop"
      type="button"
      :aria-label="t('common.close')"
      @click="handleClose"
    ></button>

    <article class="llm-panel__sheet panel">
      <!-- Header -->
      <div class="llm-panel__header panel__header">
        <div>
          <p class="eyebrow">{{ t('llm.eyebrow') }}</p>
          <h2>{{ t('llm.panelTitle') }}</h2>
          <p class="panel__lede">
            {{ snapshot?.report_date || '' }} &middot; {{ scope }}
            <SkillBadge :name="skillName" size="sm" />
          </p>
        </div>
        <div class="llm-panel__actions">
          <span v-if="triggered" class="pill pill--positive">{{ t('llm.triggered') }}</span>
          <span v-else class="pill pill--neutral">{{ t('llm.notTriggered') }}</span>
          <span class="pill pill--outline">{{ selectedAgent }}</span>
          <button
            class="llm-panel__close"
            type="button"
            :aria-label="t('common.close')"
            @click="handleClose"
          >
            &times;
          </button>
        </div>
      </div>

      <!-- Placeholder warning -->
      <div v-if="isPlaceholder" class="llm-panel__placeholder-banner">
        <span class="llm-panel__placeholder-icon" aria-hidden="true">&#9888;</span>
        {{ t('llm.placeholderWarning') }}
      </div>

      <!-- Tabs -->
      <nav class="llm-panel__tabs" role="tablist">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="llm-panel__tab"
          :class="{ 'llm-panel__tab--active': activeTab === tab.key }"
          role="tab"
          :aria-selected="activeTab === tab.key"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
        </button>
      </nav>

      <!-- Loading -->
      <div v-if="loading" class="llm-panel__loading">
        <div class="llm-panel__spinner"></div>
        <p>{{ t('llm.analyzing') }}</p>
      </div>

      <!-- Error -->
      <div v-else-if="error" class="llm-panel__error">
        <p class="llm-panel__error-title">{{ t('common.error') }}</p>
        <p class="llm-panel__error-body">{{ error }}</p>
        <button class="button button--secondary" @click="handleReanalyze">
          {{ t('llm.reanalyze') }}
        </button>
      </div>

      <!-- Empty / Config -->
      <div v-else-if="!analysis" class="llm-panel__empty">
        <template v-if="!isConfigured">
          <p>{{ t('llm.notConfigured') }}</p>
          <div class="llm-panel__config-form">
            <input
              v-model="configForm.baseUrl"
              class="input-control"
              type="text"
              :placeholder="t('llm.baseUrl')"
            />
            <input
              v-model="configForm.model"
              class="input-control"
              type="text"
              :placeholder="t('llm.model')"
            />
            <input
              v-model="configForm.apiKey"
              class="input-control"
              type="password"
              :placeholder="t('llm.apiKey')"
            />
            <button class="button button--accent" @click="handleSaveConfig">
              {{ t('llm.saveConfig') }}
            </button>
          </div>
        </template>
        <template v-else>
          <p>{{ t('llm.noAnalysisYet') }}</p>
          <button class="button button--accent" @click="handleReanalyze">
            {{ t('llm.startAnalysis') }}
          </button>
        </template>
      </div>

      <!-- Content -->
      <div v-else class="llm-panel__content">
        <!-- Tab: Conclusion -->
        <div v-if="activeTab === 'conclusion'" class="llm-panel__tab-content">
          <div
            v-if="llmAnalysisText"
            class="llm-panel__markdown"
            v-html="renderMarkdown(llmAnalysisText)"
          ></div>
          <div v-else class="llm-panel__empty-section">
            {{ t('llm.noConclusion') }}
          </div>
        </div>

        <!-- Tab: Regime -->
        <div v-if="activeTab === 'regime'" class="llm-panel__tab-content">
          <div class="llm-panel__section">
            <h3>{{ t('llm.regimeState') }}</h3>
            <dl>
              <dt>{{ t('llm.currentState') }}</dt>
              <dd>{{ regimeAnalysis.current_state || 'N/A' }}</dd>
              <dt>{{ t('llm.transitionScore') }}</dt>
              <dd>{{ formatNumber(regimeAnalysis.transition, 2) }}</dd>
              <dt>{{ t('llm.confidence') }}</dt>
              <dd>{{ formatNumber(regimeAnalysis.confidence * 100, 1) }}%</dd>
            </dl>
          </div>
          <div v-if="keyDrivers.length" class="llm-panel__section">
            <h3>{{ t('llm.keyDrivers') }}</h3>
            <ul class="llm-panel__list">
              <li v-for="driver in keyDrivers" :key="driver" class="pill pill--outline">
                {{ driver }}
              </li>
            </ul>
          </div>
        </div>

        <!-- Tab: Execution -->
        <div v-if="activeTab === 'execution'" class="llm-panel__tab-content">
          <div class="llm-panel__section">
            <h3>{{ t('llm.executionDetails') }}</h3>
            <dl>
              <dt>{{ t('llm.skill') }}</dt>
              <dd>{{ skillName }}</dd>
              <dt>{{ t('llm.scope') }}</dt>
              <dd>{{ scope }}</dd>
              <dt>{{ t('llm.agent') }}</dt>
              <dd>{{ selectedAgent }}</dd>
            </dl>
          </div>
          <div v-if="tokenUsage" class="llm-panel__section">
            <h3>{{ t('llm.tokenUsage') }}</h3>
            <dl>
              <dt>{{ t('llm.systemTokens') }}</dt>
              <dd>{{ tokenUsage.system_tokens || 0 }}</dd>
              <dt>{{ t('llm.contextTokens') }}</dt>
              <dd>{{ tokenUsage.context_tokens || 0 }}</dd>
              <dt>{{ t('llm.reasoningTokens') }}</dt>
              <dd>{{ tokenUsage.reasoning_tokens || 0 }}</dd>
              <dt>{{ t('llm.outputTokens') }}</dt>
              <dd>{{ tokenUsage.output_tokens || 0 }}</dd>
            </dl>
          </div>
        </div>

        <!-- Tab: Risk -->
        <div v-if="activeTab === 'risk'" class="llm-panel__tab-content">
          <div class="llm-panel__section">
            <h3>{{ t('llm.riskAssessment') }}</h3>
            <dl>
              <dt>{{ t('llm.riskLevel') }}</dt>
              <dd>
                <span
                  class="pill"
                  :class="{
                    'pill--negative': riskAssessment.level === 'critical' || riskAssessment.level === 'high',
                    'pill--warning': riskAssessment.level === 'elevated',
                    'pill--positive': riskAssessment.level === 'low',
                  }"
                >
                  {{ riskAssessment.level || 'N/A' }}
                </span>
              </dd>
              <dt>{{ t('llm.recommendation') }}</dt>
              <dd>{{ riskAssessment.recommendation || 'N/A' }}</dd>
            </dl>
          </div>
          <div v-if="riskAssessment.factors?.length" class="llm-panel__section">
            <h3>{{ t('llm.riskFactors') }}</h3>
            <ul class="llm-panel__list">
              <li v-for="factor in riskAssessment.factors" :key="factor">
                {{ factor }}
              </li>
            </ul>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div v-if="analysis && !loading && !error" class="llm-panel__footer">
        <div class="llm-panel__footer-actions">
          <button class="button button--secondary" @click="handleReanalyze">
            {{ t('llm.reanalyze') }}
          </button>
          <button
            class="button button--secondary"
            :disabled="exporting"
            @click="handleExportMarkdown"
          >
            {{ exporting ? t('llm.exportingMarkdown') : t('llm.exportMarkdown') }}
          </button>
        </div>
        <span v-if="tokenUsage" class="llm-panel__token-summary">
          {{ t('llm.tokenSummary', {
            input: (tokenUsage.system_tokens || 0) + (tokenUsage.context_tokens || 0) + (tokenUsage.reasoning_tokens || 0),
            output: tokenUsage.output_tokens || 0,
          }) }}
        </span>
      </div>
    </article>
  </div>
</template>

<style scoped>
.llm-panel {
  position: fixed;
  inset: 0;
  z-index: var(--layer-modal, 40);
  display: flex;
  justify-content: flex-end;
}

.llm-panel__backdrop {
  position: absolute;
  inset: 0;
  background: var(--color-overlay);
  border: none;
  cursor: pointer;
}

.llm-panel__sheet {
  position: relative;
  width: 520px;
  max-width: 92vw;
  height: 100vh;
  overflow-y: auto;
  background: var(--panel-bg);
  border-left: 1px solid var(--panel-border);
  box-shadow: var(--shadow-strong);
  display: flex;
  flex-direction: column;
}

.llm-panel__header {
  flex-shrink: 0;
  padding: var(--space-5);
  border-bottom: 1px solid var(--panel-border);
}

.llm-panel__actions {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.llm-panel__close {
  background: none;
  border: none;
  font-size: 1.5rem;
  color: var(--text-secondary);
  cursor: pointer;
  padding: var(--space-1) var(--space-2);
  line-height: 1;
}

.llm-panel__close:hover {
  color: var(--text-primary);
}

.llm-panel__tabs {
  display: flex;
  gap: var(--space-1);
  padding: var(--space-3) var(--space-5);
  border-bottom: 1px solid var(--panel-border);
  overflow-x: auto;
  flex-shrink: 0;
}

.llm-panel__tab {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  padding: var(--space-2) var(--space-3);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  white-space: nowrap;
  transition: color var(--transition-base), border-color var(--transition-base);
}

.llm-panel__tab:hover {
  color: var(--text-primary);
}

.llm-panel__tab--active {
  color: var(--color-accent);
  border-bottom-color: var(--color-accent);
}

.llm-panel__content {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4) var(--space-5);
}

.llm-panel__tab-content {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.llm-panel__section {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--space-4);
}

.llm-panel__section h3 {
  font-size: var(--font-size-meta);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 var(--space-3);
}

.llm-panel__section dl {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--space-2) var(--space-3);
  margin: 0;
}

.llm-panel__section dt {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
}

.llm-panel__section dd {
  font-size: var(--font-size-meta);
  color: var(--text-primary);
  margin: 0;
  text-align: right;
}

.llm-panel__list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.llm-panel__list li {
  font-size: var(--font-size-meta);
  color: var(--text-primary);
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

.llm-panel__empty-section {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  text-align: center;
  padding: var(--space-6);
}

.llm-panel__placeholder-banner {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-5);
  background: var(--color-warning-soft);
  color: var(--color-warning);
  font-size: var(--font-size-meta);
  font-weight: 500;
  border-bottom: 1px solid var(--color-warning-soft);
}

.llm-panel__placeholder-icon {
  font-size: 1.1rem;
}

.llm-panel__config-form {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  width: 100%;
  max-width: 24rem;
}

.input-control {
  background: var(--panel-bg-secondary);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-sm);
  padding: var(--space-3) var(--space-4);
  color: var(--text-primary);
  font-size: var(--font-size-meta);
}

.input-control::placeholder {
  color: var(--text-secondary);
}

.llm-panel__footer-actions {
  display: flex;
  gap: var(--space-2);
}

.llm-panel__footer {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  border-top: 1px solid var(--panel-border);
  background: var(--panel-bg-secondary);
}

.llm-panel__token-summary {
  font-size: var(--font-size-label);
  color: var(--text-soft);
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

.pill--warning {
  background: var(--color-warning-soft);
  color: var(--color-warning);
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
}

h2 {
  margin: var(--space-1) 0 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-primary);
}

.button--accent {
  background: var(--color-accent-soft);
  color: var(--color-accent);
  border: 1px solid var(--color-accent-border);
}

.button--accent:hover:not(:disabled) {
  background: var(--color-accent);
  color: var(--color-bg);
}
</style>

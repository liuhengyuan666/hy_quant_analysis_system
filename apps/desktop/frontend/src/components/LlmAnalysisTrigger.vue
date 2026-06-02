<script setup>
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore, updateSelectedAgent, updateSelectedSkill, updateLlmConfig, updateAvailableAgents, updateAvailableSkills } from '../store.js';
import { llmApi } from '../api/tauri.js';

const { t } = useI18n();

const emit = defineEmits(['open-panel', 'analyze']);

const llmConfig = computed(() => dashboardStore.llmConfig);
const selectedAgent = computed({
  get: () => dashboardStore.selectedAgent,
  set: (value) => updateSelectedAgent(value),
});
const selectedSkill = computed({
  get: () => dashboardStore.selectedSkill,
  set: (value) => updateSelectedSkill(value),
});
const availableAgents = computed(() => dashboardStore.availableAgents);
const availableSkills = computed(() => dashboardStore.availableSkills);
const llmLoading = computed(() => dashboardStore.llmLoading);
const isConfigured = computed(() => llmConfig.value?.configured ?? false);

const AGENT_OPTIONS = computed(() =>
  availableAgents.value.length > 0
    ? availableAgents.value.map((a) => ({
        value: a.name,
        label: t(`llm.agents.${a.name}`, a.name),
      }))
    : [
        { value: 'macro-strategist', label: t('llm.agents.macroStrategist') },
        { value: 'risk-manager', label: t('llm.agents.riskManager') },
        { value: 'technical-analyst', label: t('llm.agents.technicalAnalyst') },
      ]
);

const SKILL_OPTIONS = computed(() =>
  availableSkills.value.length > 0
    ? availableSkills.value.map((s) => ({
        value: s.name,
        label: s.name,
      }))
    : [{ value: 'market-regime-reasoning', label: 'market-regime-reasoning' }]
);

async function loadLlmStatus() {
  try {
    const status = await llmApi.getStatus();
    updateLlmConfig(status);
  } catch (error) {
    console.error('[LlmTrigger] Failed to load LLM status:', error);
    updateLlmConfig({ configured: false });
  }
}

async function loadAgentProfiles() {
  try {
    const agents = await llmApi.listAgentProfiles();
    updateAvailableAgents(agents);
  } catch (error) {
    console.error('[LlmTrigger] Failed to load agent profiles:', error);
  }
}

async function loadSkills() {
  try {
    const skills = await llmApi.listSkills();
    updateAvailableSkills(skills);
  } catch (error) {
    console.error('[LlmTrigger] Failed to load skills:', error);
  }
}

function handleAnalyze() {
  if (!isConfigured.value) {
    emit('open-panel');
    return;
  }
  emit('analyze');
}

function handleOpenPanel() {
  emit('open-panel');
}

onMounted(() => {
  loadLlmStatus();
  loadAgentProfiles();
  loadSkills();
});
</script>

<template>
  <div class="llm-trigger">
    <div class="llm-trigger__row">
      <select
        class="select-control select-control--compact"
        :value="selectedSkill"
        :disabled="llmLoading"
        @change="selectedSkill = $event.target.value"
      >
        <option v-for="opt in SKILL_OPTIONS" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
      <select
        class="select-control select-control--compact"
        :value="selectedAgent"
        :disabled="llmLoading"
        @change="selectedAgent = $event.target.value"
      >
        <option v-for="opt in AGENT_OPTIONS" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
      <button
        class="button button--accent"
        :disabled="llmLoading"
        :class="{ 'button--disabled': !isConfigured }"
        @click="handleAnalyze"
      >
        <span v-if="llmLoading" class="llm-trigger__spinner" aria-hidden="true"></span>
        <span v-else class="llm-trigger__icon" aria-hidden="true">&#x2728;</span>
        {{ llmLoading ? t('llm.analyzing') : t('llm.analyze') }}
      </button>
      <button
        v-if="!isConfigured"
        class="button button--ghost"
        :title="t('llm.notConfigured')"
        @click="handleOpenPanel"
      >
        ?
      </button>
    </div>
    <span v-if="!isConfigured" class="llm-trigger__hint">
      {{ t('llm.configureHint') }}
    </span>
    <span v-else-if="llmConfig?.model" class="llm-trigger__meta">
      {{ llmConfig.model }}
    </span>
  </div>
</template>

<style scoped>
.llm-trigger {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.llm-trigger__row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.llm-trigger__icon {
  margin-right: var(--space-1);
}

.llm-trigger__spinner {
  display: inline-block;
  width: 0.875rem;
  height: 0.875rem;
  margin-right: var(--space-1);
  border: 2px solid var(--color-accent);
  border-top-color: transparent;
  border-radius: 50%;
  animation: llm-spin 0.8s linear infinite;
}

@keyframes llm-spin {
  to {
    transform: rotate(360deg);
  }
}

.llm-trigger__hint {
  font-size: var(--font-size-label);
  color: var(--color-warning);
}

.llm-trigger__meta {
  font-size: var(--font-size-label);
  color: var(--color-text-soft);
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

.button--ghost {
  background: transparent;
  color: var(--color-text-muted);
  border: 1px solid var(--color-border);
  padding: var(--space-2) var(--space-3);
  min-width: auto;
}

.button--ghost:hover {
  color: var(--color-text);
  border-color: var(--color-border-strong);
}

.button--disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>

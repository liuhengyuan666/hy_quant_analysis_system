<script setup>
import { ref, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore, updateAvailableAgents, toggleAgentEditor } from '../store.js';
import { llmApi } from '../api/tauri.js';

const { t } = useI18n();

const emit = defineEmits(['close', 'saved']);

const selectedAgentName = ref('');
const content = ref('');
const saving = ref(false);
const message = ref('');
const messageType = ref('');

const availableAgents = computed(() => dashboardStore.availableAgents);

async function loadAgentContent(name) {
  if (!name) {
    content.value = '';
    return;
  }
  try {
    const yaml = await llmApi.readAgentProfile(name);
    content.value = yaml;
  } catch (error) {
    console.error('[AgentEditor] Failed to load agent:', error);
    // Fallback: generate a minimal template from metadata
    const agent = availableAgents.value.find((a) => a.name === name);
    if (agent) {
      content.value = `# ${agent.name}\n# ${agent.description || ''}\nname: ${agent.name}\ndescription: ${agent.description || ''}\n`;
    }
  }
}

watch(selectedAgentName, (name) => {
  loadAgentContent(name);
});

async function handleSave() {
  if (!selectedAgentName.value || !content.value.trim()) {
    message.value = t('llm.editor.missingFields');
    messageType.value = 'error';
    return;
  }
  saving.value = true;
  message.value = '';
  try {
    await llmApi.saveAgentProfile(selectedAgentName.value, content.value);
    message.value = t('llm.editor.saved');
    messageType.value = 'success';
    // Refresh agent list
    const agents = await llmApi.listAgentProfiles();
    updateAvailableAgents(agents);
    emit('saved');
  } catch (error) {
    message.value = t('llm.editor.saveFailed');
    messageType.value = 'error';
    console.error('[AgentEditor] Save failed:', error);
  } finally {
    saving.value = false;
  }
}

function handleClose() {
  emit('close');
  toggleAgentEditor(false);
}
</script>

<template>
  <div class="agent-editor-overlay" @click.self="handleClose">
    <div class="agent-editor">
      <div class="agent-editor__header">
        <h3 class="agent-editor__title">{{ t('llm.editor.title') }}</h3>
        <button class="agent-editor__close" @click="handleClose">&times;</button>
      </div>

      <div class="agent-editor__body">
        <label class="agent-editor__label">{{ t('llm.editor.selectAgent') }}</label>
        <select v-model="selectedAgentName" class="select-control">
          <option value="">{{ t('llm.editor.newAgent') }}</option>
          <option v-for="agent in availableAgents" :key="agent.name" :value="agent.name">
            {{ agent.name }}
          </option>
        </select>

        <label class="agent-editor__label">{{ t('llm.editor.yamlContent') }}</label>
        <textarea
          v-model="content"
          class="agent-editor__textarea"
          rows="18"
          :placeholder="t('llm.editor.yamlPlaceholder')"
        ></textarea>

        <div v-if="message" class="agent-editor__message" :class="`agent-editor__message--${messageType}`">
          {{ message }}
        </div>
      </div>

      <div class="agent-editor__footer">
        <button class="button button--secondary" @click="handleClose">
          {{ t('common.cancel') }}
        </button>
        <button class="button button--accent" :disabled="saving" @click="handleSave">
          {{ saving ? t('llm.editor.saving') : t('llm.editor.save') }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.agent-editor-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.agent-editor {
  background: var(--color-panel);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-lg);
  width: min(90vw, 720px);
  max-height: 90vh;
  display: flex;
  flex-direction: column;
}

.agent-editor__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--panel-border);
}

.agent-editor__title {
  font-size: var(--font-size-body);
  font-weight: 600;
  margin: 0;
}

.agent-editor__close {
  background: transparent;
  border: none;
  color: var(--color-text-muted);
  font-size: 1.5rem;
  cursor: pointer;
  line-height: 1;
}

.agent-editor__close:hover {
  color: var(--color-text);
}

.agent-editor__body {
  padding: var(--space-4) var(--space-5);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  overflow-y: auto;
}

.agent-editor__label {
  font-size: var(--font-size-label);
  color: var(--color-text-soft);
  font-weight: 500;
}

.agent-editor__textarea {
  width: 100%;
  background: var(--color-bg);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  color: var(--color-text);
  font-family: var(--font-mono, monospace);
  font-size: 0.8125rem;
  line-height: 1.6;
  padding: var(--space-3);
  resize: vertical;
}

.agent-editor__textarea:focus {
  outline: none;
  border-color: var(--color-accent);
}

.agent-editor__message {
  font-size: var(--font-size-label);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
}

.agent-editor__message--success {
  background: rgba(34, 197, 94, 0.15);
  color: #22c55e;
}

.agent-editor__message--error {
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
}

.agent-editor__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-5);
  border-top: 1px solid var(--panel-border);
}
</style>

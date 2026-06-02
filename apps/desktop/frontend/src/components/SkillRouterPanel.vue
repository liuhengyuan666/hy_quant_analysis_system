<script setup>
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { dashboardStore } from '../store.js';
import { llmApi } from '../api/tauri.js';
import SkillBadge from './SkillBadge.vue';

const { t } = useI18n();

const triggers = ref([]);
const loading = ref(false);
const error = ref('');

const triggeredSkills = computed(() => triggers.value.filter((s) => s.triggered));
const inactiveSkills = computed(() => triggers.value.filter((s) => !s.triggered));

async function loadTriggers() {
  loading.value = true;
  error.value = '';
  try {
    triggers.value = await llmApi.evaluateSkillTriggers(dashboardStore.selectedScope);
  } catch (err) {
    console.error('[SkillRouter] Failed to evaluate triggers:', err);
    error.value = err?.toString?.() || t('llm.triggerEvalFailed');
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  loadTriggers();
});
</script>

<template>
  <div class="skill-router">
    <div v-if="loading" class="skill-router__loading">
      {{ t('llm.evaluatingTriggers') }}
    </div>
    <div v-else-if="error" class="skill-router__error">
      {{ error }}
    </div>
    <div v-else class="skill-router__content">
      <div v-if="triggeredSkills.length" class="skill-router__section">
        <h3 class="skill-router__heading">
          {{ t('llm.triggeredSkills') }} ({{ triggeredSkills.length }})
        </h3>
        <div class="skill-router__list">
          <div
            v-for="skill in triggeredSkills"
            :key="skill.name"
            class="skill-router__item skill-router__item--triggered"
          >
            <div class="skill-router__item-header">
              <SkillBadge :name="skill.name" size="sm" />
              <span class="skill-router__weight">{{ skill.weight.toFixed(2) }}</span>
            </div>
            <p class="skill-router__description">{{ skill.description }}</p>
          </div>
        </div>
      </div>

      <div v-if="inactiveSkills.length" class="skill-router__section">
        <h3 class="skill-router__heading">
          {{ t('llm.inactiveSkills') }} ({{ inactiveSkills.length }})
        </h3>
        <div class="skill-router__list">
          <div
            v-for="skill in inactiveSkills"
            :key="skill.name"
            class="skill-router__item"
          >
            <div class="skill-router__item-header">
              <span class="skill-router__name">{{ skill.name }}</span>
              <span class="skill-router__weight">{{ skill.weight.toFixed(2) }}</span>
            </div>
            <p class="skill-router__description">{{ skill.description }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.skill-router {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.skill-router__loading,
.skill-router__error {
  padding: var(--space-4);
  text-align: center;
  color: var(--text-secondary);
}

.skill-router__error {
  color: var(--color-error);
}

.skill-router__section {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.skill-router__heading {
  font-size: var(--font-size-label);
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}

.skill-router__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.skill-router__item {
  padding: var(--space-3) var(--space-4);
  border: 1px solid var(--panel-border);
  border-radius: var(--radius-sm);
  background: var(--panel-bg-secondary);
}

.skill-router__item--triggered {
  border-color: var(--color-accent);
  background: var(--color-accent-soft);
}

.skill-router__item-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--space-2);
}

.skill-router__name {
  font-size: var(--font-size-meta);
  font-weight: 600;
  color: var(--text-primary);
}

.skill-router__weight {
  font-size: var(--font-size-label);
  color: var(--text-soft);
  font-family: var(--font-mono);
}

.skill-router__description {
  font-size: var(--font-size-meta);
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.4;
}
</style>

<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import {
  formatInteger,
  getErrorMessage,
  normalizeUsageGuides,
  renderMarkdownContent,
  resolveSelectedUsageGuide,
} from '../lib/dashboard-utils.js';

const { t } = useI18n();

const COMMANDS = {
  usageGuides: 'usage_guides',
};

// Local reactive state
const usageGuides = ref([]);
const usageGuidesLoading = ref(false);
const usageGuidesLoaded = ref(false);
const usageGuidesError = ref('');
const isUsageGuideOpen = ref(false);
const selectedUsageGuideId = ref('');

const guideCount = computed(() => usageGuides.value.length);

const selectedGuide = computed(() => {
  return usageGuides.value.find((g) => g.id === selectedUsageGuideId.value)
    || usageGuides.value[0] || null;
});

const selectedGuideContent = computed(() => {
  if (!selectedGuide.value) return '';
  return renderMarkdownContent(selectedGuide.value.content);
});

// Body scroll lock
watch(isUsageGuideOpen, (open) => {
  document.body.classList.toggle('body--guide-viewer-open', open);
});

// Keyboard: ESC to close
function handleKeydown(event) {
  if (event.key === 'Escape' && isUsageGuideOpen.value) {
    closeUsageGuides();
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener('keydown', handleKeydown);
  document.body.classList.remove('body--guide-viewer-open');
});

async function loadUsageGuides() {
  if (usageGuidesLoading.value) return;

  usageGuidesLoading.value = true;
  usageGuidesError.value = '';

  try {
    usageGuides.value = normalizeUsageGuides(await invoke(COMMANDS.usageGuides));
    usageGuidesLoaded.value = true;
    selectedUsageGuideId.value = resolveSelectedUsageGuide(usageGuides.value, selectedUsageGuideId.value);
  } catch (error) {
    usageGuides.value = [];
    usageGuidesLoaded.value = false;
    selectedUsageGuideId.value = '';
    usageGuidesError.value = getErrorMessage(error);
  } finally {
    usageGuidesLoading.value = false;
  }
}

function openUsageGuides() {
  isUsageGuideOpen.value = true;

  if (!usageGuidesLoaded.value && !usageGuidesLoading.value) {
    loadUsageGuides();
  }
}

function closeUsageGuides() {
  if (!isUsageGuideOpen.value) return;
  isUsageGuideOpen.value = false;
}

function selectGuide(guideId) {
  if (!guideId || guideId === selectedUsageGuideId.value) return;
  selectedUsageGuideId.value = guideId;
}

defineExpose({
  openUsageGuides,
  closeUsageGuides,
  isOpen: isUsageGuideOpen,
});
</script>

<template>
  <section
    class="guide-viewer"
    :class="{ 'guide-viewer--open': isUsageGuideOpen }"
    :aria-hidden="isUsageGuideOpen ? 'false' : 'true'"
  >
    <button
      class="guide-viewer__backdrop"
      type="button"
      :aria-label="t('common.close')"
      @click="closeUsageGuides"
    ></button>

    <div class="guide-viewer__panel" role="dialog" aria-modal="true" aria-labelledby="usageGuideViewerTitle">
      <div class="guide-viewer__header">
        <div>
          <p class="eyebrow">{{ t('usageGuides.helpUsage') }}</p>
          <h2 id="usageGuideViewerTitle">{{ t('usageGuides.guideLibrary') }}</h2>
          <p class="panel__lede">{{ t('usageGuides.lede') }}</p>
        </div>
        <div class="panel__actions">
          <span class="pill pill--outline">{{ t('usageGuides.available', { count: formatInteger(guideCount) }) }}</span>
          <button
            class="button button--secondary button--compact"
            :disabled="usageGuidesLoading"
            @click="loadUsageGuides"
          >
            {{ usageGuidesLoading ? t('usageGuides.refreshing') : t('usageGuides.reload') }}
          </button>
          <button class="button button--secondary button--compact" @click="closeUsageGuides">
            {{ t('usageGuides.close') }}
          </button>
        </div>
      </div>

      <div class="guide-viewer__body">
        <aside class="guide-viewer__sidebar" :aria-label="t('usageGuides.guideLibrary')">
          <div v-if="guideCount" class="guide-tab-list" role="tablist" aria-label="Usage guides">
            <button
              v-for="guide in usageGuides"
              :key="guide.id"
              class="guide-tab"
              :class="{ 'guide-tab--active': guide.id === selectedUsageGuideId }"
              type="button"
              role="tab"
              :aria-selected="guide.id === selectedUsageGuideId ? 'true' : 'false'"
              @click="selectGuide(guide.id)"
            >
              <span class="guide-tab__eyebrow">{{ t('usageGuides.manual') }}</span>
              <strong class="guide-tab__title">{{ guide.title }}</strong>
            </button>
          </div>
          <div v-else class="empty-state empty-state--compact guide-viewer__empty-nav">
            <p>{{ usageGuidesLoading ? t('usageGuides.preparing') : t('usageGuides.noGuides') }}</p>
          </div>
        </aside>

        <div class="guide-viewer__content">
          <!-- Loading state (when no guides loaded yet) -->
          <div v-if="usageGuidesLoading && !guideCount" class="empty-state">
            <p>{{ t('usageGuides.loadingInApp') }}</p>
          </div>

          <!-- Error state -->
          <section v-else-if="usageGuidesError" class="notice notice--error notice--inline">
            <div>
              <strong>{{ t('usageGuides.libraryUnavailable') }}</strong>
              <p>{{ usageGuidesError }}</p>
            </div>
          </section>

          <!-- No selection -->
          <div v-else-if="!selectedGuide" class="empty-state">
            <p>{{ t('usageGuides.noGuidesReturned') }}</p>
          </div>

          <!-- Content -->
          <article v-else class="guide-article">
            <header class="guide-article__header">
              <p class="eyebrow">{{ t('usageGuides.manual') }}</p>
              <h3>{{ selectedGuide.title }}</h3>
            </header>
            <div class="guide-article__content" v-html="selectedGuideContent"></div>
          </article>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.guide-viewer {
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
  z-index: 30;
}

.guide-viewer--open {
  opacity: 1;
  visibility: visible;
  pointer-events: auto;
}

.guide-viewer__backdrop {
  position: absolute;
  inset: 0;
  border: 0;
  padding: 0;
  background: linear-gradient(180deg, rgba(7, 10, 12, 0.56), var(--color-overlay));
  cursor: pointer;
}

.guide-viewer__panel {
  position: relative;
  width: min(100%, 76rem);
  max-height: calc(100vh - (var(--space-7) * 2));
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  border: 1px solid var(--color-border-strong);
  border-radius: calc(1.5rem + 0.25rem);
  background:
    linear-gradient(135deg, rgba(212, 168, 93, 0.1), transparent 34%),
    linear-gradient(180deg, rgba(18, 22, 27, 0.98), rgba(14, 17, 21, 0.96));
  box-shadow: 0 24px 56px rgba(0, 0, 0, 0.34);
  overflow: hidden;
}

.guide-viewer__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-4);
  padding: var(--space-5);
  border-bottom: 1px solid var(--color-border);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.03), rgba(255, 255, 255, 0));
}

.guide-viewer__body {
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(0, 17rem) minmax(0, 1fr);
}

.guide-viewer__sidebar {
  min-height: 0;
  padding: var(--space-4);
  border-right: 1px solid var(--color-border);
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.02), rgba(255, 255, 255, 0));
}

.guide-tab-list {
  display: grid;
  gap: var(--space-3);
}

.guide-tab {
  display: grid;
  gap: var(--space-1);
  width: 100%;
  padding: var(--space-3) var(--space-4);
  border: 1px solid var(--color-border);
  border-radius: 1.125rem;
  background: linear-gradient(180deg, var(--color-surface-soft), rgba(19, 23, 28, 0.88));
  color: var(--color-text);
  text-align: left;
  cursor: pointer;
  transition:
    transform 180ms ease,
    border-color 180ms ease,
    background-color 180ms ease;
  font: inherit;
}

.guide-tab:hover,
.guide-tab:focus-visible {
  transform: translateY(-1px);
  border-color: var(--color-border-strong);
}

.guide-tab--active {
  border-color: rgba(212, 168, 93, 0.28);
  background: linear-gradient(180deg, rgba(212, 168, 93, 0.18), rgba(19, 23, 28, 0.92));
}

.guide-tab__eyebrow {
  color: var(--color-secondary);
  font-size: 0.8rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.guide-tab__title {
  font-size: 0.95rem;
  line-height: 1.4;
}

.guide-viewer__content {
  min-height: 0;
  overflow-y: auto;
  padding: var(--space-5);
}

.guide-viewer__empty-nav {
  min-height: 100%;
}

.guide-article {
  display: grid;
  gap: var(--space-5);
}

.guide-article__header {
  display: grid;
  gap: var(--space-2);
}

.guide-article__header h3 {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 600;
  letter-spacing: -0.03em;
  color: var(--color-text);
  font-size: clamp(1.8rem, 2.6vw, 2.8rem);
  line-height: 1;
}

.guide-article__content {
  display: grid;
  gap: var(--space-4);
  color: var(--color-text-muted);
  font-size: 1rem;
  line-height: 1.65;
}

.guide-article__content :deep(h1) {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 600;
  letter-spacing: -0.03em;
  color: var(--color-text);
  font-size: clamp(1.8rem, 2.6vw, 2.8rem);
  line-height: 1;
}

.guide-article__content :deep(h2) {
  padding-top: var(--space-2);
  margin: 0;
  font-family: var(--font-display);
  font-weight: 600;
  letter-spacing: -0.03em;
  color: var(--color-text);
  font-size: clamp(1.25rem, 1.8vw, 1.7rem);
}

.guide-article__content :deep(h3) {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 600;
  letter-spacing: -0.03em;
  color: var(--color-text);
  font-size: 1.15rem;
}

.guide-article__content :deep(h4) {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 600;
  letter-spacing: 0.08em;
  color: var(--color-text);
  font-size: 0.95rem;
  text-transform: uppercase;
}

.guide-article__content :deep(p),
.guide-article__content :deep(ul),
.guide-article__content :deep(ol),
.guide-article__content :deep(blockquote),
.guide-article__content :deep(pre) {
  margin: 0;
}

.guide-article__content :deep(ul),
.guide-article__content :deep(ol) {
  display: grid;
  gap: var(--space-2);
  padding-left: var(--space-5);
}

.guide-article__content :deep(li) {
  overflow-wrap: anywhere;
}

.guide-article__content :deep(strong) {
  color: var(--color-text);
}

.guide-article__content :deep(a) {
  color: var(--color-secondary);
}

.guide-article__content :deep(code) {
  display: inline-block;
  max-width: 100%;
  padding: var(--space-1) var(--space-2);
  border-radius: 0.75rem;
  background: var(--color-surface-raised);
  overflow-wrap: anywhere;
  font-family: monospace;
  font-size: 0.92em;
}

.guide-article__content :deep(pre) {
  padding: var(--space-4);
  border: 1px solid var(--color-border);
  border-radius: 1.125rem;
  background: var(--color-surface-strong);
  overflow: auto;
}

.guide-article__content :deep(pre code) {
  display: block;
  padding: 0;
  background: transparent;
}

.guide-article__content :deep(blockquote) {
  padding: var(--space-3) var(--space-4);
  border-left: var(--space-1) solid var(--color-accent);
  border-radius: 0 1.125rem 1.125rem 0;
  background: linear-gradient(90deg, rgba(212, 168, 93, 0.18), rgba(255, 255, 255, 0));
}

.guide-article__content :deep(hr) {
  width: 100%;
  height: 1px;
  border: 0;
  background: var(--color-border);
}

.panel__lede {
  margin: 0;
  color: var(--color-text-muted);
  line-height: 1.65;
}

.panel__actions {
  display: flex;
  justify-content: space-between;
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

.empty-state--compact {
  padding: var(--space-4);
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
  margin: var(--space-4) 0 0;
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
  .guide-viewer {
    padding: var(--space-4);
  }

  .guide-viewer__body {
    grid-template-columns: minmax(0, 1fr);
  }

  .guide-viewer__sidebar {
    border-right: 0;
    border-bottom: 1px solid var(--color-border);
  }

  .guide-tab-list {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 720px) {
  .guide-viewer {
    padding: var(--space-3);
  }

  .guide-viewer__panel {
    max-height: calc(100vh - (var(--space-3) * 2));
  }

  .guide-viewer__header,
  .guide-viewer__content {
    padding: var(--space-4);
  }

  .guide-viewer__sidebar {
    padding: var(--space-3);
  }

  .guide-tab-list {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>

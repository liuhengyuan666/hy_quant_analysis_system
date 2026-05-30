<script setup>
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { prettifyToken } from '../lib/dashboard-utils.js';
import { dashboardStore } from '../store.js';

const { t } = useI18n();
const status = computed(() => dashboardStore.status);
</script>

<template>
  <article class="panel panel--soft">
    <div class="panel__header">
      <div>
        <p class="eyebrow">{{ t('status.eyebrow') }}</p>
        <h2>{{ t('status.title') }}</h2>
        <p v-if="status" class="panel__lede">
          {{ t('status.lede') }}
        </p>
      </div>
      <span v-if="status" class="pill pill--outline">{{ prettifyToken(status.profile) }}</span>
    </div>

    <div v-if="!status" class="empty-state">
      <p>{{ t('status.unavailable') }}</p>
    </div>

    <dl v-else class="detail-grid">
      <div class="detail-item">
        <dt>{{ t('status.profile') }}</dt>
        <dd>{{ status.profile }}</dd>
      </div>
      <div class="detail-item">
        <dt>{{ t('status.database') }}</dt>
        <dd>{{ status.clickhouse_database }}</dd>
      </div>
      <div class="detail-item detail-item--full">
        <dt>{{ t('status.clickhouseUrl') }}</dt>
        <dd><code>{{ status.clickhouse_url }}</code></dd>
      </div>
      <div class="detail-item detail-item--full">
        <dt>{{ t('status.sqlitePath') }}</dt>
        <dd><code>{{ status.sqlite_path }}</code></dd>
      </div>
      <div class="detail-item detail-item--full">
        <dt>{{ t('status.universePath') }}</dt>
        <dd><code>{{ status.universe_path }}</code></dd>
      </div>
    </dl>
  </article>
</template>

<style scoped>
.panel {
  background: var(--panel-bg);
  border: 1px solid var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--panel-padding);
}

.panel--soft {
  background: var(--panel-bg-secondary);
}

.panel__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: var(--space-4);
}

.panel__lede {
  color: var(--text-secondary);
  font-size: var(--font-size-meta);
  margin-top: var(--space-1);
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-3);
}

.detail-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.detail-item--full {
  grid-column: 1 / -1;
}

.detail-item dt {
  font-size: var(--font-size-label);
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.detail-item dd {
  font-size: var(--font-size-meta);
  color: var(--text-primary);
  margin: 0;
}

.detail-item code {
  font-family: monospace;
  font-size: var(--font-size-label);
  padding: var(--space-1) var(--space-2);
  background: var(--panel-bg);
  border-radius: var(--space-1);
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
</style>
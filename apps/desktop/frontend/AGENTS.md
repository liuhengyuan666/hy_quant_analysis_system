# FRONTEND KNOWLEDGE BASE

## OVERVIEW
Plain-JS dashboard frontend with Vue 3 progressive migration. Single root render tree in main.js, Vue components mounting to #vue-app, shared reactive state via src/store.js (10 properties synced).

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Root state + commands | `src/main.js` top section | `state`, `COMMANDS`, load/render orchestration |
| Shared reactive state | `src/store.js` | Vue reactive store with 10 synced properties |
| Vue entry point | `src/main-vue.js` | Vue app creation and mounting |
| Vue root component | `src/App.vue` | Reads from shared store, renders Vue panels, wires events |
| Shared helpers | `src/lib/dashboard-utils.js` | formatting, normalization, scope/report helpers |
| Trust entry panel | `src/main.js::renderTrustSummaryPanel` | primary trust verdict with freshness + health evidence summaries |
| Refresh UI | `src/main.js::renderRefreshProgress` + `startRefreshJob` | full refresh CTA, stage selector, retry flow |
| Recent reports panel | `src/components/RecentReportsPanel.vue` | Vue component, reads from store |
| Data health panel    | `src/components/DataHealthPanel.vue`    | Vue component, reads from store |
| Usage guides panel   | `src/components/UsageGuidesPanel.vue`   | Vue component, reads from store |
| Environment panel    | `src/components/EnvironmentPanel.vue`   | Vue component, reads from store |
| Breadth panel        | `src/components/BreadthPanel.vue`       | Vue component, reads from store |
| i18n infrastructure  | `src/i18n.js`                           | vue-i18n@11 config, setLocale/getLocale |
| Language toggle      | `src/components/LanguageToggle.vue`     | Top-right corner toggle |
| Locale files         | `src/locales/zh.json`, `src/locales/en.json` | ~280 translation keys each |
| Vue components | `src/components/*.vue` | 20+ Vue panels (BreadthPanel, MetricCard, DateSelector, LanguageToggle, etc.) |
| Visual system | `src/styles.css` | panel/grid/pill/refresh/report-history styles + Vue CSS variable bridge |

## CONVENTIONS
- Plain JS and Vue coexist during progressive migration.
- `main.js` owns root state, command names, and render scheduling; feature slices own local actions/renderers once extracted.
- `src/store.js` provides shared reactive state with 10 properties: snapshot, status, selectedScope, selectedReportDate, availableDates, loading, error, exporting, exportResult, refreshStatus.
- main.js calls sync*ToStore() functions at all state mutation points.
- Vue components read from store via computed(), never load data independently.
- Startup and scope changes should use `dashboard_bundle`; historical date changes should stay on `dashboard_snapshot`.
- `render()` schedules and `commitRender()` mutates the DOM.
- `renderTrustSummaryPanel()` is the primary "can I trust this snapshot?" entry point; the inline trust notice is secondary post-refresh context.
- `Recent reports` supports `Open snapshot` for `DAILY_REPORT*`, plus `Open artifact` and `Copy path` for all artifacts.
- Default refresh is full refresh. Stage rerun options are advanced recovery controls and must stay aligned with backend order: `ingest -> indicators -> macro -> rotation -> strategy -> signals -> backtests`.
- Environment layer and watchlist breadth intentionally coexist: explanation layer + raw proxy breakdown.
- Vue components should import utility functions from `dashboard-utils.js`, not re-implement them.
- Vue templates use `{{ }}` interpolation which auto-escapes; do not wrap with `escapeHtml()`.
- Vue components consume CSS variables via the bridge defined in styles.css `:root`.
- Signal reason is an object; render `item.reason?.summary` not `item.reason`.

## ANTI-PATTERNS
- Do **not** add business logic or provider semantics in frontend.
- Do **not** bypass scheduled rendering with scattered direct DOM writes.
- Do **not** reintroduce root-level startup fan-out when `dashboard_bundle` already supplies the bootstrap payload.
- Do **not** let snapshot/export/recent-report actions ignore `selectedScope`.
- Do **not** push feature-specific logic back into `main.js` once a dedicated slice exists.
- Do **not** label Environment Layer as true stock-market breadth.
- Do **not** load data independently in Vue components; read from shared store instead.
- Do **not** re-implement utility functions in Vue components; import from `dashboard-utils.js`.
- Do **not** use `escapeHtml()` in Vue template bindings; Vue handles escaping automatically.
- Do **not** use CSS variable names that don't match the bridge in styles.css.
- Do **not** render `{{ item.reason }}` directly; use `{{ item.reason?.summary }}` since reason is an object.
- Do **not** forget to call sync*ToStore() when updating state in main.js.
- Do **not** hardcode UI strings in Vue components; use `t('key')` from `useI18n()`.
- Do **not** add new locale keys without updating both `zh.json` and `en.json`.

## NOTES
- `main.js` retains root state and global orchestration; all panel rendering migrated to Vue components.
- Phase 3 complete: all panels migrated to Vue components. i18n implemented with vue-i18n@11 (zh/en).
- Phase 0 deleted ~700 lines dead code (`features/*.js`, `renderers/*.js`).
- Store has 10 synced properties covering all Vue component dependencies.
- App.vue wires all component events (DateSelector, SignalsPanel, RefreshProgress, SignalDetailModal, LanguageToggle).
- `node_modules/` and `dist/` are generated artifacts.

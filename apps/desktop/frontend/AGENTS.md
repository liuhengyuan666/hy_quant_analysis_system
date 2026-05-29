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
| Recent-reports slice | `src/features/recent-reports.js` | snapshot jump, artifact open, copy path |
| Data-health slice | `src/features/data-health.js` | cache, load, export, render |
| Usage-guides slice | `src/features/usage-guides.js` | guide loading and viewer state |
| Environment/breadth renderers | `src/renderers/environment-breadth.js` | paired environment explanation + breadth proxy panels |
| Vue components | `src/components/*.vue` | 15 Vue panels (BreadthPanel, MetricCard, DateSelector, etc.) |
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

## NOTES
- `main.js` is still large, but pure helper logic and several interaction areas have already moved out into dedicated slices.
- Phase 2 complete: all 14 panels migrated to Vue components.
- Store has 10 synced properties covering all Vue component dependencies.
- App.vue wires all component events (DateSelector, SignalsPanel, RefreshProgress, SignalDetailModal).
- `node_modules/` and `dist/` are generated artifacts.

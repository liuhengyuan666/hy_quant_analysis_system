# FRONTEND KNOWLEDGE BASE

## OVERVIEW
Plain-JS dashboard frontend. Single root render tree, Tauri invoke bridge, no active Vue runtime.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Root state + commands | `src/main.js` top section | `state`, `COMMANDS`, load/render orchestration |
| Shared helpers | `src/lib/dashboard-utils.js` | formatting, normalization, scope/report helpers |
| Trust entry panel | `src/main.js::renderTrustSummaryPanel` | primary trust verdict with freshness + health evidence summaries |
| Refresh UI | `src/main.js::renderRefreshProgress` + `startRefreshJob` | full refresh CTA, stage selector, retry flow |
| Recent-reports slice | `src/features/recent-reports.js` | snapshot jump, artifact open, copy path |
| Data-health slice | `src/features/data-health.js` | cache, load, export, render |
| Usage-guides slice | `src/features/usage-guides.js` | guide loading and viewer state |
| Environment/breadth renderers | `src/renderers/environment-breadth.js` | paired environment explanation + breadth proxy panels |
| Visual system | `src/styles.css` | panel/grid/pill/refresh/report-history styles |

## CONVENTIONS
- Plain JS is the real UI runtime; do not plan around a hidden Vue layer.
- `main.js` owns root state, command names, and render scheduling; feature slices own local actions/renderers once extracted.
- Startup and scope changes should use `dashboard_bundle`; historical date changes should stay on `dashboard_snapshot`.
- `render()` schedules and `commitRender()` mutates the DOM.
- `renderTrustSummaryPanel()` is the primary “can I trust this snapshot?” entry point; the inline trust notice is secondary post-refresh context.
- `Recent reports` supports `Open snapshot` for `DAILY_REPORT*`, plus `Open artifact` and `Copy path` for all artifacts.
- Default refresh is full refresh. Stage rerun options are advanced recovery controls and must stay aligned with backend order: `ingest -> indicators -> macro -> rotation -> strategy -> signals -> backtests`.
- Environment layer and watchlist breadth intentionally coexist: explanation layer + raw proxy breakdown.

## ANTI-PATTERNS
- Do **not** add business logic or provider semantics in frontend.
- Do **not** bypass scheduled rendering with scattered direct DOM writes.
- Do **not** reintroduce root-level startup fan-out when `dashboard_bundle` already supplies the bootstrap payload.
- Do **not** let snapshot/export/recent-report actions ignore `selectedScope`.
- Do **not** push feature-specific logic back into `main.js` once a dedicated slice exists.
- Do **not** label Environment Layer as true stock-market breadth.

## NOTES
- `main.js` is still large, but pure helper logic and several interaction areas have already moved out into dedicated slices.
- Recent report labels still encode scope (`DAILY_REPORT_CN`, `DAILY_REPORT_HK`); keep UI scope inference aligned with backend report types until metadata grows a first-class scope field.
- The next likely seam for result-management work is still `src/features/recent-reports.js` plus the snapshot-loading path in `main.js`.
- `node_modules/` and `dist/` are generated artifacts.

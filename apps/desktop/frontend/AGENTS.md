# FRONTEND KNOWLEDGE BASE

## OVERVIEW
Plain-JS dashboard frontend. Single root render, Tauri invoke bridge, no active Vue runtime.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| State + commands | `src/main.js` top section | global `state`, `COMMANDS`, stateful helpers |
| Shared frontend helpers | `src/lib/dashboard-utils.js` | formatting, normalization, markdown rendering, tone helpers |
| Guide viewer slice | `src/features/usage-guides.js` | guide loading, guide viewer rendering, guide-specific event wiring |
| Data-health slice | `src/features/data-health.js` | health cache checks, summary loading, export flow, health-specific rendering and events |
| Recent-reports slice | `src/features/recent-reports.js` | report-history rendering, snapshot jump, artifact open, artifact-path actions |
| Environment/breadth renderers | `src/renderers/environment-breadth.js` | environment layer and watchlist breadth panels rendered as a paired market-participation view |
| Trust summary entry | `src/main.js::renderTrustSummaryPanel` | primary trust verdict with freshness/data-health evidence summaries |
| Startup flow | `src/main.js::loadDashboard` | bundled bootstrap path with `selectedScope` |
| Historical date flow | `src/main.js::loadSelectedSnapshot` | lighter snapshot-only path |
| Scope control | `src/main.js::renderDateSelector` | scope selector + date selector share one control card |
| Environment panel | `src/renderers/environment-breadth.js` | scope-aware environment layer UI paired with watchlist breadth proxy |
| Render pipeline | `src/main.js::render` + `commitRender` | rAF-coalesced DOM writes |
| Health cache flow | `src/features/data-health.js` + `src/main.js::loadDashboard` | 5-minute session cache + manual refresh; bootstrap trigger still starts from dashboard load |
| Visual system | `src/styles.css` | panel/grid/pill/score styles |

## CONVENTIONS
- Plain JS is the real UI runtime; frontend no longer carries a direct `vue` dependency.
- Startup should prefer `dashboard_bundle`; avoid multi-invoke bootstrap drift.
- Scope changes should trigger a full bundle reload; date changes stay on the snapshot-only path.
- Historical date changes should stay on `dashboard_snapshot` unless bundle semantics truly need expansion.
- `render()` schedules; `commitRender()` mutates the DOM.
- Environment UI should read the persisted snapshot contract directly; do not invent derived regime semantics in frontend.
- Data health is intentionally decoupled from normal dashboard correctness and may be stale-cached.
- Trust summary should be the primary “can I trust this snapshot?” entry point; freshness and data health should read as evidence layers beneath it.
- Usage guides load on demand; keep docs rendering isolated from dashboard hot paths.

## ANTI-PATTERNS
- Do **not** add business logic or provider semantics in frontend.
- Do **not** bypass `render()` scheduling with ad-hoc direct DOM writes all over the file.
- Do **not** reintroduce root-level startup fan-out when `dashboard_bundle` already returns the bootstrap payload.
- Do **not** let export/snapshot calls ignore `selectedScope`; scope must flow through bundle, snapshot, and export consistently.
- Do **not** assume data health must refresh every dashboard load.
- Do **not** label Environment Layer as stock-market breadth; it is tracked-universe proxy + liquidity/stress decomposition.

## NOTES
- `src/main.js` is still large and central, but pure formatting / normalization / markdown helpers now live in `src/lib/dashboard-utils.js`.
- Usage-guide interactions now live in `src/features/usage-guides.js`, keeping guide-specific rendering and loading away from the main dashboard flow.
- Data-health interactions now live in `src/features/data-health.js`, keeping cache/load/export behavior decoupled from the rest of the dashboard state flow.
- `main.js` still owns the post-bundle stale-cache check that decides when to trigger `dataHealth.loadSummary()`.
- `renderTrustSummaryPanel()` is now the main trust entry point; `renderRefreshProgress()` may still reuse the lighter inline trust notice after successful refresh.
- Environment and watchlist breadth renderers now live together in `src/renderers/environment-breadth.js`, matching the documented intent that they should coexist as explanation layer + raw proxy view.
- Recent-reports interactions now live in `src/features/recent-reports.js`; `DAILY_REPORT*` items can jump back into matching dashboard snapshots and all artifacts support open/copy actions.
- Recent report labels still encode scope (`DAILY_REPORT_CN`, `DAILY_REPORT_HK`); keep UI formatting and scope inference aligned with backend report types until report metadata grows a first-class `scope` field.
- Next frontend split should continue by area (`snapshot`, remaining render clusters) rather than pushing more utility code back into `main.js` or folding feature-specific behavior back into the main file.
- `renderEnvironmentPanel` and watchlist breadth panel intentionally coexist: environment = explanation layer, breadth panel = raw proxy breakdown.
- `node_modules/` and `dist/` are generated artifacts.

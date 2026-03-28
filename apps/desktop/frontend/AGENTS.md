# FRONTEND KNOWLEDGE BASE

## OVERVIEW
Plain-JS dashboard frontend. Single root render, Tauri invoke bridge, no active Vue runtime.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| State + commands | `src/main.js` top section | global `state`, `COMMANDS`, helpers |
| Startup flow | `src/main.js::loadDashboard` | bundled bootstrap path |
| Historical date flow | `src/main.js::loadSelectedSnapshot` | lighter snapshot-only path |
| Render pipeline | `src/main.js::render` + `commitRender` | rAF-coalesced DOM writes |
| Health cache flow | `src/main.js::loadDataHealthSummary` | 5-minute session cache + manual refresh |
| Visual system | `src/styles.css` | panel/grid/pill/score styles |

## CONVENTIONS
- Plain JS is the real UI runtime; `vue` dependency exists but is not the active implementation.
- Startup should prefer `dashboard_bundle`; avoid multi-invoke bootstrap drift.
- Historical date changes should stay on `dashboard_snapshot` unless bundle semantics truly need expansion.
- `render()` schedules; `commitRender()` mutates the DOM.
- Data health is intentionally decoupled from normal dashboard correctness and may be stale-cached.
- Usage guides load on demand; keep docs rendering isolated from dashboard hot paths.

## ANTI-PATTERNS
- Do **not** add business logic or provider semantics in frontend.
- Do **not** bypass `render()` scheduling with ad-hoc direct DOM writes all over the file.
- Do **not** reintroduce root-level startup fan-out when `dashboard_bundle` already returns the bootstrap payload.
- Do **not** assume data health must refresh every dashboard load.

## NOTES
- `src/main.js` is large and central; if it grows further, split by area (`snapshot`, `health`, `guides`, `render`) before behavior drifts.
- `node_modules/` and `dist/` are generated artifacts.

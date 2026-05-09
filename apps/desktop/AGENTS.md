# DESKTOP APP KNOWLEDGE BASE

## OVERVIEW
Desktop delivery surface split between a plain-JS frontend and a Tauri-native bridge. This is the default operator workflow for the project.

## STRUCTURE
```text
apps/desktop/
├── frontend/   # Vite bundle, plain JS dashboard, feature slices, styles
└── src-tauri/  # Tauri app, command registration, refresh coordinator, config
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| UI composition | `frontend/src/main.js` | root state, scope/date flow, refresh CTA, top-level render |
| Feature-specific UI | `frontend/src/features/*.js` | recent reports, data health, usage guides |
| Frontend-only rules | `frontend/AGENTS.md` | nearest guidance for slice work |
| Native command boundary | `src-tauri/src/lib.rs` | bundle/snapshot/export, refresh coordination, safe artifact opening |
| Tauri config | `src-tauri/tauri.conf.json` | packaging assumes `../frontend/dist` exists |

## CONVENTIONS
- Frontend uses `invoke()` only; it does not talk to DB or arbitrary files directly.
- `src-tauri` stays thin over `app-service`, except for desktop-local coordination and safety such as `RefreshCoordinator` and report-artifact path validation.
- Default CTA is full `Refresh data`; `Run from stage` and `Retry failed stage` are advanced recovery branches.
- Refresh stage order is `ingest -> indicators -> macro -> rotation -> strategy -> signals -> backtests`.
- Startup and scope reloads use `dashboard_bundle`; historical date changes use `dashboard_snapshot`.
- `Trust summary` is the primary top-level verdict; `Recent reports` is a research-results action surface, not just a file-path list.
- Safe artifact opening stays restricted to managed files under `reports/`.

## ANTI-PATTERNS
- Do **not** move analytics/business logic into frontend.
- Do **not** let `src-tauri` duplicate orchestration already in `app-service`.
- Do **not** expose broad native filesystem capabilities when a narrow app-local command is enough.
- Do **not** assume Vue is the active UI runtime; current implementation is plain JS only.
- Do **not** revert startup back into many small invokes unless the bundle path proves insufficient.
- Do **not** make stage reruns look like the default happy path.

## NOTES
- `frontend/node_modules/` and `frontend/dist/` are generated artifacts.
- Help / Usage docs are loaded in-app from project markdown through Tauri.
- Scoped desktop views use scoped regime + scoped environment, while upstream signal/backtest provenance still needs explicit reading in Phase 1.
- Keep this file cross-boundary; push frontend-only detail into `frontend/AGENTS.md` and Tauri-native detail into `src-tauri/AGENTS.md`.

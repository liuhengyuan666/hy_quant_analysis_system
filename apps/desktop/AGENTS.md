# DESKTOP APP KNOWLEDGE BASE

## OVERVIEW
Desktop shell split into webview frontend and Tauri-native bridge. Frontend renders; Rust commands expose curated native capabilities.

## STRUCTURE
```text
apps/desktop/
├── frontend/   # Vite bundle, plain JS dashboard, styles
└── src-tauri/  # Tauri app, command registration, config
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| UI composition | `frontend/src/main.js` | single-page dashboard flow |
| Visual system | `frontend/src/styles.css` | dark dashboard styling |
| Frontend build | `frontend/package.json` | Vite + `@tauri-apps/api` |
| Native command surface | `src-tauri/src/lib.rs` | `dashboard_bundle`, `dashboard_snapshot`, `data_health_summary`, `usage_guides`, async refresh commands |
| Tauri config | `src-tauri/tauri.conf.json` | window + bundle config |

## CONVENTIONS
- Frontend uses `invoke()`; it does not talk to DB or files directly.
- `src-tauri` commands are thin wrappers over `app-service`.
- Dashboard is current single-screen control-room view, not a routed multi-page app.
- Frontend bootstrap now prefers a bundled desktop payload; historical date changes stay on snapshot-only reads.
- Health-check summary and health-report export are exposed in desktop UI, but health is cached and manually refreshable.
- Dashboard defaults to latest selectable analysis date and supports historical date selection from persisted snapshots.
- Refresh is background-job based; the frontend polls progress instead of running heavy refresh work inline.

## ANTI-PATTERNS
- Do **not** move analytics/business logic into frontend.
- Do **not** let `src-tauri` duplicate orchestration already in `app-service`.
- Do **not** expose new native commands broadly without checking least-privilege intent.
- Do **not** assume Vue is the active UI runtime; current implementation is plain JS despite Vue dependency present.
- Do **not** split startup back into many small invokes unless the bundle path proves insufficient.

## NOTES
- `frontend/node_modules/` and `frontend/dist/` are generated artifacts.
- If UI scope grows, decide deliberately: stay plain JS or migrate fully to Vue components.
- Help / Usage docs are in-app, loaded from project markdown files through Tauri.
- `apps/desktop/frontend/AGENTS.md` now covers frontend-only rules; keep this file focused on cross-boundary desktop behavior.

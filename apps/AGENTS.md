# APPS KNOWLEDGE BASE

## OVERVIEW
Delivery surfaces for the quant analysis system: a clap-based CLI for engineering/advanced workflows and a Tauri + Vue 3 desktop app as the default operator surface.

## STRUCTURE
```text
apps/
├── cli/          # clap-based CLI over AppContext
└── desktop/      # Tauri app + Vite/Vue frontend
    ├── frontend/ # Vite bundle, plain JS + Vue 3 (25+ components)
    └── src-tauri/# Tauri native bridge
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| CLI entry & dispatch | `cli/src/main.rs` | `Command` enum + `main()` dispatch |
| CLI command modules | `cli/src/commands/*.rs` | 10 modules; `audit.rs` is a monolith (~3,578 lines) |
| Tauri command surface | `desktop/src-tauri/src/lib.rs` | refresh coordinator, artifact opening, LLM bridge |
| Frontend root | `desktop/frontend/src/main.js` | plain JS orchestration + event bridge |
| Vue app root | `desktop/frontend/src/App.vue` | composes all panels |
| Shared frontend state | `desktop/frontend/src/store.js` | Vue reactive store with ~20 properties |
| Frontend components | `desktop/frontend/src/components/*.vue` | 25 flat Vue panels |

## CONVENTIONS
- `apps/` contains **only** delivery surfaces; no quant logic lives here.
- CLI commands stay thin over `AppContext`; do not fork business logic in match arms.
- Desktop frontend uses `invoke()` only; it does not talk to DB or arbitrary files directly.
- Plain JS (`main.js`) and Vue 3 (`main-vue.js`) coexist during progressive migration; shared state via `store.js`.
- Tauri bridge stays thin over `app-service`; desktop-local coordination (refresh, artifact validation) is OK, quant logic is not.
- Build frontend (`npm run build`) before building Tauri (`cargo build -p quant-desktop`).

## ANTI-PATTERNS
- Do **not** put quant logic, report shaping, or SQL in `src-tauri`.
- Do **not** move analytics/business logic into the frontend.
- Do **not** add broad filesystem access when a narrow app-local command is enough.
- Do **not** load data independently in Vue components; read from shared store instead.
- Do **not** add new audit subcommands to `cli/src/commands/audit.rs` without considering decomposition.

## NOTES
- See nearest AGENTS.md for each subtree: `cli/AGENTS.md`, `desktop/AGENTS.md`, `desktop/src-tauri/AGENTS.md`, `desktop/frontend/AGENTS.md`.
- `apps/desktop/frontend/node_modules/` and `apps/desktop/frontend/dist/` are generated artifacts.

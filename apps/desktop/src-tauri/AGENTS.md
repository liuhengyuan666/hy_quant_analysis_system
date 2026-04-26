# SRC-TAURI KNOWLEDGE BASE

## OVERVIEW
Tauri-native desktop bridge. Owns command registration, refresh-job coordination, scope parsing, and desktop-local file-opening safety.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Entry point | `src/main.rs` | delegates to `quant_desktop_lib::run()` |
| Command registration | `src/lib.rs::run` | `tauri::generate_handler!` list is the real surface |
| Refresh status contract | `src/lib.rs::DashboardRefreshStatus` | shared with frontend polling UI |
| Refresh orchestration | `src/lib.rs::spawn_dashboard_refresh` | suffix-run flow and end-state consistency check |
| Stage parsing/order | `src/lib.rs::RefreshStartStage` | source of truth for stage names and labels |
| Safe artifact opening | `src/lib.rs::validate_report_artifact_path` + `open_report_artifact` | canonical path must stay under `reports/` |
| Packaging assumptions | `tauri.conf.json` | desktop build expects `../frontend/dist` |

## CONVENTIONS
- Heavy work belongs in `AppContext`; use Tauri commands as a bridge, not a second orchestration layer.
- Blocking service calls go through `spawn_blocking`; desktop-local thread/state handling stays in the refresh coordinator.
- `RefreshCoordinator` is in-memory UI state, not persisted workflow history.
- Scope parsing only accepts `global`, `cn`, and `hk`.
- Refresh stage names, labels, and ordering must stay aligned with the frontend selector.
- Artifact opening must remain restricted to canonical files under `reports/`.

## ANTI-PATTERNS
- Do **not** put quant logic, report shaping, or SQL in this crate.
- Do **not** add broad filesystem access when a narrow app-local command is enough.
- Do **not** duplicate stage ordering or scope parsing in ad-hoc branches.
- Do **not** bypass `refresh_consistency_alerts()` on the success path.

## NOTES
- `start_dashboard_refresh` and `retry_dashboard_refresh` implement suffix-run semantics only; there is still no cancel/resume/persisted job state.
- The default user path is full refresh; later-stage reruns are recovery tools.

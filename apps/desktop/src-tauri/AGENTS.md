# SRC-TAURI KNOWLEDGE BASE

## OVERVIEW
Tauri-native desktop bridge. Owns command registration, refresh-job coordination, scope parsing, and desktop-local file-opening safety.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Entry point | `src/main.rs` | delegates to `quant_desktop_lib::run()` |
| Command registration | `src/lib.rs::run` | `tauri::generate_handler!` list is the real surface |
| Refresh status contract | `src/lib.rs::DashboardRefreshStatus` | shared with frontend polling UI |
| Refresh orchestration | `src/lib.rs::spawn_dashboard_refresh` | suffix-run flow, cancel/resume coordination, and end-state consistency check |
| Stage parsing/order | `src/lib.rs::RefreshStartStage` | source of truth for stage names and labels |
| Safe artifact opening | `src/lib.rs::validate_report_artifact_path` + `open_report_artifact` | canonical path must stay under `reports/` |
| Strategy perspectives commands | `src/lib.rs::strategy_scoreboard` + `strategy_attribution` | RV1 四策略评分/归因（attribution 为按需重算，成本较高） |
| LLM bridge | `src/lib.rs::analyze_with_llm` | 透传 action + adversarial 注入级别（full/standard/compact/none） |
| Packaging assumptions | `tauri.conf.json` | desktop build expects `../frontend/dist` |

## CONVENTIONS
- Heavy work belongs in `AppContext`; use Tauri commands as a bridge, not a second orchestration layer.
- Blocking service calls go through `spawn_blocking`; desktop-local thread/state handling stays in the refresh coordinator.
- `RefreshCoordinator` is in-memory UI state plus a cancel flag; durable refresh history is persisted by `app-service` / `market-store` in SQLite.
- Scope parsing only accepts `global`, `cn`, and `hk`.
- Refresh stage names, labels, and ordering must stay aligned with the frontend selector.
- Artifact opening must remain restricted to canonical files under `reports/`.
- `strategy_scoreboard` / `strategy_attribution` 与带 adversarial 参数的 `analyze_with_llm` 已注册为 Tauri 命令；V6/V7 Research Surface 命令仍为 CLI-only，如需暴露在此注册并委托 `app-service`。

## ANTI-PATTERNS
- Do **not** put quant logic, report shaping, or SQL in this crate.
- Do **not** add broad filesystem access when a narrow app-local command is enough.
- Do **not** duplicate stage ordering or scope parsing in ad-hoc branches.
- Do **not** bypass `refresh_consistency_alerts()` on the success path.
- Do **not** implement V6 research document assembly here; call `app-service` methods.

## NOTES
- `start_dashboard_refresh`, `cancel_dashboard_refresh`, and `retry_dashboard_refresh` implement suffix-run refresh with cooperative stage-bound cancellation and resume from the latest cancelled/error persisted job.
- The default user path is full refresh; later-stage reruns are recovery tools.

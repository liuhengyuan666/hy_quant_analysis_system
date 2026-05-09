# PROJECT KNOWLEDGE BASE

## OVERVIEW
Local desktop quant research system. The Rust workspace owns the ingestion-to-report pipeline, the Tauri desktop app is the default operator surface, and `docs/` plus `memory/` are part of the working system rather than side notes.

## STRUCTURE
```text
rust-quant-analysis-system/
├── apps/          # CLI + desktop delivery surfaces
├── crates/        # contracts, engines, persistence, orchestration
├── config/        # instrument universe + runtime inputs
├── docs/          # truth sources, operator guides, active design
├── infra/         # Docker / ClickHouse bootstrap
├── memory/        # durable project context, decisions, history
├── reports/       # exported artifacts; desktop open-file support stays here only
├── runtime/       # agent workflow and memory rules
└── sql/           # storage bootstrap DDL
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Agent rules + memory workflow | `TOOLS.md` + `runtime/memory.md` | highest-priority collaboration rules |
| Current repo truth source | `README.md` + `docs/文档状态说明.md` | start here before old planning docs |
| Orchestration / trust / freshness guards | `crates/app-service/src/lib.rs` | `AppContext`, `build_trust_summary`, `dashboard_bundle_with_scope`, `refresh_consistency_alerts` |
| Shared contracts | `crates/core-domain/src/lib.rs` | `AnalysisScope`, shared snapshot DTOs, provenance fields |
| Persistence boundary | `crates/market-store/src/lib.rs` | all ClickHouse / SQLite IO and latest-date gating |
| Pure macro regime logic | `crates/macro-engine/src/lib.rs` | factor normalization + `GLOBAL/CN/HK` regime rows |
| Report contract | `crates/report-engine/src/lib.rs` | `DashboardSnapshot`, `TrustSummary`, markdown report rendering |
| Desktop shell / refresh bridge | `apps/desktop/src-tauri/src/lib.rs` | command boundary, refresh coordinator, safe artifact opening |
| Frontend composition | `apps/desktop/frontend/src/main.js` | root state, scope/date flow, refresh UI, top-level render |
| Frontend feature slices | `apps/desktop/frontend/src/features/*.js` | recent reports, data health, usage guides |
| Environment + breadth UI | `apps/desktop/frontend/src/renderers/environment-breadth.js` | paired explanation layer + proxy view |
| Current phase memory | `docs/阶段性更新-2026-04-26.md` + `memory/context.md` + `memory/decisions.md` | latest intent, decisions, and next seam |

## CONVENTIONS
- Read `TOOLS.md`, `memory/context.md`, and `memory/decisions.md` before deep edits.
- Desktop `Refresh data` is the default user path. CLI full-chain runs remain explicit engineering / advanced-user paths.
- Startup and scope reloads use `dashboard_bundle`; historical date changes use `dashboard_snapshot`.
- `Trust summary` is the primary trust verdict. `Pipeline freshness` and `Data health` stay as evidence / drilldown layers beneath it.
- `Recent reports` is a research-results entry point: `DAILY_REPORT*` items can reopen matching snapshots, and all artifacts can open or copy paths from desktop.
- Signal freshness / completeness guards are centralized in pipeline diagnostics alerts and enforced by `compute_signals` plus desktop refresh across `GLOBAL`, `CN`, and `HK`.
- `app-service` orchestrates, engine crates compute, and `market-store` persists. CLI, Tauri, and frontend must not own quant logic.
- Scoped dashboard/report semantics and signal/backtest provenance are intentionally explicit; read `analysis_scope`, `regime_basis_scope`, and snapshot scope together.

## ANTI-PATTERNS
- Do **not** access ClickHouse or SQLite outside `crates/market-store`.
- Do **not** move scoring, freshness, or trust logic into Tauri commands or frontend slices.
- Do **not** describe CLI full-chain execution as the default operator workflow in docs or UI.
- Do **not** collapse `dashboard_bundle` and `dashboard_snapshot` back into one path without a measured reason.
- Do **not** widen desktop artifact opening beyond canonical files under `reports/`.
- Do **not** treat the latest available date as trustworthy when diagnostics still report stale or partial decision stages.

## NOTES
- No CI workflow is present; validation is cargo check/test, frontend build, and live CLI/desktop flows.
- `reports/`, `target/`, `apps/desktop/frontend/node_modules/`, and `apps/desktop/frontend/dist/` are generated/runtime artifacts.
- Root planning docs are archive/reference material unless `docs/文档状态说明.md` says they are current.

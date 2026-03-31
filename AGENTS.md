# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-31 17:05:00 +08:00
**Commit:** 15482cd
**Branch:** main

## OVERVIEW
Local desktop quant research system V1. Rust workspace core, Tauri desktop shell, ClickHouse analytics store, SQLite local state, plain-JS dashboard frontend.

## STRUCTURE
```text
rust-quant-analysis-system/
├── apps/          # CLI + desktop shell
├── crates/        # domain, engines, storage, orchestration
├── config/        # instrument universe
├── docs/          # user guides + architecture + breadth planning
├── infra/         # Docker / ClickHouse bootstrap
├── reports/       # exported markdown reports
└── sql/           # ClickHouse / SQLite init DDL
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| End-to-end orchestration | `crates/app-service/src/lib.rs` | `AppContext`; refresh plan, pipeline, dashboard bundle/snapshot, health, export |
| Shared types | `crates/core-domain/src/lib.rs` | canonical DTOs / enums |
| Market data fetch | `crates/data-ingestion/src/lib.rs` | Eastmoney first, Tencent fallback, FRED macro, forward-adjusted bars |
| DB read/write | `crates/market-store/src/lib.rs` | raw ClickHouse/SQLite boundary; scoped dashboard reads live here |
| Reports + dashboard shapes | `crates/report-engine/src/lib.rs` | `DashboardSnapshot`, `DashboardLoadMetrics`, markdown rendering |
| Desktop bridge | `apps/desktop/src-tauri/src/lib.rs` | thin async command surface, refresh coordinator |
| Desktop UI | `apps/desktop/frontend/src/main.js` | single-page dashboard, bundled bootstrap, rAF render scheduling |
| CLI surface | `apps/cli/src/main.rs` | clap commands mirror `AppContext` |
| Scoped CN/HK reporting | `crates/app-service/src/lib.rs` + `apps/cli/src/main.rs` | `ReportScope`, scoped dashboard/export flows |
| Architecture docs | `docs/系统架构与数据流.md` | system flow, date semantics, health flow |
| Module docs | `docs/功能模块与处理逻辑.md` | per-module IO, source, processing logic |
| Breadth planning | `docs/市场广度指标-MA30规划.md` | true stock breadth vs. V1 proxy boundary |
| Freshness diagnostics | `crates/app-service/src/lib.rs` + `apps/cli/src/main.rs` | `pipeline_date_diagnostics`, `pipeline-dates` |

## CODE MAP
| Symbol | Type | Location | Role |
|--------|------|----------|------|
| `AppContext` | struct | `crates/app-service/src/lib.rs` | orchestration facade |
| `dashboard_bundle` | method | `crates/app-service/src/lib.rs` | bundled dashboard bootstrap payload |
| `ReportScope` | enum | `crates/app-service/src/lib.rs` | `Global` / `Cn` / `Hk` routing |
| `compute_watchlist_breadth_snapshot` | method | `crates/app-service/src/lib.rs` | V1 watchlist breadth proxy |
| `pipeline_date_diagnostics` | method | `crates/app-service/src/lib.rs` | stage freshness summary |
| `StorageConfig` | struct | `crates/market-store/src/lib.rs` | runtime paths + DB endpoints |
| `fetch_dashboard_available_dates` | function | `crates/market-store/src/lib.rs` | scoped dashboard date helper |
| `fetch_latest_table_date` | function | `crates/market-store/src/lib.rs` | generic table max-date probe |
| `DashboardSnapshot` | struct | `crates/report-engine/src/lib.rs` | selected-date dashboard payload |
| `DashboardLoadMetrics` | struct | `crates/report-engine/src/lib.rs` | per-stage snapshot timing |
| `start_dashboard_refresh` | command | `apps/desktop/src-tauri/src/lib.rs` | background refresh entrypoint |
| `loadDashboard` | function | `apps/desktop/frontend/src/main.js` | single bundled startup flow |
| `commitRender` | function | `apps/desktop/frontend/src/main.js` | actual DOM write pass |

## CONVENTIONS
- `app-service` orchestrates phases; engine crates compute; `market-store` persists.
- CLI mirrors `AppContext` closely; desktop commands stay thin and call into `app-service` only.
- ClickHouse transport stays raw HTTP + `JSONEachRow`; scoped helpers beat whole-table reads on hot paths.
- Macro regime computation requires buffered FRED history; narrow `[from,to]` windows alone are not enough for forward-filled regime dates.
- Daily bars are canonicalized as **forward-adjusted** (`Eastmoney fqt=1`, `Tencent qfq`).
- Desktop startup now prefers a bundled payload (`dashboard_bundle`) over many small invokes.
- Dashboard/report selection is now explicitly scope-aware: `global` keeps old semantics, `cn` and `hk` isolate latest complete dates per market.
- Historical dashboard date changes use snapshot-only reads; startup and date switching are intentionally different paths.
- Data health is async + session-cached in the desktop UI; it is not part of normal snapshot correctness.

## ANTI-PATTERNS (THIS PROJECT)
- Do **not** add direct DB access outside `crates/market-store`.
- Do **not** put analytics/scoring logic into `src-tauri`, CLI glue, or frontend.
- Do **not** reintroduce whole-table or per-symbol full-history reads on dashboard/report hot paths.
- Do **not** duplicate enum/string conversions across crates.
- Do **not** treat ClickHouse delete+insert mutations as cheap; acceptable for V1 scale only.
- Do **not** rely on async ClickHouse delete mutations for correctness-critical refreshes; current delete path is synchronized intentionally.
- Do **not** label watchlist breadth proxy as true full-market stock breadth.
- Do **not** silently replace global report semantics with CN-only logic; use explicit scope.

## UNIQUE STYLES
- External data is source-layer only: Eastmoney / Tencent / FRED stay in `data-ingestion`.
- Current V1 is a complete chain: data -> indicators -> macro -> rotation -> strategy -> signal -> backtest -> report -> desktop.
- Dashboard/report reads are derived from persisted snapshots and latest-on-or-before date semantics, not live provider calls.
- `report_date` and `regime_as_of_date` are intentionally separate semantics.
- `pipeline-dates` is the fastest truth source for stale-stage diagnosis; use it before assuming dashboard bugs.
- Data-health export is dated by the freshest checked market date, not by wall-clock export time.
- Scoped CN/HK reports intentionally reuse the current global regime; only dates, symbols, rotation, signals, and breadth are market-scoped.
- Watchlist breadth is a V1 proxy over enabled INDEX/ETF instruments, not a stock-universe breadth metric.

## COMMANDS
```bash
docker compose -f infra/docker/docker-compose.yml up -d
cargo run -p quant-cli -- init-storage
cargo run -p quant-cli -- seed-universe
cargo run -p quant-cli -- ingest-daily --from 2026-03-01 --to 2026-03-18
cargo run -p quant-cli -- compute-indicators
cargo run -p quant-cli -- compute-macro --from 2024-01-01 --to 2026-03-18
cargo run -p quant-cli -- compute-rotation
cargo run -p quant-cli -- compute-strategy-preferences
cargo run -p quant-cli -- compute-signals
cargo run -p quant-cli -- check-data-health
cargo run -p quant-cli -- pipeline-dates
cargo run -p quant-cli -- dashboard-dates --scope cn
cargo run -p quant-cli -- dashboard-snapshot --scope hk
cargo run -p quant-cli -- export-report --scope cn
cargo run -p quant-cli -- export-data-health-report
cargo test -p app-service -p report-engine -p market-store
cargo check --workspace
```

## NOTES
- No CI workflow present; validation is cargo check/test plus live CLI flows and data-health checks.
- `README.md` is the landing page; detailed architecture and module docs live in `docs/`.
- `apps/desktop/frontend/node_modules/`, `dist/`, `target/`, and `reports/` are generated/runtime artifacts, not source of truth.
- Root planning docs (`设计规划.md`, `实施路径-v1.md`, `数据源方案评审.md`) still matter for architectural intent even though the active reference set is under `docs/`.

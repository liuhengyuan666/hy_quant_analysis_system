# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-16

## OVERVIEW
Local desktop quant research system. Rust workspace core, Tauri desktop shell, ClickHouse analytics store, SQLite local state.

## STRUCTURE
```text
rust-quant-analysis-system/
├── apps/          # CLI + desktop shell
├── crates/        # domain, engines, storage, orchestration
├── config/        # instrument universe
├── docs/          # user guides + architecture docs
├── infra/         # Docker / ClickHouse bootstrap
├── reports/       # exported markdown reports
└── sql/           # ClickHouse / SQLite init DDL
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| End-to-end orchestration | `crates/app-service/src/lib.rs` | `compute_*`, `run_backtest`, `check_data_health`, `dashboard_snapshot`, `export_report` |
| Shared types | `crates/core-domain/src/lib.rs` | canonical DTOs / enums |
| Market data fetch | `crates/data-ingestion/src/lib.rs` | Eastmoney first, Tencent fallback, FRED for macro, forward-adjusted daily bars |
| DB read/write | `crates/market-store/src/lib.rs` | biggest boundary; owns ClickHouse + SQLite IO |
| Architecture docs | `docs/系统架构与数据流.md` | end-to-end architecture and date semantics |
| Module docs | `docs/功能模块与处理逻辑.md` | per-module IO, source, and processing logic |
| Strategy logic | `crates/strategy-engine/src/lib.rs` | four strategy scorers |
| Final signals | `crates/signal-engine/src/lib.rs` | score aggregation + label mapping |
| Backtest logic | `crates/backtest-engine/src/lib.rs` | next-day-open, long-only, simple portfolio model |
| Reports | `crates/report-engine/src/lib.rs` | dashboard snapshot + markdown rendering |
| Desktop bridge | `apps/desktop/src-tauri/src/lib.rs` | thin Tauri command surface |
| Desktop UI | `apps/desktop/frontend/src/main.js` | single-page dashboard |

## CODE MAP
| Symbol | Location | Role |
|--------|----------|------|
| `AppContext` | `crates/app-service/src/lib.rs` | main orchestration facade |
| `StorageConfig` | `crates/market-store/src/lib.rs` | runtime paths + DB endpoints |
| `build_strategy_preferences` | `crates/strategy-engine/src/lib.rs` | four-strategy scoring |
| `build_signal_snapshots` | `crates/signal-engine/src/lib.rs` | final action layer |
| `run_signal_backtest` | `crates/backtest-engine/src/lib.rs` | signal-driven portfolio simulation |
| `build_dashboard_snapshot` | `crates/report-engine/src/lib.rs` | selected-date dashboard aggregation with regime as-of fallback |
| `check_data_health` | `crates/app-service/src/lib.rs` | provider/gap/jump health summary |
| `start_dashboard_refresh` | `apps/desktop/src-tauri/src/lib.rs` | async background refresh entrypoint |

## CONVENTIONS
- `app-service` orchestrates phases; engine crates compute; `market-store` persists.
- CLI mirrors `AppContext` almost 1:1.
- Desktop commands stay thin; no business logic in Tauri bridge.
- ClickHouse reads/writes use raw HTTP + `JSONEachRow`.
- Summary-return pattern: `failed_items` / `failed_symbols` instead of hard-failing every batch.
- Daily bars are now canonicalized as **forward-adjusted** (`Eastmoney fqt=1`, `Tencent qfq`).
- Desktop refresh is now background-job based with progress status and disabled controls while active.

## ANTI-PATTERNS (THIS PROJECT)
- Do **not** add direct DB access outside `crates/market-store`.
- Do **not** put analytics/scoring logic into `src-tauri` or frontend.
- Do **not** expand `AppContext` blindly; extract services when adding new verticals.
- Do **not** duplicate enum/string conversions in multiple crates.
- Do **not** treat ClickHouse mutations as cheap; current delete+insert style is V1-only.

## UNIQUE STYLES
- External data is source-layer only: Eastmoney / Tencent / FRED stay in `data-ingestion`.
- Dashboard/report reads are derived from persisted snapshots, with dashboard defaulting to latest selectable date and supporting historical date selection.
- Current V1 is a complete chain: data -> indicators -> macro -> rotation -> strategy -> signal -> backtest -> report -> desktop.
- Health-check flow exists: provider reachability + gap/jump/turnover checks before trusting trend signals.
- Docs now exist for both user workflow and engineering architecture inside `docs/`.

## COMMANDS
```bash
cargo run -p quant-cli -- init-storage
cargo run -p quant-cli -- seed-universe
cargo run -p quant-cli -- ingest-daily --from 2024-01-01 --to 2024-12-31
cargo run -p quant-cli -- compute-indicators
cargo run -p quant-cli -- compute-macro --from 2024-12-01 --to 2025-01-31
cargo run -p quant-cli -- compute-rotation
cargo run -p quant-cli -- compute-strategy-preferences
cargo run -p quant-cli -- compute-signals
cargo run -p quant-cli -- check-data-health
cargo run -p quant-cli -- run-backtest
cargo run -p quant-cli -- dashboard-dates
cargo run -p quant-cli -- dashboard-snapshot
cargo run -p quant-cli -- export-report
cargo run -p quant-cli -- export-data-health-report
npm run build
cargo check --workspace
```

## NOTES
- No test suite / CI workflow found yet.
- `README.md` is the landing page; detailed architecture and module docs now live in `docs/`.
- `apps/desktop/frontend/node_modules/` exists locally; treat as generated artifact, not source of truth.

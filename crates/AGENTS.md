# CRATES KNOWLEDGE BASE

## OVERVIEW
Rust workspace implementation layer. Contracts, pure compute, persistence, orchestration, and report shaping all live here.

## STRUCTURE
```text
crates/
├── core-domain/      # shared models + enums
├── data-ingestion/   # external market/macro fetch + canonical adjustment rules
├── indicator-engine/ # technical indicators
├── macro-engine/     # macro snapshots + per-scope regime scoring
├── rotation-engine/  # RS / momentum ranking
├── strategy-engine/  # four-strategy scoring
├── signal-engine/    # final action labels
├── backtest-engine/  # signal-driven simulation
├── report-engine/    # dashboard/report payload + markdown rendering
├── market-store/     # ClickHouse + SQLite IO
├── app-service/      # orchestration facade, trust assembly, refresh guards
└── task-runner/      # placeholder utility crate
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add / change shared DTOs | `core-domain/` | type first; child AGENTS file covers contract traps |
| Add new factor/provider | `data-ingestion/` + engine crate | fetch outside, compute inside |
| Change pure macro scoring | `macro-engine/` | regime math only; no fetch/persist |
| Add persistence | `market-store/` | fetch/insert pair, enum/string mapping, scoped date helpers |
| Change trust / dashboard bootstrap / recent reports | `app-service/` + `report-engine/` | `app-service` assembles; `report-engine` shapes |
| Add pipeline step | `app-service/` | expose summary DTO + CLI/Tauri hook |

## CONVENTIONS
- `core-domain` owns serializable contracts; other crates depend inward on it.
- Engine crates should stay pure whenever possible; `macro-engine` especially must not fetch or persist.
- `market-store` owns all SQL, ClickHouse/SQLite IO, and date-gating helpers.
- `report-engine` shapes dashboard/report payloads; `app-service` injects trust summary, pipeline diagnostics, and recent reports.
- Signal alignment and refresh consistency use diagnostics alerts as the single source of truth.
- Dashboard/report are scope-aware; strategy/signal/backtest computations are also scope-aware with provenance fields (analysis_scope, regime_basis_scope, signal_scope) in the data contract.

## ANTI-PATTERNS
- Do **not** fetch HTTP data from `strategy-engine`, `signal-engine`, `backtest-engine`, or `macro-engine`.
- Do **not** duplicate snapshot DTOs outside `core-domain`.
- Do **not** add more persistence code to `app-service`.
- Do **not** let `market-store` absorb domain scoring logic.
- Do **not** put orchestration or trust assembly into `report-engine`.

## HOTSPOTS
- `market-store/src/lib.rs` is still the largest and most coupled crate file.
- `app-service/src/lib.rs` is still the orchestration monolith; review nearby helpers before adding more flow logic.
- `data-ingestion/src/lib.rs` owns source semantics such as forward-adjustment and provider validation.
- `report-engine/src/lib.rs` is a shared payload contract; field drift breaks CLI, Tauri, and frontend together.

## NOTES
- `task-runner` is still placeholder-level.
- Tests are sparse and local; practical validation is cargo check/test plus live CLI/report/desktop flows.

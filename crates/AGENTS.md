# CRATES KNOWLEDGE BASE

## OVERVIEW
Rust workspace implementation layer. Domain types, compute engines, persistence, orchestration, report shaping, and most runtime semantics live here.

## STRUCTURE
```text
crates/
├── core-domain/      # shared models + enums
├── data-ingestion/   # external market/macro fetch + canonical adjustment rules
├── indicator-engine/ # technical indicators
├── macro-engine/     # macro snapshots + regime
├── rotation-engine/  # RS / momentum ranking
├── strategy-engine/  # four-strategy scoring
├── signal-engine/    # final action labels
├── backtest-engine/  # signal-driven simulation
├── report-engine/    # dashboard/report render
├── market-store/     # ClickHouse + SQLite IO
├── app-service/      # orchestration facade
└── task-runner/      # placeholder utility crate
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add new snapshot type | `core-domain` | type first, then store + service |
| Add new factor/provider | `data-ingestion` + engine crate | fetch outside, compute inside |
| Add derived analytics | dedicated engine crate | keep pure if possible |
| Add persistence | `market-store` | fetch/insert pair, enum/string mapping |
| Add pipeline step | `app-service` | expose summary DTO + CLI/Tauri hook |
| Add data quality checks | `app-service` + `report-engine` | keep checks orchestration-level, not store-level |
| Change dashboard/report payload | `report-engine` + `app-service` | shape in report-engine, load in app-service |

## CONVENTIONS
- `core-domain` owns contracts; other crates depend inward on it.
- Engine crates should not know ClickHouse / Tauri / CLI.
- `market-store` owns all SQL, HTTP-to-ClickHouse, and persistence shaping.
- `app-service` is orchestration, not a dumping ground for scoring formulas or SQL.
- `report-engine` stays pure shaping/rendering: snapshot structs, markdown rendering, dashboard-date semantics.
- Dashboard hot paths should use scoped store helpers, not whole-table fetches.

## ANTI-PATTERNS
- Do **not** fetch HTTP data from `strategy-engine`, `signal-engine`, or `backtest-engine`.
- Do **not** duplicate snapshot DTOs outside `core-domain`.
- Do **not** add more persistence code to `app-service`.
- Do **not** let `market-store` absorb domain logic.
- Do **not** put snapshot fetch orchestration into `report-engine`.

## HOTSPOTS
- `market-store/src/lib.rs` = largest, most coupled crate file.
- `app-service/src/lib.rs` = orchestration monolith; review before adding more flows.
- `data-ingestion/src/lib.rs` = source semantics matter here; forward-adjustment + validation live there.
- `report-engine/src/lib.rs` = dashboard/report payload contract; field drift breaks CLI/Tauri/frontend together.

## NOTES
- `task-runner` is still placeholder-level.
- Tests are sparse and local; most validation is cargo check/test plus live CLI/report flows.

# CRATES KNOWLEDGE BASE

## OVERVIEW
Rust workspace implementation layer. Domain types, compute engines, persistence, orchestration all live here.

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
| Add pipeline step | `app-service` | expose summary DTO + CLI hook |
| Add data quality checks | `app-service` + `report-engine` | keep checks orchestration-level, not store-level |

## CONVENTIONS
- `core-domain` owns contracts; other crates depend inward on it.
- Engine crates should not know ClickHouse / Tauri / CLI.
- `market-store` owns all SQL, HTTP-to-ClickHouse, and persistence shaping.
- `app-service` is orchestration, not a dumping ground for scoring formulas.

## ANTI-PATTERNS
- Do **not** fetch HTTP data from `strategy-engine`, `signal-engine`, or `backtest-engine`.
- Do **not** duplicate snapshot DTOs outside `core-domain`.
- Do **not** add more persistence code to `app-service`.
- Do **not** let `market-store` absorb domain logic.

## HOTSPOTS
- `market-store/src/lib.rs` = largest, most coupled crate file.
- `app-service/src/lib.rs` = orchestration monolith; review before adding more flows.
- `data-ingestion/src/lib.rs` = source semantics matter here; forward-adjustment + validation now live there.
- `app-service/src/lib.rs` = now also owns data-health orchestration; review before adding more summary flows.

## NOTES
- `task-runner` is still placeholder-level.
- No explicit crate-level tests found yet.

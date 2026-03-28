# MARKET-STORE KNOWLEDGE BASE

## OVERVIEW
Persistence boundary. Owns ClickHouse/SQLite initialization, fetch/insert helpers, enum/string bridging, and several report/backtest aggregation reads.

## STRUCTURE
```text
crates/market-store/
└── src/lib.rs   # all storage logic currently centralized here
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Runtime paths / DB endpoints | `StorageConfig` | root discovery, sqlite path, universe path |
| Generic ClickHouse helpers | `fetch_clickhouse_text`, `execute_clickhouse_query` | raw HTTP layer |
| Snapshot writes | `insert_*` functions | delete+insert mutation pattern |
| Snapshot reads | `fetch_*` functions | `JSONEachRow` decode + enum remap |
| Backtest persistence | `insert_backtest_result` | run + trades + equity curve |
| Report metadata | `insert_report_snapshot` | report artifact registry |
| Universe sync | `insert_instruments` | table sync now treats config as source of truth |

## CONVENTIONS
- ClickHouse transport is raw HTTP, not typed client usage.
- Shared blocking client is centralized; do not recreate clients per query.
- Reads usually parse `JSONEachRow` line-by-line.
- Enum-like DB values are bridged with `serde_json::Value` remapping when needed.
- Current dedupe/update approach = `ALTER TABLE ... DELETE` then reinsert.
- `instrument` sync is now full refresh style to avoid stale symbol drift.
- Dashboard hot path now has scoped helpers (`fetch_dashboard_available_dates`, `fetch_latest_market_regime_on_or_before`, date-scoped rotation/signal fetches, symbol-range bar/indicator fetches).

## ANTI-PATTERNS
- Do **not** add scoring, ranking, or label-selection logic here.
- Do **not** duplicate query helpers; centralize transport/parsing utilities.
- Do **not** scatter enum/string conversion across other crates.
- Do **not** assume ClickHouse numeric JSON always arrives as numeric type; some counters arrive as strings.
- Do **not** send dashboard/report paths back to whole-table fetch helpers when scoped helpers exist.

## REVIEW NOTES
- This file is now a god-module; split by domain (`bars`, `signals`, `backtest`, `reports`) before much more growth.
- Mutation-heavy refresh is acceptable for V1 volume, but not a scalable long-term pattern.
- `project_root()` is heuristic-based; keep runtime path assumptions explicit.
- `instrument` schema now includes `display_symbol`; keep config, schema, and insert payload in lockstep.
- If startup timings regress, inspect scoped helper query count first, then parse cost.

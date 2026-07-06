# MARKET-STORE KNOWLEDGE BASE

## OVERVIEW
Persistence boundary. Owns ClickHouse/SQLite initialization, fetch/insert helpers, enum/string bridging, and several report/backtest aggregation reads.

## STRUCTURE
```text
crates/market-store/
└── src/
    ├── lib.rs          # pub use re-exports of all domain modules
    ├── core.rs         # StorageConfig + raw ClickHouse HTTP transport
    ├── sqlite.rs       # SQLite app config / credentials
    ├── instruments.rs  # universe sync
    ├── bars.rs         # daily bar IO
    ├── indicators.rs   # indicator snapshot IO
    ├── macro.rs        # macro snapshot IO
    ├── regime.rs       # market regime IO
    ├── environment.rs  # environment snapshot IO
    ├── rotation.rs     # rotation rank IO
    ├── strategy.rs     # strategy preference IO
    ├── signals.rs      # signal snapshot IO
    ├── backtest.rs     # backtest result IO
    ├── reports.rs      # report artifact registry
    └── dates.rs        # date-gating helpers
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
- Delete mutations for correctness-critical refresh paths now run with `mutations_sync = 1`.
- `instrument` sync is now full refresh style to avoid stale symbol drift.
- Dashboard hot path now has scoped helpers (`fetch_dashboard_available_dates`, `fetch_latest_market_regime_on_or_before`, date-scoped rotation/signal fetches, symbol-range bar/indicator fetches).
- Scope-aware completeness checks now rely on `fetch_distinct_entity_count_for_date_in_symbols` for per-market symbol coverage.
- `environment_snapshot` is now a persisted ClickHouse table with scoped read/write helpers.
- Macro snapshot refresh must delete by `(factor_name, date range)`, not by raw date range only, to avoid erasing healthy factor history on partial provider failures.
- `fetch_macro_snapshots_in_range` exists specifically to support compute-macro history fallback.

## ANTI-PATTERNS
- Do **not** add scoring, ranking, or label-selection logic here.
- Do **not** duplicate query helpers; centralize transport/parsing utilities.
- Do **not** scatter enum/string conversion across other crates.
- Do **not** assume ClickHouse numeric JSON always arrives as numeric type; some counters arrive as strings.
- Do **not** send dashboard/report paths back to whole-table fetch helpers when scoped helpers exist.
- Do **not** add new `ALTER TABLE ... DELETE` refresh paths without deciding whether they also require synchronous mutation semantics.
- Do **not** fake scoped coverage in app-service without a storage-level symbol filter; use the scoped distinct-count helper.
- Do **not** reintroduce whole-range macro deletes that wipe unrelated factor history.

## REVIEW NOTES
- `src/lib.rs` is now a thin re-export hub; domain logic lives in the 14 modules below it.
- Mutation-heavy refresh is acceptable for V1 volume, but not a scalable long-term pattern.
- Synchronous delete mutations improve correctness but increase refresh latency; correctness wins on report/dashboard tables.
- `project_root()` is heuristic-based; keep runtime path assumptions explicit.
- `instrument` schema now includes `display_symbol`; keep config, schema, and insert payload in lockstep.
- If startup timings regress, inspect scoped helper query count first, then parse cost.
- Empty scoped max-date queries should resolve to `None`, not synthetic `1970-01-01`-style placeholders.

## SCHEMA EVOLUTION POLICY

Any struct deserialized from ClickHouse `JSONEachRow` (e.g., `RotationRankSnapshot`, `SignalSnapshot`, `StrategyPreferenceSnapshot`, `IndicatorSnapshot`, `DailyBar`, `MacroSnapshot`) is a persisted JSON schema. Adding a new field to such a struct without a backward-compatibility mechanism will break deserialization of existing rows.

**Rule**: Every new field on a persisted DTO MUST use one of these three patterns:

1. **`#[serde(default)]` on the field** — serde fills the default when the key is missing in stored JSON. Preferred for primitive types and `String`.
2. **Struct-level `#[serde(default)]` with a `Default` impl** — used when most fields have sensible defaults.
3. **Manual `serde_json::Value` remap in the fetch function** — patch missing keys before calling `serde_json::from_value`. Documented example: `fetch_strategy_preferences` (lines 2021-2037).

**Anti-pattern**: Adding a required field with no default and no remap. This causes runtime deserialization crashes on old data.

**When in doubt**: Add `#[serde(default)]`.

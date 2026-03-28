# breadth-ma30-v1

## Goal

Ship a **V1 watchlist breadth proxy** for the current `INDEX + ETF` universe.

This is **not** true stock-market breadth.

## Scope

- Compute `% of tracked instruments with close > ma30`
- Split output into `CN` and `HK`
- Show in dashboard
- Include in markdown report
- Support historical date selection through existing dashboard snapshot flow

## Non-goals

- No `InstrumentType::Stock`
- No full-market stock universe
- No new market breadth ClickHouse table
- No new Tauri command if existing `dashboard_snapshot` can carry data
- No signal-score integration
- No `<10%` / `<20%` stock-breadth extreme labels on this proxy

## Preferred architecture

Use **on-demand computation** from existing persisted data:

- `quant.daily_bar.close`
- `quant.indicator_snapshot.ma30`
- current enabled universe from `config/universe.json`

Do **not** add a new persistence layer in V1.

## Data shape

Add report-layer structs:

- `WatchlistBreadthMarketSnapshot`
- `WatchlistBreadthSnapshot`

Attach to `DashboardSnapshot` as:

- `watchlist_breadth: Option<WatchlistBreadthSnapshot>`

Per market fields:

- `market`
- `universe_label`
- `eligible_count`
- `above_count`
- `breadth_pct`
- `breadth_pct_sma5`
- `breadth_5d_delta`
- `range_low_60d`
- `range_high_60d`
- `range_position_60d`
- `status_label`

## Computation location

Implement in `crates/app-service/src/lib.rs` as an internal helper such as:

- `compute_watchlist_breadth_snapshot(&self, report_date: NaiveDate) -> Result<Option<WatchlistBreadthSnapshot>>`

Logic:

1. Load enabled universe
2. Split into `CN` / `HK`
3. For each symbol, use existing store accessors:
   - `fetch_daily_bars(symbol)`
   - `fetch_indicator_snapshots(symbol)`
4. Align data to `report_date`
5. Include in denominator only when both `close` and `ma30` exist on that date
6. Count `close > ma30`
7. Build recent 60-observation breadth series
8. Derive `sma5`, `5d delta`, local range position, and conservative status label

## Files to change

### `crates/report-engine/src/lib.rs`
- Add breadth snapshot structs
- Extend `DashboardSnapshot`
- Extend `render_markdown_report(...)`

### `crates/app-service/src/lib.rs`
- Add breadth aggregation helper
- Compute breadth inside `dashboard_snapshot(...)`

### `apps/desktop/src-tauri/src/lib.rs`
- No new command unless unavoidable
- Reuse existing `dashboard_snapshot(report_date)` flow

### `apps/desktop/frontend/src/main.js`
- Add `renderWatchlistBreadthPanel(snapshot)`
- Insert panel after market regime card

### `apps/desktop/frontend/src/styles.css`
- Add styles for breadth cards / pills / bars as needed

## UI wording

- Title: `Watchlist Breadth (MA30)`
- Subtitle should explicitly say tracked `INDEX + ETF`
- Show note: `Proxy only · not full-market stock breadth`

## Status labels

Use conservative proxy wording only:

- `improving`
- `weakening`
- `weak`
- `neutral`
- `strong`
- `near_local_low`
- `near_local_high`
- `unavailable`

Deterministic priority rules:

1. `eligible_count == 0` -> `unavailable`
2. `range_position_60d <= 0.20` -> `near_local_low`
3. `range_position_60d >= 0.80` -> `near_local_high`
4. `breadth_5d_delta >= 10.0` -> `improving`
5. `breadth_5d_delta <= -10.0` -> `weakening`
6. `breadth_pct < 35.0` -> `weak`
7. `breadth_pct > 65.0` -> `strong`
8. otherwise -> `neutral`

## Validation

### Backend
- `dashboard_snapshot` includes breadth data
- report export includes breadth section
- historical selected date changes breadth output

Executable checks:
- `cargo run -p quant-cli -- dashboard-snapshot` returns JSON with `watchlist_breadth`
- `cargo run -p quant-cli -- dashboard-dates` returns selectable dates
- `cargo run -p quant-cli -- dashboard-snapshot --date YYYY-MM-DD` returns breadth for the chosen date
- `cargo run -p quant-cli -- export-report` produces markdown containing `## Watchlist Breadth (MA30)` and proxy wording

### UI
- breadth panel renders for latest date
- breadth updates when selecting historical dates
- missing data degrades gracefully
- wording never implies true full-market breadth

Edge-case checks:
- if one market has `eligible_count = 0`, return `status_label = unavailable`
- if there are fewer than 5 valid breadth observations, allow `breadth_pct_sma5` / `breadth_5d_delta` to be null
- if there are fewer than 60 valid breadth observations, allow range metrics to be null

Executable UI QA:
- Launch desktop app with `cargo run -p quant-desktop`
- Wait for dashboard load to complete
- Confirm a new `Watchlist Breadth (MA30)` panel appears **after** the market regime card
- Confirm the panel subtitle/note explicitly mentions tracked `INDEX + ETF` and `Proxy only · not full-market stock breadth`
- Record the displayed latest-date `breadth_pct` / `status_label` for CN and/or HK
- Use the existing date selector to switch to a historical dashboard date returned by `dashboard-dates`
- Confirm the breadth panel remains visible and the displayed `report_date`-dependent values change or correctly remain consistent with backend data for that date
- If one market has no eligible instruments, confirm that market block shows an unavailable/empty-state rendering instead of crashing or disappearing silently

### Safety
- no schema changes
- no ingestion pipeline changes
- no scoring changes

## Deferred to V2

- stock universe support
- Tushare stock ingestion
- true CN all-stock breadth
- HK main-board stock breadth
- breadth persistence table
- breadth-based regime/signal scoring

# v2-roadmap

## Goal

Turn the current V1 index/ETF research system into a more layered, explainable V2 without over-expanding into infrastructure or stock-universe work that the repo is not yet ready for.

## Core V2 Direction

1. Environment layer becomes more truthful
   - per-scope regime (`GLOBAL`, `CN`, `HK`)
   - breadth integrated into environment
   - liquidity / stress proxies
2. Strategy layer becomes stateful
   - left probe vs trend confirm vs full trend vs de-risk
3. Execution layer becomes actionable
   - position sizing prototype
   - drawdown control prototype

## Current Repo Grounding

- V1 already has ingest, indicators, macro/regime, rotation, strategy preference, signals, backtest, dashboard, markdown export, health diagnostics, pipeline freshness diagnostics, and scoped GLOBAL/CN/HK report flows.
- Current universe is still INDEX / ETF centric.
- Scoped dashboard/report now uses scoped regime + scoped environment.
- Strategy, signal, and backtest still intentionally rely on GLOBAL regime semantics.
- `app-service` and `market-store` are still monolithic pressure points.
- There is no stock fundamentals pipeline.

## Heavy-User Priority Order

### P0. Close the trust gap first
- Make signal/backtest provenance explicit in dashboard/report/export.
- Do not let CN/HK views imply fully scoped strategy/backtest until that path is real.
- Extend `AnalysisScope` into strategy/signal/backtest only after provenance is clear.

### P1. Turn refresh into a validated research cycle
- Refresh completion should surface pipeline freshness, latest-day completeness, macro-source degradation, and trust judgement together.
- Exported reports should carry freshness/health/basis context.
- Latest backtest should show scope, date range, config, and whether it matches the current snapshot.

### P2. Improve interpretability and drilldown
- Structured signal reasons instead of debug-style explanation strings.
- Symbol drilldown for trend structure, strategy breakdown, historical signal/rotation context.
- Scope-aware diagnostic presentation for symbol-level trust.

### P3. Improve repeated-use efficiency
- Persist scope/date/UI preferences.
- Make recent reports actionable (open/copy/compare).
- Add partial rerun / retry / cancel around refresh flow.

### P4. Defer deeper performance work until semantics are trusted
- Ingest parallelization, refresh concurrency, and deeper optimization are valid later work, but not the first user-value milestone.

## Completed Foundation

### 1. Per-scope regime
- `GLOBAL / CN / HK` regime rows are now in place.

### 2. Breadth as environment input
- Breadth is now persisted inside `environment_snapshot`.

### 3. Light liquidity/stress proxies
- The first environment-layer proxy set is already wired into dashboard/report.

## Do Now

### 1. Trust closure around scope semantics
- Make GLOBAL-regime basis explicit wherever scoped dashboard/report still shows global-scored signal/backtest outputs.
- Promote scoped strategy/signal/backtest from plan to implementation only after provenance is visible.

### 2. Validated refresh / export / backtest cycle
- Refresh completion should surface pipeline freshness, latest-day completeness, macro-source degradation, and trust judgement together.
- Exported reports should carry freshness/health/basis context.
- Latest backtest should show scope, date range, config, and whether it matches the current snapshot.

### 3. Dual-stage strategy framework
- Introduce explicit states:
  - `NO_TRADE`
  - `LEFT_PROBE`
  - `CONFIRM_ADD`
  - `FULL_TREND`
  - `DE_RISK`

### 4. Better explainability and drilldown
- Structured signal reasons instead of debug-style explanation strings.
- Symbol drilldown for trend structure, strategy breakdown, historical signal/rotation context.
- Scope-aware diagnostic presentation for symbol-level trust.

### 5. Execution prototype
- Add position sizing and drawdown control prototype to the backtestable execution flow.

### 6. Workflow efficiency
- Persist scope/date/UI preferences.
- Make recent reports actionable (open/copy/compare).
- Add partial rerun / retry / cancel around refresh flow.

## Module-Level Execution Checklist

### P0-A. Provenance contract and trust labeling — ✅ IMPLEMENTED
- `crates/core-domain/src/lib.rs`: add provenance fields for strategy/signal/backtest outputs.
- `crates/backtest-engine/src/lib.rs`: expose backtest run metadata and config summary.
- `sql/clickhouse/001_init.sql` + `crates/market-store/src/lib.rs`: persist provenance columns and read helpers.
- `crates/app-service/src/lib.rs`: aggregate provenance into dashboard/export payloads.
- `crates/report-engine/src/lib.rs` + desktop/CLI surfaces: render regime-basis / scope-basis notices.
- **Note**: DTOs, storage schema, trust summary aggregation, and report rendering all complete.

### P0-B. Real scoped strategy / signal / backtest path — ✅ IMPLEMENTED
- `crates/strategy-engine/src/lib.rs` + `crates/app-service/src/lib.rs`: make strategy computation scope-aware.
- `crates/signal-engine/src/lib.rs` + `crates/app-service/src/lib.rs`: make signal generation scope-aware.
- `crates/backtest-engine/src/lib.rs` + `crates/app-service/src/lib.rs`: run backtests on scoped signal streams.
- `apps/cli/src/main.rs` + `apps/desktop/src-tauri/src/lib.rs`: expose scope-compatible command/runtime paths.
- **Note**: `compute_strategy_preferences` iterates Global/Cn/Hk; `signal-engine` uses `row.regime_basis_scope`; `run_backtest` takes scope param.

### P1. Validated research cycle — ✅ MOSTLY IMPLEMENTED
- `crates/app-service/src/lib.rs`: build a unified trust summary from pipeline freshness + data health + provenance.
- `apps/desktop/src-tauri/src/lib.rs` + `apps/desktop/frontend/src/main.js`: attach trust judgement to refresh completion.
- `crates/report-engine/src/lib.rs`: add freshness / health / basis context to exports.
- desktop backtest card: show scope/date/config/match-to-current-snapshot metadata.
- **Note**: unified trust summary exists; refresh completion triggers dashboard reload with trust panel; exports carry context; backtest metadata includes scope/date/config/match.

### P2. Structured explanation and drilldown — NOT STARTED
- `crates/core-domain/src/lib.rs` + `crates/signal-engine/src/lib.rs`: replace explanation-string-only flow with structured reason model.
- `crates/app-service/src/lib.rs` + `crates/report-engine/src/lib.rs`: define symbol detail payload.
- `apps/desktop/src-tauri/src/lib.rs` + `apps/desktop/frontend/src/main.js`: implement click-through symbol drilldown.
- diagnostics presentation: keep provider health global, make symbol trust views scope-aware.

### P3. Workflow efficiency — PARTIALLY DONE
- SQLite preference path: persist scope/date/viewer state through market-store/app-service/Tauri/frontend.
- Recent report actions: open artifact, copy path, compare previous. **(done)**
- Refresh controls: cancel, retry failed stage, partial rerun; likely requires real task orchestration instead of the current linear thread. **(not done)**

## Recommended Safe Order

1. provenance DTO/schema
2. provenance store/app-service aggregation
3. dashboard/report/export trust labeling
4. unified trust summary after refresh
5. real scoped strategy/signal/backtest path
6. structured signal reasons
7. drilldown UI
8. workflow-efficiency wins

## Defer

### 1. Full stock-universe value system
- No stock universe, no fundamentals pipeline, no stock instrument flow yet.
- This is not a good immediate V2 target.

### 2. Automatic trading / broker integration
- Outside current product scope.

### 3. Advanced portfolio optimizer
- Risk-budget / covariance optimizer can wait until execution layer is proven.

### 4. Full crate/service rewrite
- Start with internal module boundaries first; do not make architecture churn the first milestone.

## Not Necessary Yet

- Web / cloud / multi-user platform
- Minute-level or high-frequency system
- Full per-bar provider provenance platform
- Enterprise deployment workflows

## Recommended Phases

### Phase 1: Environment semantics
- per-scope regime
- breadth environment integration
- light liquidity/stress proxies

### Phase 2: Trust closure and research-loop validation
- explicit signal/backtest provenance
- refresh completion with freshness + health judgement
- trustable exports

### Phase 3: Strategy state machine
- left probe vs trend confirm vs de-risk
- strategy-state outputs in dashboard/report

### Phase 4: Explainability and drilldown
- structured signal explanation
- symbol-level detail workflows
- scope-aware diagnostic presentation

### Phase 5: Execution prototype
- staged position sizing
- drawdown control
- execution-aware backtest logic

### Phase 6: Workflow efficiency and selective performance work
- saved UI preferences
- recent-report actions
- partial rerun / retry / cancel
- targeted performance improvements only if refresh cost becomes recurrent pain

## Acceptance Criteria

- Scoped dashboard/report semantics no longer rely on global regime approximation
- A user can explain why the system is in `LEFT_PROBE`, `CONFIRM_ADD`, or `DE_RISK`
- Position-sizing guidance is backtestable, not just prose
- V2 remains index/ETF-first and does not silently expand into unsupported stock fundamentals

## Executable QA Scenarios

### QA 1: Per-scope regime correctness
- Run:
  - `cargo run -p quant-cli -- dashboard-snapshot --scope global`
  - `cargo run -p quant-cli -- dashboard-snapshot --scope cn`
  - `cargo run -p quant-cli -- dashboard-snapshot --scope hk`
- Verify: scoped dashboard/report uses regime rows matching the selected scope, not only the legacy global row

### QA 2: Breadth environment integration
- Run:
  - `cargo run -p quant-cli -- compute-macro --from 2026-03-01 --to 2026-03-31`
  - `cargo run -p quant-cli -- compute-rotation`
  - `cargo run -p quant-cli -- compute-strategy-preferences`
  - `cargo run -p quant-cli -- compute-signals`
  - `cargo run -p quant-cli -- dashboard-snapshot --scope cn`
- Verify: environment output includes breadth-derived fields and those fields change when breadth conditions change

### QA 2A: Provenance and trust labeling
- Run:
  - `cargo run -p quant-cli -- init-storage`
  - `cargo run -p quant-cli -- dashboard-snapshot --scope cn --date 2026-04-01`
  - `cargo run -p quant-cli -- export-report --scope cn --date 2026-04-01`
- Verify:
  - signal payload includes explicit provenance fields (`analysis_scope`, `regime_basis_scope`)
  - latest backtest payload includes scope/date/config provenance when available
  - exported markdown visibly states signal/backtest basis and snapshot-match context

### QA 3: Strategy state machine
- Replay three windows:
  - weak: `cargo run -p quant-cli -- export-report --date 2026-03-10 --scope cn`
  - improving: `cargo run -p quant-cli -- export-report --date 2026-03-20 --scope cn`
  - strong: `cargo run -p quant-cli -- export-report --date 2026-03-28 --scope cn`
- Verify: system emits expected state family (`LEFT_PROBE`, `CONFIRM_ADD`, `FULL_TREND`, `DE_RISK`) with readable justification

### QA 4: Execution prototype
- Planned command contract:
  - `cargo run -p quant-cli -- run-backtest --from 2025-01-01 --to 2026-03-31 --execution-mode static`
  - `cargo run -p quant-cli -- run-backtest --from 2025-01-01 --to 2026-03-31 --execution-mode staged`
- Verify: trades, exposure, and drawdown behavior differ in the expected direction and are reproducible from the state logic

### QA 5: Scope compatibility
- Run:
  - `cargo run -p quant-cli -- dashboard-dates --scope global`
  - `cargo run -p quant-cli -- dashboard-dates --scope cn`
  - `cargo run -p quant-cli -- dashboard-dates --scope hk`
  - `cargo run -p quant-cli -- export-report --scope cn`
  - `cargo run -p quant-cli -- export-report --scope hk`
- Verify `global` behavior remains backward compatible after new V2 changes
- Verify `cn` / `hk` exports still produce distinct report files and dates under mixed-market lag conditions

## Guardrails

- Keep global behavior backward compatible
- Do not start stock-fundamentals V2 until data model and provider pipeline exist
- Do not trigger a large crate split before feature semantics are stable
- Prefer evolutionary layering over rewrite-driven architecture churn

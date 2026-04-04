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
- Scoped reporting exists, but regime is still effectively global.
- `app-service` and `market-store` are still monolithic pressure points.
- There is no stock fundamentals pipeline.

## Do Now

### 1. Per-scope regime
- Replace current global-only regime semantics in scoped reports with actual `GLOBAL / CN / HK` regime rows.
- Keep global path intact.

### 2. Breadth as environment input
- Upgrade breadth from display-only proxy to environment factor.
- Use breadth level + breadth momentum + repair/exhaustion semantics.

### 3. Dual-stage strategy framework
- Introduce explicit states:
  - `NO_TRADE`
  - `LEFT_PROBE`
  - `CONFIRM_ADD`
  - `FULL_TREND`
  - `DE_RISK`

### 4. Execution prototype
- Add position sizing and drawdown control prototype to the backtestable execution flow.

### 5. Better explainability
- Environment decomposition
- Strategy decision breakdown
- Report/dashboard wording that explains stage transitions clearly

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

### Phase 2: Strategy state machine
- left probe vs trend confirm vs de-risk
- strategy-state outputs in dashboard/report

### Phase 3: Execution prototype
- staged position sizing
- drawdown control
- execution-aware backtest logic

### Phase 4: Explainability and diagnostics
- decision decomposition
- scope-aware diagnostic clarity
- more explicit report justification

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

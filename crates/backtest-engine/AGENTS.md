# BACKTEST-ENGINE KNOWLEDGE BASE

## OVERVIEW
Signal-driven backtest simulation crate. Given signal snapshots, daily bars, and optional strategy-state sizing, produces equity curves, trades, and summary statistics.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Config contract | `src/lib.rs::BacktestConfig` | scope, capital, fees, slippage, drawdown limits |
| Result contract | `src/lib.rs::BacktestResult` | summary + trades + equity curve |
| Trade record | `src/lib.rs::BacktestTrade` | run_id, date, symbol, action, price, quantity |
| Equity point | `src/lib.rs::BacktestEquityPoint` | run_id, date, equity, drawdown |
| Simulation entry | `src/lib.rs::run_signal_backtest` | main loop: sell exits -> state sizing -> buy candidates -> equity mark |
| Stats helpers | `src/lib.rs` private fns | `annualized_cagr`, `max_drawdown`, `sharpe_ratio` |

## CONVENTIONS
- Keep this crate pure: no storage, no HTTP, no orchestration.
- `BacktestConfig` carries provenance scopes (`analysis_scope`, `signal_scope`, `regime_basis_scope`) so results are self-describing.
- Trade execution uses next-open prices with slippage.
- Drawdown limit triggers liquidation at the next open, not intraday.
- Strategy-state sizing (`use_strategy_state=true`) dynamically caps holdings and capital exposure per regime state.
- Config summary string is for human-readable report labeling, not parsing.

## ANTI-PATTERNS
- Do **not** add storage or provider fetch logic here.
- Do **not** change drawdown liquidation semantics without updating report wording.
- Do **not** bypass slippage or fee modeling for "simpler" tests.
- Do **not** treat `BacktestResult` as a live trading execution record.

## NOTES
- `market-store` depends on this crate for `BacktestResult` types; avoid circularity by keeping types here and storage logic in `market-store`.
- This crate has no tests yet; the simulation loop is validated through live CLI/desktop backtest flows.

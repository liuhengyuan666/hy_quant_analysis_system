# SIGNAL-ENGINE KNOWLEDGE BASE

## OVERVIEW
Final signal-label generation crate. Blends strategy preference, regime, and rotation inputs into `SignalSnapshot` rows with explicit `SignalReason` provenance.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Builder | `src/lib.rs::build_signal_snapshots` | strategies + regimes + rotations -> (snapshots, stats) |
| Score clamp | `src/lib.rs::clamp_score` | bounds to 0-100 |
| Label mapping | `src/lib.rs::label_from_score` | thresholds: 80 StrongBuy, 65 Buy, 50 Watch, 35 Hold, 20 Reduce, else Sell |
| Best strategy | `src/lib.rs::best_strategy_score` | extracts the winning strategy's raw score |
| Alignment | `src/lib.rs::aligned_strategies` | counts strategies >= 60 as "aligned" |

## CONVENTIONS
- Keep this crate pure: no I/O, no storage, no provider logic.
- Final score = `strategy * 0.45 + alignment * 0.15 + regime * 0.20 + rotation * 0.20`.
- Missing regime falls back to 50.0; missing rotation falls back to 40.0.
- `SignalBuildStats` tracks missing-regime and missing-rotation counts for diagnostics.
- `SignalReason.summary` is Chinese-language human-readable justification.

## ANTI-PATTERNS
- Do **not** add fetch or persistence logic here.
- Do **not** change label thresholds without updating report legend and frontend signal panel.
- Do **not** remove provenance fields (`analysis_scope`, `regime_basis_scope`) from output.

## NOTES
- This is the last computation stage before backtest and report; any scoring change ripples to both.
- No tests currently; validated through dashboard signal panel and live report export.

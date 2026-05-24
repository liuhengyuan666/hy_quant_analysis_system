# INDICATOR-ENGINE KNOWLEDGE BASE

## OVERVIEW
Pure technical-indicator computation crate. Consumes `DailyBar` slices and produces `IndicatorSnapshot` vectors with MA, EMA, MACD, RSI, ATR, and volume MA.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Main builder | `src/lib.rs::build_indicator_snapshots` | entry point: bars -> Vec<IndicatorSnapshot> |
| MA helper | `src/lib.rs::rolling_mean` | simple moving average over f64 slice |
| EMA helper | `src/lib.rs::ema_series` | exponential moving average |
| RSI helper | `src/lib.rs::rsi_series` | 14-period RSI with smoothed averages |
| ATR helper | `src/lib.rs::atr_series` | average true range over DailyBar slice |

## CONVENTIONS
- Keep this crate pure: no I/O, no storage, no provider logic.
- All series helpers return `Vec<Option<f64>>` to represent insufficient warm-up periods naturally.
- MACD signal line is EMA-9 of the MACD line; histogram = MACD - signal.
- `build_indicator_snapshots` maps bars 1:1 to snapshots; missing values are `None`.

## ANTI-PATTERNS
- Do **not** add fetch or persistence logic here.
- Do **not** change indicator periods without coordinating upstream `app-service` and downstream strategy/signal scoring.
- Do **not** emit `NaN` or sentinel values instead of `None` for insufficient warm-up.

## NOTES
- This is one of the smallest engine crates but sits on the hot path for every symbol during indicator refresh.
- No tests currently; correctness is validated through downstream report/dashboard consistency.

# ROTATION-ENGINE KNOWLEDGE BASE

## OVERVIEW
Relative-strength and momentum ranking crate. Computes per-symbol RS windows and ranks a universe by blended momentum score per trading day.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Rank builder | `src/lib.rs::build_rotation_ranks` | bars-by-symbol -> daily ranked RotationRankSnapshot rows |
| Latest view | `src/lib.rs::latest_rotation_view` | filters and sorts to the most recent date only |
| RS helper | `src/lib.rs::compute_rs_window` | pct-change over N-day window |

## CONVENTIONS
- Keep this crate pure: no I/O, no storage, no provider logic.
- Momentum score = `rs_20 * 0.5 + rs_60 * 0.3 + rs_120 * 0.2`.
- Missing longer windows gracefully fall back to shorter available windows (`rs_120` -> `rs_60` -> `rs_20`).
- Rank is 1-based per day; ties break by symbol lexical order.

## ANTI-PATTERNS
- Do **not** add fetch or persistence logic here.
- Do **not** change the momentum-score weights without coordinating strategy-engine and signal-engine scoring.
- Do **not** treat rotation rank as a chase-high ranking in documentation or UI.

## NOTES
- Very small crate (~76 lines); the ranking formula is a shared contract with strategy-engine and signal-engine.
- No tests currently; validated through live dashboard rotation panel and report output.

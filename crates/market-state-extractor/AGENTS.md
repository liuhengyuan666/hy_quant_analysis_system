# MARKET-STATE-EXTRACTOR KNOWLEDGE BASE

## OVERVIEW
Semantic market-state observation layer (ADR-053). Converts aligned `DailyBar` + `IndicatorSnapshot` series into structured observations across Trend, Liquidity, Volatility, and optional Breadth dimensions.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Top-level observation | `src/lib.rs::MarketStateObservation` | date, scope, trend/liquidity/volatility/breadth/drawdown |
| Trend classification | `src/lib.rs::extract_trend_with_method` | MA20 short-term, MA60 medium-term, multiple methods |
| Liquidity extraction | `src/lib.rs::extract_liquidity_from_bars` | volume regime + turnover strength |
| Volatility extraction | `src/lib.rs::extract_volatility` | 20-day realized vol, annualized |
| Main entry point | `src/lib.rs::build_market_state_observations` | aligned bars + indicators → observations |

## CONVENTIONS
- Keep this crate pure: no fetch, no persistence, no orchestration.
- `bars` and `indicators` must be aligned 1:1 by date (same length, same order).
- Breadth is always `None` here; it must be injected later via `with_breadth()` when multi-constituent data is available.
- Trend direction methods: Baseline (default), RelativeSlope, Percentile, ZScore.

## ANTI-PATTERNS
- Do **not** synthesize breadth from single-index data here.
- Do **not** add storage or HTTP concerns here.
- Do **not** change trend method thresholds without coordinating `gt-regime-generator` and `regime-audit`.
- Do **not** assume all indicators are present; gracefully handle `None` MA values.

## NOTES
- `slope_approx` uses simple linear regression over the closing price window.
- `calculate_momentum_strength` maps 6-month return [-30%, +30%] → [0, 100].
- Volatility thresholds: >40% Spike, >25% Elevated, >12% Normal, else Low.
- Tests cover uptrend observation, drawdown, volatility regime, and volume regime.

# STRATEGY-ENGINE KNOWLEDGE BASE

## OVERVIEW
Four-strategy preference scoring crate. Each strategy implements the `StrategyScorer` trait over an `AnalysisContext` (bar + indicators + regime + rotation); the best strategy and alignment are returned per symbol-day.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Trait contract | `src/lib.rs::StrategyScorer` | `kind() -> StrategyKind`, `score(&AnalysisContext) -> f64` |
| Context | `src/lib.rs::AnalysisContext` | bar, indicators, optional regime/rotation, scopes |
| Builder | `src/lib.rs::build_strategy_preferences` | contexts -> Vec<StrategyPreferenceSnapshot> |
| Attribution | `src/lib.rs::build_strategy_attributions` + `ScoreBreakdown` + `AttributionDriver` | RV1 新增：每策略因子级归因（factor/value/contribution/note），供 `strategy-perspectives --mode detail` 重算校验 drift |
| ValueLeft | `src/lib.rs::ValueLeftScorer` | mean-reversion: low RSI + distance below MA20 + liquidity |
| TrendPullback | `src/lib.rs::TrendPullbackScorer` | uptrend + price between MA20/MA60 + mid RSI |
| TrendBreakout | `src/lib.rs::TrendBreakoutScorer` | price above MA20>MA60 + MACD hist + rotation |
| MomentumRight | `src/lib.rs::MomentumRightScorer` | high momentum score + rank bonus + trend + MACD |

## CONVENTIONS
- Keep this crate pure: no I/O, no storage, no provider logic.
- Scores are clamped 0-100; alignment counts how many strategies exceed 60.
- `best_strategy` is the highest-scoring kind; `confidence` is that highest score.
- All scorers gracefully fallback when indicators/regime/rotation are missing.

## ANTI-PATTERNS
- Do **not** add fetch or persistence logic here.
- Do **not** change scoring formulas without coordinating signal-engine weights and report wording.
- Do **not** remove the `StrategyScorer` trait abstraction; it keeps the four strategies testable and swappable.

## NOTES
- `AnalysisContext` is built upstream in `app-service` from bars, indicators, regime, and rotation rows.
- Attribution（`build_strategy_attributions`）与存储分数同源计算，drift 应≈0；`app-service::strategy_perspectives` 的 detail 路径用它做防漂移校验。
- No tests currently; validated through downstream signal panel and report export.

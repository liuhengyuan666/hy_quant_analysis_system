# MACRO-ENGINE KNOWLEDGE BASE

## OVERVIEW
Pure macro/regime computation crate. Builds normalized macro snapshots and per-scope market regime rows from prepared factor series and anchor bars.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Prepared factor input | `src/lib.rs::MacroFactorSeries` | upstream fetch layer builds this input |
| Factor normalization | `src/lib.rs::build_macro_snapshots` | rolling min/max -> bounded score |
| Regime construction | `src/lib.rs::build_market_regimes` | emits `GLOBAL`, `CN`, `HK` rows |
| Behavioral guardrail | `src/lib.rs` test `build_market_regimes_outputs_global_cn_hk_rows` | encodes expected scope output shape |

## CONVENTIONS
- Keep this crate pure and deterministic: no fetch, no persistence, no `AppContext` orchestration.
- Upstream `app-service` fetches FRED history, merges persisted fallback rows, loads CN/HK anchors, and persists outputs.
- Current factor groups assume `vix`, `dollar_index`, `us10y`, and `fed_funds` names.
- `macro_as_of_date` and factor fallback semantics are resolved by upstream history availability; this crate just consumes the prepared rows.

## ANTI-PATTERNS
- Do **not** add storage or HTTP concerns here.
- Do **not** hardcode desktop/report presentation logic in regime computation.
- Do **not** change factor names or scope labels without coordinated `app-service` and storage updates.

## NOTES
- CN/HK trend inputs come from upstream anchor bars (`000300`, `HSI`).
- Global trend is the average of available anchors, not a separate fetched market series.

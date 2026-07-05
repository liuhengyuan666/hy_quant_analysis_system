# EXECUTION-ENGINE KNOWLEDGE BASE

## OVERVIEW
V5 pre-close execution filter. Pattern library that answers "When to buy" for signal-positive candidates based on real-time intraday snapshots.

## STRUCTURE
```text
crates/execution-engine/src/
├── lib.rs      # re-exports
├── types.rs    # ExecutionState, ReasonTag, IntradaySnapshot, ExecutionDecision
├── engine.rs   # pattern matching logic
└── fetcher.rs  # Tencent real-time quote fetch + enrichment
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Decision types | `src/types.rs` | `ExecutionState`, `ReasonTag`, `IntradaySnapshot`, `ExecutionDecision` |
| Pattern logic | `src/engine.rs` | `analyze`, `check_no_chase`, `check_distribution`, `check_strong_close` |
| Real-time fetch | `src/fetcher.rs` | Tencent `qt.gtimg.cn` API, GB18030 decode, MA5/volume enrichment |

## CONVENTIONS
- Keep this crate pure: no persistence, no orchestration, no signal generation.
- `IntradaySnapshot` is enriched by the caller with MA5 and volume ratio from stored data.
- Patterns are checked in priority order: NoChase → Distribution → StrongClose → Wait.
- On fetch failure, every symbol degrades to `Skip` with `DataUnavailable`; never panic.

## ANTI-PATTERNS
- Do **not** add signal scoring or regime logic here.
- Do **not** persist execution samples in this crate; `app-service` writes to `reports/execution-samples/`.
- Do **not** change pattern thresholds without updating the operator manual and preclose report wording.
- Do **not** treat `ExecutionState` as a trading signal; it is a tactical execution filter over existing signals.

## NOTES
- Real-time data comes from Tencent API; Eastmoney fallback is not implemented here.
- Output is written to `reports/execution-samples/YYYY-MM-DD.json`.
- 90-day observation period; do not claim performance advantage or optimize parameters.

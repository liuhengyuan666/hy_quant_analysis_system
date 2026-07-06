# LLM-CONTEXT KNOWLEDGE BASE

## OVERVIEW
Builds an LLM-specific, 8-dimension `ResearchContext` from `DashboardSnapshot` for LLM-powered analysis. This is distinct from the canonical `ResearchContext` in `crates/research-context`, which is the V6 cross-consumer semantic contract.

## STRUCTURE
```text
crates/llm-context/src/
├── lib.rs            # module declarations + re-exports
├── semantic_state.rs # LLM ResearchContext + 8 context types (Market, Liquidity, Breadth, Rotation, Regime, Signals, Macro, Risk)
├── builder.rs        # ContextBuilder::build() from DashboardSnapshot
├── feature_engine.rs # feature extraction utilities
└── compression.rs    # context compression for token budget
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add new LLM context dimension | `semantic_state.rs` | add type + field to LLM ResearchContext |
| Change LLM context building | `builder.rs` | ContextBuilder methods |
| Extract features | `feature_engine.rs` | feature utilities |
| Compress context | `compression.rs` | token reduction |

## CONVENTIONS
- The LLM `ResearchContext` is the primary output of this crate; all 8 dimensions are populated for prompts.
- Builder uses heuristics from `DashboardSnapshot` data (e.g., liquidity_score thresholds for pressure).
- Many fields have TODO placeholders (spread, yield_curve_status, dollar_strength, skewness, kurtosis, tail_index).
- Compression is WIP (Wave 2).
- Do not confuse this crate with `research-context`, which owns the canonical V6 semantic model.

## ANTI-PATTERNS
- Do **not** add persistence or fetch logic here.
- Do **not** change context types without updating `research-skills` consumers.
- Do **not** remove TODO markers; they track Wave 2+ implementation items.
- Do **not** merge this crate into `research-context`; the two ResearchContext types serve different consumers.

## NOTES
- Depends on `report-engine` for `DashboardSnapshot` type.
- `compression.rs` is Wave 2 work (semantic compression for token budget).
- V4.5 `research-skills` consumes the LLM context built here.

# RESEARCH-CONTEXT KNOWLEDGE BASE

## OVERVIEW
Builds structured `ResearchContext` from `DashboardSnapshot` for LLM-powered analysis. Extracts semantic state across market, liquidity, breadth, rotation, regime, signals, macro, and risk dimensions.

## STRUCTURE
```text
crates/research-context/src/
├── lib.rs            # module declarations + re-exports
├── semantic_state.rs # ResearchContext + 8 context types (Market, Liquidity, Breadth, etc.)
├── builder.rs        # ContextBuilder::build() from DashboardSnapshot
├── feature_engine.rs # feature extraction utilities
└── compression.rs    # context compression for token budget
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add new context dimension | `semantic_state.rs` | add type + field to ResearchContext |
| Change context building | `builder.rs` | ContextBuilder methods |
| Extract features | `feature_engine.rs` | feature utilities |
| Compress context | `compression.rs` | token reduction |

## CONVENTIONS
- `ResearchContext` is the primary output; all 8 dimensions are always populated.
- Builder uses heuristics from DashboardSnapshot data (e.g., breadth_pct thresholds for condition).
- Many fields have TODO placeholders with hardcoded values (confidence: 0.8, leadership_stability: 0.7).
- Compression is WIP (Wave 2).

## ANTI-PATTERNS
- Do **not** add persistence or fetch logic here.
- Do **not** change context types without updating `research-skills` consumers.
- Do **not** remove TODO markers; they track Wave 2+ implementation items.

## NOTES
- Depends on `report-engine` for `DashboardSnapshot` type.
- 10 TODO items in `builder.rs` for unimplemented data-quality computations.
- `compression.rs` is Wave 2 work (semantic compression for token budget).

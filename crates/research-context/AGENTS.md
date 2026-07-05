# RESEARCH-CONTEXT KNOWLEDGE BASE

## OVERVIEW
Owns the canonical, consumer-neutral semantic model for V6 research outputs: `ResearchContext` and `TrustLevel`. Used by `reporting`, `report-builder`, and `app-service` to ensure all consumers speak the same research language.

## STRUCTURE
```text
crates/research-context/src/
└── lib.rs
    ├── TrustLevel          # Unassessed / Low / Medium / High
    └── ResearchContext     # canonical cross-consumer research summary
        ├── version
        ├── scope
        ├── date
        ├── market_state
        ├── breadth
        ├── rotation
        ├── signal
        ├── divergence
        └── trust
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Change trust levels | `TrustLevel` enum | storage/report/frontend must stay aligned |
| Change canonical research model | `ResearchContext` | affects `reporting`, `report-builder`, and all consumers |
| Add new summary dimension | `ResearchContext` + child summary structs | prefer additive changes; do not modify existing field semantics |
| Change market/breadth/rotation/signal summary | child structs inside `lib.rs` | keep consumer-neutral |

## CONVENTIONS
- `ResearchContext` is a summary-only, consumer-neutral contract. It does NOT carry raw data or consumer-specific fields.
- New fields are additive; avoid changing existing semantics without ADR review.
- `TrustLevel` is the canonical trust enum; do not use raw strings for trust in new code.
- This crate has minimal dependencies (`chrono`, `serde`, `core-domain`) by design.

## ANTI-PATTERNS
- Do **not** add raw data, provider metadata, or consumer-specific fields here.
- Do **not** add I/O, HTTP, or DB access here.
- Do **not** rename variants or reshape fields without coordinated updates in `reporting` / `report-builder` / consumers.
- Do **not** duplicate this model in `llm-context`; `llm-context` builds a separate LLM-specific context from `DashboardSnapshot`.

## NOTES
- `reporting` depends on this crate for `ResearchContext`.
- `report-builder` uses it to build `ReportDocument` metadata and sections.
- `app-service` populates it from engine outputs before passing it to document builders.
- V6 canonical semantic model; treat as a stable boundary.

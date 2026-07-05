# REPORTING KNOWLEDGE BASE

## OVERVIEW
V6 presentation-model crate. Owns the cross-consumer presentation contracts: `ReportingSnapshot`, `ReportDocument`, `ReportSection`, `SectionContent`, and the `Formatter` trait. No business logic and no rendering implementation beyond the trait boundary.

## STRUCTURE
```text
crates/reporting/src/
└── lib.rs
    ├── ReportingSnapshot    # generated_at + ResearchContext
    ├── ReportDocument       # layout + sections + metadata
    ├── ReportMetadata       # title, generated_at, scope, date
    ├── ReportLayout         # Detail / Summary / Review
    ├── ReportSection        # kind + title + content
    ├── SectionKind          # Trend, Breadth, Rotation, Signal, Divergence, Stretch, Analytics, ...
    ├── SectionContent       # Markdown / Table / Json
    ├── TableData            # headers + rows
    └── Formatter trait      # render_document, render_section, finalize
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Change presentation snapshot | `ReportingSnapshot` | only `generated_at` + canonical `ResearchContext` |
| Change document model | `ReportDocument`, `ReportMetadata`, `ReportLayout` | layout enum + metadata contract |
| Change section taxonomy | `SectionKind`, `SectionContent` | new section kinds affect all formatters |
| Change table contract | `TableData` | headers + rows of strings |
| Implement a new renderer | `Formatter` trait | Markdown / Text / JSON / HTML / PDF in consumer crates |

## CONVENTIONS
- This crate is pure presentation model: no computation, no I/O, no consumer-specific formatting.
- `ReportingSnapshot` must not duplicate scope/date already present in `ResearchContext`; it only adds `generated_at`.
- `ReportDocument` is the rendering-neutral document model; concrete rendering happens in consumer `Formatter` implementations.
- `SectionKind` is the shared vocabulary for document sections; add new kinds sparingly.

## ANTI-PATTERNS
- Do **not** add business logic or scoring here.
- Do **not** add I/O, HTTP, or DB access here.
- Do **not** duplicate metadata already in `ResearchContext` into `ReportDocument`.
- Do **not** add consumer-specific rendering code here (e.g., Vue templates, HTML).
- Do **not** widen `SectionContent` with implementation-specific types.

## NOTES
- Depends on `research-context` for the canonical `ResearchContext` type.
- `Formatter::finalize(self) -> String` assumes text-based output; future binary consumers (PDF) may need a different trait or adapter.
- V6 Reporting Platform is frozen.

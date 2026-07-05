# REPORT-RENDERER KNOWLEDGE BASE

## OVERVIEW
Rendering implementations for the V6 `reporting::Formatter` trait. Provides `MarkdownFormatter`, `JsonFormatter`, and `TextFormatter` to turn `ReportDocument` into concrete output strings. The crate also retains deprecated V4 skill-composer types (`ResearchSummary`, `ComposerRegistry`) that are no longer used by the V4.5 research layer.

## STRUCTURE
```text
crates/report-renderer/src/
├── lib.rs          # render() helper + deprecated V4 ResearchSummary/ComposerRegistry
├── markdown.rs     # MarkdownFormatter for ReportDocument
├── json.rs         # JsonFormatter for ReportDocument
└── text.rs         # TextFormatter for ReportDocument
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Render ReportDocument to Markdown | `src/markdown.rs::MarkdownFormatter` | V6 Formatter implementation |
| Render ReportDocument to JSON | `src/json.rs::JsonFormatter` | V6 Formatter implementation |
| Render ReportDocument to plain text | `src/text.rs::TextFormatter` | V6 Formatter implementation |
| Top-level render helper | `src/lib.rs::render<F: Formatter>` | iterates sections and drives formatter |
| V4 dead code | `src/lib.rs` ResearchSummary/ComposerRegistry | no longer called by V4.5 prompts; marked for future cleanup |

## CONVENTIONS
- Keep this crate pure: no storage, no HTTP, no orchestration.
- Formatter implementations must only read `ReportDocument` / `ReportSection` / `SectionContent`.
- Markdown output is the primary human-readable format; text and JSON are for machine consumers.
- V4 skill-composer code (`ResearchSummary`, `ComposerRegistry`, `MarketRegimeComposer`, `SectorRotationComposer`) is deprecated and should not be extended.

## ANTI-PATTERNS
- Do **not** add LLM calls here.
- Do **not** add persistence or provider logic here.
- Do **not** duplicate full daily-report rendering already in `report-engine`.
- Do **not** extend the deprecated V4 composer registry; V4.5 research layer uses `research-skills::action.rs` only.
- Do **not** change output wording without checking that trust-level and regime semantics stay consistent.

## NOTES
- Implements the `Formatter` trait from `crates/reporting`.
- Depends on `reporting`, `llm-context` (for deprecated V4 types), and `core-domain`.
- Chinese-language output strings are hardcoded in Markdown/Text formatters; changing them affects exported reports.
- V4 composer code is kept for reference but is not part of the active V6 reporting platform.

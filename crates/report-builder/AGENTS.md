# REPORT-BUILDER KNOWLEDGE BASE

## OVERVIEW
V6 document-assembly crate. Owns document-specific `ReportInput` structs and three frozen builders (`ResearchReportBuilder`, `ReviewReportBuilder`, `AuditReportBuilder`). Emits `ReportDocument` from the `reporting` crate.

## STRUCTURE
```text
crates/report-builder/src/
└── lib.rs
    ├── ReportBuilder trait           # Pending Evaluation; no artificial implementors
    ├── *ReportInput structs          # document-specific, transient payloads
    ├── ResearchReportBuilder         # SRD / Stretch / Analytics documents
    ├── ReviewReportBuilder           # Quarterly review documents
    └── AuditReportBuilder            # Rotation ranking / scoreboard tables
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add a new document type | `src/lib.rs` | add `ReportInput` + builder method; requires ADR amendment |
| Change SRD document | `ResearchReportBuilder::build_srd` + `build_srd_sections` | Signal-Regime Divergence output |
| Change Stretch document | `ResearchReportBuilder::build_stretch` + `build_stretch_sections` | market-stretch four-dimension output |
| Change Analytics document | `ResearchReportBuilder::build_analytics` + `build_analytics_sections` | conditional forward-return output |
| Change Quarterly review | `ReviewReportBuilder::build` + `build_review_sections` | 90-day window aggregation |
| Change Audit tables | `AuditReportBuilder` | rotation ranking / symbol scoreboard |

## CONVENTIONS
- `ReportInput` structs carry only document-specific data computed upstream in `app-service`. They do NOT repeat metadata (scope/date/generated_at).
- Builders consume a `ReportingSnapshot` (from `reporting`) plus the matching `ReportInput`.
- Output is a `ReportDocument` (`reporting` crate) containing `ReportSection` values.
- The `ReportBuilder` trait is intentionally `Pending Evaluation`; do not add artificial implementations just to satisfy it.
- All builder APIs are frozen for V6; new document profiles require ADR amendment.

## ANTI-PATTERNS
- Do **not** put metadata (scope/date/generated_at) into `ReportInput` structs.
- Do **not** add artificial `ReportBuilder` implementations.
- Do **not** perform I/O, HTTP, or DB access here.
- Do **not** add consumer-specific rendering (e.g., HTML) here; use `Formatter` implementations in consumers.
- Do **not** duplicate research computation that belongs in `core-domain::research`.

## NOTES
- This crate depends on `reporting`, `research-context`, and `core-domain`.
- Tests in `src/lib.rs` exercise all builders with dummy snapshots.
- V6 Reporting Platform is frozen; treat this crate as a stable boundary.

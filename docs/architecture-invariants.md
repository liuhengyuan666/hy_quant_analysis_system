# Architecture Invariants

> **Status**: Frozen as of V6 Reporting Platform Foundation.  
> These invariants are the non-negotiable rules of the layered architecture. Every PR must keep them intact.

---

## Invariant-01 — Semantic Layer stays consumer-neutral

`ResearchContext` never contains consumer-specific fields.

- No `Option<SrdReportInput>`.
- No `markdown`, `chart_data`, `timeline`, `window`, `narrative`, or `confidence` fields.
- `ResearchContext` is the canonical semantic model; presentation semantics live in `ReportInput`.

---

## Invariant-02 — ResearchDataset never crosses the app-service boundary

`ResearchDataset` is an internal, ephemeral raw-data container.

- It is never `pub` outside `crates/app-service/src/`.
- CLI, Desktop, API, GPT, Formatter, and Email consumers must never reference it.
- It must not be cached by any consumer.

---

## Invariant-03 — ReportInput owns payload, not metadata

`ReportInput` structs carry document-specific payload only.

- No `scope`, `date`, or `generated_at` fields.
- Metadata travels in `ReportingSnapshot`.
- `ReportInput` may reference `ResearchContext`; `ResearchContext` must never reference `ReportInput`.

---

## Invariant-04 — Formatter never performs domain computation

Formatters render `ReportDocument` to Markdown, Text, or JSON.

- No `classify_level`, no `percentile_rank`, no `weighted_stretch_overall` inside formatters.
- All domain computation belongs in `core-domain::research` or the computation workspace (`ResearchSnapshot`).

---

## Invariant-05 — core-domain never imports report-builder

`core-domain` owns shared contracts and pure domain helpers.

- It must not depend on `report-builder`, `report-renderer`, CLI, or `app-service`.
- Reusable research computation lives under `core-domain::research`.

---

## Invariant-06 — ReportingSnapshot is the only metadata carrier

Scope, date, and generation timestamp belong to `ReportingSnapshot` (and its derived `ReportMetadata`).

- Do not duplicate metadata into `ReportInput` or `ReportDocument`.
- Builders derive `ReportMetadata` from `ReportingSnapshot`.

---

## Invariant-07 — CLI does not write raw SQL for research data

CLI handlers consume research data through `AppContext` methods or typed `market-store` functions.

- No `format!("SELECT ...")` for research queries in CLI.
- Raw SQL for non-research concerns (e.g., date resolution, bar fetching) must remain thin and localized.

---

## Invariant-08 — Builder methods accept concrete ReportInput

Builders do not use aggregation containers like `ResearchDetail` or `AuditDetail`.

- `ResearchReportBuilder::build_srd(&ReportingSnapshot, &SrdReportInput)`.
- `AuditReportBuilder::build_scoreboard(&ReportingSnapshot, &ScoreboardReportInput)`.
- No `Option<...>` dispatch inside builder entry points.

---

## Invariant-09 — Domain helpers stay in core-domain::research

All reusable research computation (`classify_*`, `score_*`, `rank_*`, `percentile_*`) belongs to `core-domain::research`.

- They must not live in `report-builder`, CLI, or `app-service`.
- Empty placeholder modules (`signal`, `breadth`, `rotation`) are architecture intent, not technical debt.

---

## Invariant-10 — Production Surface remains frozen

`DashboardSnapshot`, `export-report`, and daily dashboard semantics are the production surface.

- Reporting-layer refactors must not change these contracts.
- New consumers should consume the reporting layer, not replace the production surface.

---

## How to use this document

1. Before submitting a PR that touches `ResearchContext`, `ReportInput`, `ReportBuilder`, `Formatter`, or `core-domain::research`, read the relevant invariant.
2. If the change violates an invariant, the default action is **do not merge**; escalate to an ADR amendment if the violation is intentional.
3. Add new invariants only through ADR review; do not grow this list casually.

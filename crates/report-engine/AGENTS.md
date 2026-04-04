# REPORT-ENGINE KNOWLEDGE BASE

## OVERVIEW
Pure snapshot shaping and markdown rendering layer for dashboard/report outputs.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Snapshot contract | `src/lib.rs::DashboardSnapshot` | shared CLI/Tauri/frontend payload |
| Timing contract | `src/lib.rs::DashboardLoadMetrics` | dashboard stage timings |
| Scope contract | `src/lib.rs::DashboardSnapshot.scope` | `GLOBAL` / `CN` / `HK` markdown + UI labeling |
| Environment contract | `src/lib.rs::DashboardSnapshot.environment` | scope-aware environment payload rendered in UI + markdown |
| Legacy date collector | `collect_dashboard_dates` | old whole-set helper; prefer scoped store path upstream |
| Date-specific builder | `build_dashboard_snapshot_for_date` | current selected-date assembly path |
| Report rendering | `render_markdown_report` | markdown output contract |
| Health rendering | `render_data_health_report` | data-health markdown output |

## CONVENTIONS
- Keep this crate pure: no storage reads, no provider fetches, no filesystem writes.
- Snapshot structs are cross-surface contracts; field changes affect CLI, Tauri, frontend, and exported reports together.
- Preserve date semantics explicitly: `report_date` vs `regime_as_of_date` vs `latest_available_date`.
- Scoped markdown must label `Scope:` explicitly; consumers should never infer CN/HK from filename alone.
- Markdown/report wording should keep `Market Regime` (conclusion) separate from `Environment Layer` (decomposition).
- Markdown rendering should mirror UI semantics, not invent parallel terminology.

## ANTI-PATTERNS
- Do **not** pull `market-store` or provider logic into this crate.
- Do **not** hide expensive orchestration inside report rendering.
- Do **not** rename or repurpose snapshot fields casually; downstream consumers rely on them.
- Do **not** blur watchlist breadth proxy with true market breadth language.
- Do **not** let scoped report markdown omit scope labeling or show mixed-market breadth blocks.
- Do **not** collapse environment and breadth into contradictory terminology across markdown vs desktop UI.

## NOTES
- This crate is small but high-leverage: even tiny payload changes ripple across CLI, desktop, docs, and tests.
- If dashboard payload grows again, review duplication between markdown/report wording and frontend labels.
- Scoped report file names/types are decided upstream, but the payload must still carry `scope` so render output is self-describing.
- Report markdown now includes an `Environment Layer` section; keep tests aligned when payload fields change.

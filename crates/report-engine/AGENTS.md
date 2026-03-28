# REPORT-ENGINE KNOWLEDGE BASE

## OVERVIEW
Pure snapshot shaping and markdown rendering layer for dashboard/report outputs.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Snapshot contract | `src/lib.rs::DashboardSnapshot` | shared CLI/Tauri/frontend payload |
| Timing contract | `src/lib.rs::DashboardLoadMetrics` | dashboard stage timings |
| Legacy date collector | `collect_dashboard_dates` | old whole-set helper; prefer scoped store path upstream |
| Date-specific builder | `build_dashboard_snapshot_for_date` | current selected-date assembly path |
| Report rendering | `render_markdown_report` | markdown output contract |
| Health rendering | `render_data_health_report` | data-health markdown output |

## CONVENTIONS
- Keep this crate pure: no storage reads, no provider fetches, no filesystem writes.
- Snapshot structs are cross-surface contracts; field changes affect CLI, Tauri, frontend, and exported reports together.
- Preserve date semantics explicitly: `report_date` vs `regime_as_of_date` vs `latest_available_date`.
- Markdown rendering should mirror UI semantics, not invent parallel terminology.

## ANTI-PATTERNS
- Do **not** pull `market-store` or provider logic into this crate.
- Do **not** hide expensive orchestration inside report rendering.
- Do **not** rename or repurpose snapshot fields casually; downstream consumers rely on them.
- Do **not** blur watchlist breadth proxy with true market breadth language.

## NOTES
- This crate is small but high-leverage: even tiny payload changes ripple across CLI, desktop, docs, and tests.
- If dashboard payload grows again, review duplication between markdown/report wording and frontend labels.

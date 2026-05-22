# review-dataflow-init-deep

## Goal

Perform an end-to-end correctness review of the quant dataflow, involve Oracle in the logic review, fix any concrete correctness bugs found, verify the pipeline live, then run `/init-deep` in update mode.

## Steps

1. Inspect code paths for ingestion, indicators, macro/regime, rotation, strategy, signal, report, desktop bundle, and health diagnostics.
2. Run parallel explore searches plus direct reads and live CLI checks to identify stale-stage or dataflow mismatch risks.
3. Capture at least one concrete failing or suspicious reproduction artifact before fixing anything (CLI output, table freshness mismatch, dashboard/report inconsistency).
4. Consult Oracle on end-to-end dataflow correctness and highest-risk logic points.
5. Apply the smallest concrete fix needed for any verified correctness issue.
6. Re-run the focused failing scenario first, then re-run end-to-end verification.
7. Run `/init-deep` update pass to refresh AGENTS hierarchy after the review/fixes.
8. Summarize Oracle findings, fixes, verification evidence, and remaining cautions.

## Non-goals

- No speculative architecture rewrite
- No git commit unless explicitly requested
- No cosmetic-only refactors without correctness impact

## Verification

- `cargo check --workspace`
- targeted `cargo test`
- `cargo run -p quant-cli -- pipeline-dates`
- `cargo run -p quant-cli -- dashboard-snapshot`
- `cargo run -p quant-cli -- export-report`
- `cargo run -p quant-cli -- check-data-health`

## Executable QA Scenarios

### Scenario 1: Stage freshness consistency
- Run `cargo run -p quant-cli -- pipeline-dates`
- Expect: `daily_bar`, `indicator_snapshot`, `rotation_rank`, `strategy_preference`, `signal_snapshot`, and `dashboard_available` show consistent latest dates after a successful full pipeline run.

### Scenario 2: Dashboard/report date consistency
- Run `cargo run -p quant-cli -- dashboard-snapshot`
- Run `cargo run -p quant-cli -- export-report`
- Expect: exported report date and snapshot `report_date` match the latest available dashboard date.

### Scenario 3: Health vs pipeline interpretation
- Run `cargo run -p quant-cli -- check-data-health`
- Compare with `pipeline-dates`
- Expect: symbol bar freshness can lead macro-source freshness, and any lag is explainable by source publication timing or identified pipeline bugs.

### Scenario 4: Bug-fix replay
- Re-run the exact CLI sequence that exposed the issue before the fix.
- Expect: the previously stale downstream stage advances, and the reproduced inconsistency disappears.

## Commit Strategy (if later requested)

- Separate correctness fixes from docs or AGENTS updates.
- Prefer one commit for backend logic fix, one commit for docs/examples, one commit for `/init-deep` hierarchy updates.

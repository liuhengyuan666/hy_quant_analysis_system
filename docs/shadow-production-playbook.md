# Shadow Production Playbook

> Current phase: `rv1_capability_consolidation`
> Observation window: original 90-day window completed 2026-09-05; continued observation remains active
> Goal: accumulate evidence before deciding whether any ADR is warranted for State / Signal / Regime / Execution layers.

## Daily cadence

Two daily actions must both run; one does not replace the other:

- Run `shadow-production/daily-log.ps1` with the appropriate phase (`-Phase A` = state layer only, `-Phase B` = + economic layer, `-Phase C` = + allocation suggestion). This step keeps `shadow-master.csv` current, which `weekly-review.ps1` requires. The script's original `A`/`B`/`C` gates were day-based on the 90-day window; continued observation still uses an existing phase, and `-Phase C` keeps every column the weekly report reads populated.
- Run `quant-cli research observe --scope global|cn|hk` (aggregates SRD / Stretch / Analytics / Data Health into the daily observation report)
  - `research observe` also maintains the deterministic per-symbol TASK-093 divergence ledger automatically; manual capture of these `StrongBuy + DE_RISK` cases is no longer needed. This ledger supplements rather than replaces `daily-log.ps1` / `shadow-master.csv`, which remains the broader per-day Shadow Production record consumed by `weekly-review.ps1`:
    - Predicate: the symbol's exact-date signal is `StrongBuy` AND the scope's exact-date strategy state is `DE_RISK`. A prior-day-only state never creates cases.
    - Case identity: `(scope, symbol, observation_date)`, stored at `workspace/divergence-ledger/{scope}/{symbol}/{YYYY-MM-DD}.json` (gitignored; scope is lowercased on disk).
    - Each record persists the full `SignalSnapshot` (signal attribution) plus the exact-date `StrategyStateSnapshot` (strategy-state facts).
    - T+20 / T+60 / T+120 outcomes mature independently from strictly-subsequent trading bars: `Pending` -> `Filled` when maturity facts exist. Missing or invalid observation bars remain `Pending` so a later backfill can repair them; only genuinely permanent structural failures may become `Unavailable`.
    - Repeated observe calls sweep every existing record in the scope and advance maturity; a case is written once (identical bytes never rewritten, conflicting facts preserved).
    - An explicit past `--date` (strictly before the scope's latest signal date) is recorded as `Reconstructed`; any other run (including the default latest-date run) is `Prospective`.
    - Classification starts `Unclassified`; assigning categories stays manual and is future TASK-100 work.

## Weekly cadence

- Run `quant-cli symbol-scoreboard`
- Run `quant-cli research analytics --scope global|cn|hk`
- Review the divergence ledger (`workspace/divergence-ledger/`) for accumulating patterns
- Check kill criteria (see below)

## Quarterly cadence

- Run `quant-cli research review --scope global|cn|hk --from <start> --to <end>`
- Aggregate SRD / Stretch / Analytics distributions
- Evaluate whether State Layer is too conservative

## Kill criteria — stop and file ADR review if any of the following triggers

1. Persistent StrongBuy + `DE_RISK` divergence with positive forward returns across multiple symbols.
2. `research review` shows statistically significant under-reaction in State Layer.
3. Data quality issues make the divergence ledger unreliable.

## Exit criteria

- 90-day observation period completed, **or**
- A kill criterion triggered and documented in a new ADR proposal.

## Notes

- Research Layer feature set is frozen during Shadow Production. No new Research CLI tools.
- All conclusions must be based on 1-day persistence; do not use 10-day persistence for decision-making.

# Shadow Production Playbook

> Current phase: `shadow_production_observation`
> Observation window: 90 days
> Goal: accumulate evidence before deciding whether any ADR is warranted for State / Signal / Regime / Execution layers.

## Daily cadence

- Run `quant-cli research srd --scope global|cn|hk`
- Run `quant-cli research stretch --scope global|cn|hk`
- Run `quant-cli symbol-diagnostics` for divergence candidates
- Record StrongBuy signal + `DE_RISK` state combinations in the divergence log:
  - Symbol, Date, Signal Score, Attribution Breakdown, State
  - T+20 / T+60 / T+120 forward returns (measured later)

## Weekly cadence

- Run `quant-cli symbol-scoreboard`
- Run `quant-cli research analytics --scope global|cn|hk`
- Review divergence log for accumulating patterns
- Check kill criteria (see below)

## Quarterly cadence

- Run `quant-cli research review --scope global|cn|hk --from <start> --to <end>`
- Aggregate SRD / Stretch / Analytics distributions
- Evaluate whether State Layer is too conservative

## Kill criteria — stop and file ADR review if any of the following triggers

1. Persistent StrongBuy + `DE_RISK` divergence with positive forward returns across multiple symbols.
2. `research review` shows statistically significant under-reaction in State Layer.
3. Data quality issues make divergence log unreliable.

## Exit criteria

- 90-day observation period completed, **or**
- A kill criterion triggered and documented in a new ADR proposal.

## Notes

- Research Layer feature set is frozen during Shadow Production. No new Research CLI tools.
- All conclusions must be based on 1-day persistence; do not use 10-day persistence for decision-making.

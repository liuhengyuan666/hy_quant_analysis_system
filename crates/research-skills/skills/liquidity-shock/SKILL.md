---
name: liquidity-shock
description: Analyze liquidity conditions and detect shocks in funding markets
version: "1.0.0"
author: "quant-team"

trigger:
  all: []
  any:
    - field: liquidity.pressure
      operator: "=="
      value: high
    - field: liquidity.pressure
      operator: "=="
      value: critical
    - field: breadth.breadth_pct
      operator: "<"
      value: 30
  none: []
  weight:
    liquidity: 0.9
    breadth: 0.7

inputs:
  - liquidity.pressure
  - liquidity.spread
  - breadth.breadth_pct
  - breadth.breadth_delta
  - regime.state
  - regime.confidence

outputs:
  - liquidity_state
  - shock_detected
  - funding_stress
  - recommendation
  - severity
  - confidence

dependencies: []

confidence_model:
  base: 0.7
  factors:
    - data_freshness
    - signal_convergence
    - breadth_alignment

failure_modes:
  - condition: "liquidity.pressure is null"
    action: "abort"
    message: "No liquidity pressure data available, cannot assess shock"
  - condition: "liquidity.spread is null"
    action: "reduce_confidence"
    message: "Spread data missing, funding stress assessment degraded"
  - condition: "breadth.breadth_pct is null"
    action: "reduce_confidence"
    message: "Breadth data missing, shock confirmation degraded"

evaluation_metrics:
  - detection_rate
  - false_positive_rate
  - severity_accuracy

output_schema: schema.json
priority: high
---

## Overview

Analyze liquidity conditions and detect shocks in funding markets by combining funding stress signals, spread widening patterns, and breadth deterioration. A liquidity shock occurs when funding conditions tighten abruptly while market breadth collapses simultaneously.

The analysis follows a structured reasoning process:
1. Assess funding stress from pressure and spread signals
2. Confirm market breadth deterioration
3. Detect shock when funding stress and breadth weakness converge
4. Determine severity and generate actionable recommendation

## Reasoning Graph

```yaml
steps:
  funding_analysis:
    inputs:
      - liquidity.pressure
      - liquidity.spread
    checks:
      - is_funding_stressed
      - is_spread_widening
    outputs:
      - funding_state
      - funding_signal

  breadth_confirmation:
    inputs:
      - breadth.breadth_pct
      - breadth.breadth_delta
      - regime.current
    checks:
      - is_breadth_weakening
      - is_regime_deteriorating
    outputs:
      - breadth_confirmed
      - breadth_severity

  shock_detection:
    inputs:
      - funding_state
      - funding_signal
      - breadth_confirmed
      - breadth_severity
    checks:
      - is_shock_confirmed
      - calculate_severity
    states:
      - normal
      - warning
      - shock_active
      - shock_critical
    transitions:
      - from: normal
        to: warning
        condition: "funding_state == stressed && breadth_confirmed == true"
      - from: warning
        to: shock_active
        condition: "funding_state == stressed && breadth_severity == high"
      - from: shock_active
        to: shock_critical
        condition: "funding_signal == critical && breadth_severity == extreme"
      - from: shock_active
        to: warning
        condition: "funding_state == moderating && breadth_confirmed == false"
      - from: shock_critical
        to: shock_active
        condition: "funding_state == stressed && breadth_severity != extreme"
    outputs:
      - shock_detected
      - severity
      - liquidity_state
```

## Execution Instructions

1. Retrieve `liquidity.pressure` and `liquidity.spread` from the current analysis scope
2. Run `funding_analysis` to determine baseline funding conditions
3. Retrieve `breadth.breadth_pct`, `breadth.breadth_delta`, and current `regime.current`
4. Run `breadth_confirmation` to assess market breadth alignment
5. Feed both results into `shock_detection` for final state classification
6. Populate confidence by evaluating data freshness, signal convergence, and breadth alignment
7. Generate recommendation based on detected severity:
   - `normal` → maintain positions
   - `warning` → reduce marginal exposure, tighten stops
   - `shock_active` → reduce exposure significantly, raise cash
   - `shock_critical` → defensive posture, prioritize capital preservation

## Output Format

```json
{
  "liquidity_state": "shock_active",
  "shock_detected": true,
  "funding_stress": "high",
  "recommendation": "reduce_exposure",
  "severity": "high",
  "confidence": 0.85,
  "reasoning_trace": {
    "funding_analysis": {
      "input": "pressure=critical, spread=widening",
      "funding_state": "stressed",
      "funding_signal": "critical",
      "conclusion": "severe funding stress detected"
    },
    "breadth_confirmation": {
      "input": "breadth_pct=22, breadth_delta=-18, regime=risk_off",
      "breadth_confirmed": true,
      "breadth_severity": "extreme",
      "conclusion": "breadth collapse confirms liquidity event"
    },
    "shock_detection": {
      "from": "warning",
      "to": "shock_active",
      "condition": "funding_state == stressed && breadth_severity == high",
      "conclusion": "liquidity shock active"
    }
  }
}
```

## Error Handling

When data is stale or missing, degradation rules apply:

| Condition | Action | Result |
|-----------|--------|--------|
| `liquidity.pressure` is null | Abort analysis | Cannot proceed |
| `liquidity.spread` is null | Reduce confidence by 0.3 | Funding stress flagged as uncertain |
| `breadth.breadth_pct` is null | Reduce confidence by 0.2 | Shock detection unconfirmed |
| Data staleness > 3 days | Reduce confidence by 0.1 per day | Staleness penalty applied |
| `regime.confidence` < 0.5 | Reduce confidence by 0.2 | Regime context unreliable |

If both `liquidity.pressure` and `liquidity.spread` are missing, the analysis aborts with an error indicating insufficient data.

## Dependencies

- **market-regime-reasoning**: Regime state provides the macro context for liquidity shock assessment. A risk-off or de-risk regime amplifies the severity of funding stress signals.
- **breadth-analysis** (future): Dedicated breadth analysis would provide richer breadth condition inputs (advance/decline ratios, participation rates) beyond the basic breadth_pct and breadth_delta used here.

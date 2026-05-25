---
name: factor-composite
description: Composite multi-factor signal with risk parity weighting
version: "1.0.0"
author: "quant-team"

trigger:
  all:
    - field: regime.confidence
      operator: ">="
      value: 0.6
  any:
    - field: rotation.momentum_factor
      operator: ">"
      value: 0.5
    - field: rotation.value_factor
      operator: ">"
      value: 0.5
    - field: rotation.quality_factor
      operator: ">"
      value: 0.5
    - field: rotation.crowding_factor
      operator: "<"
      value: 0.5
  none:
    - field: breadth.breadth_pct
      operator: "<"
      value: 20
  weight:
    momentum: 0.35
    value: 0.25
    quality: 0.25
    crowding: -0.15

inputs:
  - regime.current
  - regime.confidence
  - rotation.momentum_factor
  - rotation.value_factor
  - rotation.quality_factor
  - rotation.crowding_factor
  - breadth.breadth_pct

outputs:
  - composite_signal
  - factor_weights
  - factor_contributions
  - risk_parity_weight
  - recommendation

dependencies: []

confidence_model:
  base: 0.6
  factors:
    - data_freshness
    - factor_agreement
    - regime_alignment
    - breadth_environment

failure_modes:
  - condition: "any_factor_null"
    action: "reduce_confidence"
    message: "One or more factors missing, composite signal may be unreliable"
  - condition: "all_factors_stale"
    action: "abort"
    message: "All factor data stale, cannot compute composite signal"
  - condition: "regime_confidence_below_0.5"
    action: "reduce_confidence"
    message: "Regime assessment uncertain, factor weights may be misaligned"

evaluation_metrics:
  - signal_accuracy
  - factor_contribution

output_schema: schema.json
priority: high
---

## Overview

Composite multi-factor signal analysis that combines momentum, value, quality, and crowding factors
using risk parity weighting. The analysis normalizes raw factor scores, computes inverse-volatility
weights, and produces a unified composite signal with factor-level contribution breakdowns.

The analysis follows a structured reasoning process:
1. Normalize raw factor scores to a common scale
2. Calculate risk parity weights based on factor volatility
3. Blend weighted factors into a composite signal
4. Decompose contributions by factor
5. Generate an actionable recommendation aligned with market regime

## Reasoning Graph

```yaml
steps:
  factor_normalization:
    inputs:
      - rotation.momentum_factor
      - rotation.value_factor
      - rotation.quality_factor
      - rotation.crowding_factor
    checks:
      - are_factors_available
    outputs:
      - normalized_factors

  weight_calculation:
    inputs:
      - normalized_factors
    checks:
      - is_weight_valid
    outputs:
      - factor_weights
      - risk_parity_weight

  composite_signal:
    inputs:
      - factor_weights
      - normalized_factors
    checks:
      - is_signal_strong
    outputs:
      - composite_signal
      - factor_contributions
```

## Execution Instructions

1. **Load factor inputs**: Retrieve `momentum_factor`, `value_factor`, `quality_factor`, and
   `crowding_factor` from the rotation engine output for the target analysis date.

2. **Validate factor availability**: Check that all four factor values are non-null. If any factor
   is missing, apply the `any_factor_null` failure mode — reduce confidence but proceed with
   available factors by redistributing weights proportionally.

3. **Normalize factors**: Transform each raw factor score to a z-score or min-max normalized
   value on a 0–1 scale. Crowding is inverted so that lower crowding (less crowded) maps to
   higher normalized scores.

4. **Compute risk parity weights**:
   - Calculate the inverse of each factor's historical volatility (trailing 60-period std).
   - Normalize weights so they sum to 1.0.
   - Weight cap: no single factor exceeds 40% of total weight.

5. **Calculate composite signal**:
   ```
   composite_signal = Σ (factor_weights[i] × normalized_factors[i])
   ```
   Positive composite → bullish tilt; negative or near-zero → neutral/defensive.

6. **Decompose contributions**: Compute each factor's marginal contribution as
   `factor_weights[i] × normalized_factors[i]`, expressed as both absolute and percentage
   of composite.

7. **Generate recommendation**: Map the composite signal to a recommendation aligned with
   the current regime (`risk_on`, `neutral`, `risk_off`, `de_risk`).

## Output Format

The analysis produces a JSON object compliant with the `schema.json` output schema.
Key fields:

| Field | Type | Description |
|-------|------|-------------|
| `composite_signal` | number | Normalized composite score (-1 to +1). Positive = bullish, negative = defensive. |
| `factor_weights` | object | Risk parity weights per factor (momentum, value, quality, crowding), summing to 1.0. |
| `factor_contributions` | object | Marginal contribution of each factor to the composite signal. |
| `risk_parity_weight` | number | The risk parity scaling factor applied, derived from inverse volatility. |
| `recommendation` | string | Actionable signal: `overweight_risk`, `neutral`, `defensive`, `high_quality`, `avoid_crowded`. |
| `confidence` | number | Confidence score (0–1), adjusted by data freshness, factor agreement, and regime alignment. |

Example:

```json
{
  "composite_signal": 0.62,
  "factor_weights": {
    "momentum": 0.30,
    "value": 0.28,
    "quality": 0.27,
    "crowding": 0.15
  },
  "factor_contributions": {
    "momentum": 0.21,
    "value": 0.17,
    "quality": 0.16,
    "crowding": 0.08
  },
  "risk_parity_weight": 0.85,
  "recommendation": "overweight_risk",
  "confidence": 0.78
}
```

## Error Handling

- **Missing factors (any_factor_null)**: Flag the missing factor(s), redistribute weights
  across available factors, reduce confidence by 0.15 per missing factor, and include a
  `warnings` array in the output noting which factors are absent.
- **All factors stale**: Abort computation and return an error state with `composite_signal: null`
  and `confidence: 0`. The calling layer should interpret this as "insufficient data".
- **Regime confidence below 0.5**: Apply a regime-alignment penalty to factor weights,
  shifting weight toward quality and away from momentum. Reduce overall confidence by 0.1.
- **Breadth below 20%**: This is a hard trigger in the `trigger.none` list — the skill
  should not fire when breadth is this extreme, as the composite signal is unreliable in
  panic conditions.

## Dependencies

- **market-regime-reasoning**: Provides `regime.current` and `regime.confidence` used for
  trigger gating and recommendation alignment. The composite signal should be interpreted
  in the context of the current regime.
- **rotation-engine**: Source of raw factor scores (`momentum_factor`, `value_factor`,
  `quality_factor`, `crowding_factor`) consumed as inputs.
- **breadth analysis**: Provides `breadth.breadth_pct` used as a trigger guard to prevent
  the composite skill from firing in extreme breadth conditions.

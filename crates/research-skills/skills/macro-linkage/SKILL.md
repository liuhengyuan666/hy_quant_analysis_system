---
name: macro-linkage
description: Analyze overseas macro linkages including China-US spread, USD index, and foreign capital flows
version: "1.0.0"
author: "quant-team"

trigger:
  all: []
  any:
    - field: regime.macro_stale_days
      operator: ">"
      value: 3
    - field: breadth.breadth_pct
      operator: "<"
      value: 50
    - field: liquidity.pressure
      operator: "=="
      value: high
  none: []
  weight:
    regime: 0.85
    flow: 0.90

inputs:
  - regime.current
  - regime.confidence
  - regime.macro_stale_days
  - macro.spread_10y
  - macro.dxy_index
  - macro.foreign_flow
  - macro.vix

outputs:
  - linkage_signal
  - spread_analysis
  - dxy_analysis
  - flow_analysis
  - recommendation

dependencies: []

confidence_model:
  base: 0.7
  factors:
    - data_freshness
    - signal_alignment
    - linkage_strength

failure_modes:
  - condition: "macro_stale_days > 5"
    action: "reduce_confidence"
    message: "Macro data severely stale, linkage analysis may be unreliable"
  - condition: "macro.spread_10y is null or macro.dxy_index is null"
    action: "skip_linkage"
    message: "Core linkage inputs missing, cannot assess overseas macro linkage"

evaluation_metrics:
  - signal_accuracy
  - correlation_strength

output_schema: schema.json
priority: high
---

## Overview

Analyze overseas macro linkages by examining the China-US 10-year yield spread,
USD index (DXY), foreign capital flows, and VIX risk appetite indicators.
Detect whether these external factors are aligned in direction and provide
actionable linkage signals for the domestic market.

The analysis follows a structured reasoning process:
1. Assess China-US spread dynamics and USD strength
2. Evaluate foreign capital flow direction and VIX risk appetite
3. Detect alignment across external linkage factors
4. Generate a composite linkage signal and recommendation

## Reasoning Graph

```yaml
steps:
  spread_analysis:
    inputs:
      - macro.spread_10y
      - macro.dxy_index
    checks:
      - is_spread_widening
      - is_dxy_strengthening
    outputs:
      - spread_signal
      - dxy_signal

  flow_analysis:
    inputs:
      - macro.foreign_flow
      - macro.vix
    checks:
      - is_flow_positive
      - is_vix_elevated
    outputs:
      - flow_signal
      - vix_signal

  linkage_detection:
    inputs:
      - spread_signal
      - dxy_signal
      - flow_signal
    checks:
      - is_linkage_aligned
    outputs:
      - linkage_direction
      - linkage_strength
```

## Execution Instructions

To execute the macro-linkage analysis:

1. **Collect inputs**: Retrieve the latest macro data including `spread_10y`,
   `dxy_index`, `foreign_flow`, and `vix` from the macro snapshot.
2. **Run spread_analysis**: Compare the China-US 10-year yield spread against
   recent thresholds. Check if DXY is strengthening or weakening.
3. **Run flow_analysis**: Evaluate foreign capital net flow direction and
   VIX level to gauge risk appetite.
4. **Run linkage_detection**: Determine if spread, DXY, and flow signals
   are aligned. Aligned signals carry higher confidence.
5. **Assemble output**: Combine all signals into the final linkage assessment
   with confidence scoring.

## Output Format

```json
{
  "linkage_signal": "inflow_aligned",
  "spread_analysis": {
    "spread_signal": "widening",
    "spread_value": 1.25,
    "assessment": "China-US spread widening favors foreign inflow"
  },
  "dxy_analysis": {
    "dxy_signal": "weakening",
    "dxy_value": 101.5,
    "assessment": "DXY weakening supports EM capital inflow"
  },
  "flow_analysis": {
    "flow_signal": "positive",
    "flow_value": 5.2,
    "assessment": "Foreign flow positive, northbound capital returning"
  },
  "recommendation": "favorable_external",
  "confidence": 0.85,
  "reasoning_trace": {
    "spread_analysis": {
      "input": "spread_10y=1.25, dxy_index=101.5",
      "conclusion": "spread widening, DXY weakening - favorable for EM"
    },
    "flow_analysis": {
      "input": "foreign_flow=5.2, vix=16.3",
      "conclusion": "flow positive, VIX moderate"
    },
    "linkage_detection": {
      "direction": "inflow_aligned",
      "strength": "moderate",
      "condition": "spread_widening && dxy_weakening && flow_positive"
    }
  }
}
```

## Error Handling

When data is stale or missing:

- **macro_stale_days > 5**: Reduce confidence significantly. Mark recommendation
  as `data_stale` and warn that linkage assessment may be unreliable.
- **spread_10y or dxy_index is null**: Abort linkage detection. Return
  `linkage_signal: "insufficient_data"` with a clear error message.
- **foreign_flow is null**: Skip flow analysis. Run partial linkage detection
  using only spread and DXY signals, with reduced confidence.
- **VIX is null**: Substitute with an assumption of moderate volatility (VIX ~20).
  Flag this in the reasoning trace.

## Dependencies

- **market-regime-reasoning**: The macro-linkage skill is typically invoked
  after regime assessment to add an external-factor overlay. Regime state
  provides context for interpreting linkage direction.
- **liquidity-shock**: Foreign flow dynamics can interact with liquidity
  conditions. Cross-reference when flow signals conflict with liquidity pressure.

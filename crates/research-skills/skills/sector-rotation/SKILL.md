---
name: sector-rotation
description: Detect sector rotation patterns and identify momentum/value/quality/crowding factors
version: "1.0.0"
author: "quant-team"

trigger:
  all: []
  any:
    - field: rotation.momentum_factor
      operator: ">"
      value: 0.6
    - field: rotation.value_factor
      operator: ">"
      value: 0.5
    - field: regime.current
      operator: "=="
      value: "risk_on"
    - field: rotation.crowding_factor
      operator: ">"
      value: 0.7
  none: []
  weight:
    rotation: 0.85
    regime: 0.7

inputs:
  - rotation.top_sectors
  - rotation.bottom_sectors
  - rotation.momentum_factor
  - rotation.value_factor
  - rotation.quality_factor
  - rotation.crowding_factor
  - regime.current
  - regime.confidence

outputs:
  - rotation_detected
  - rotation_type
  - leading_sectors
  - lagging_sectors
  - factor_analysis
  - recommendation

dependencies: []

confidence_model:
  base: 0.65
  factors:
    - data_freshness
    - factor_alignment
    - momentum_strength
    - regime_compatibility

failure_modes:
  - condition: "rotation.top_sectors is empty"
    action: "reduce_confidence"
    message: "No top sector data available, rotation detection unreliable"
  - condition: "rotation.momentum_factor is null"
    action: "reduce_confidence"
    message: "Momentum factor missing, cannot assess momentum-driven rotation"
  - condition: "regime.confidence < 0.4"
    action: "reduce_confidence"
    message: "Regime confidence too low, rotation signal may be noise"

evaluation_metrics:
  - detection_rate
  - false_positive_rate

output_schema: schema.json
priority: high
---

## Overview

Detect sector rotation patterns by analyzing momentum, value, quality, and crowding factor dynamics across sectors. Identify whether the market is rotating into or out of specific sectors and determine the dominant factor style driving the rotation.

The analysis follows a structured reasoning process:
1. Analyze momentum signals across top and bottom sectors
2. Evaluate factor alignment (value, quality, crowding)
3. Detect rotation type and determine confidence
4. Identify leading and lagging sectors
5. Generate actionable sector allocation recommendation

## Reasoning Graph

```yaml
steps:
  momentum_analysis:
    inputs:
      - rotation.momentum_factor
      - rotation.top_sectors
    checks:
      - is_momentum_strong
    outputs:
      - momentum_signal
      - leading_sectors

  factor_analysis:
    inputs:
      - rotation.value_factor
      - rotation.quality_factor
      - rotation.crowding_factor
    checks:
      - is_factor_aligned
    outputs:
      - factor_signal
      - preferred_style

  rotation_detection:
    inputs:
      - momentum_signal
      - factor_signal
    types:
      - momentum_rotation
      - value_rotation
      - quality_rotation
      - defensive_rotation
      - no_rotation
    checks:
      - is_rotation_detected
    transitions:
      - from: momentum_rotation
        to: value_rotation
        condition: "momentum_factor < 0.4 && value_factor > 0.7"
      - from: value_rotation
        to: momentum_rotation
        condition: "momentum_factor > 0.7 && value_factor < 0.4"
      - from: momentum_rotation
        to: defensive_rotation
        condition: "crowding_factor > 0.8 && quality_factor < 0.3"
      - from: value_rotation
        to: quality_rotation
        condition: "quality_factor > 0.7 && value_factor < 0.4"
      - from: quality_rotation
        to: no_rotation
        condition: "momentum_factor < 0.3 && value_factor < 0.3 && quality_factor < 0.3"
    outputs:
      - rotation_type
      - rotation_confidence

  sector_allocation:
    inputs:
      - rotation_type
      - leading_sectors
      - rotation.bottom_sectors
    checks:
      - calculate_sector_spread
      - identify_lagging_sectors
    outputs:
      - lagging_sectors
      - recommendation
```

## Execution Instructions

1. **Load Input Data**: Gather `rotation` and `regime` snapshot data for the analysis date.
2. **Momentum Analysis**: Evaluate whether momentum is strong enough to drive rotation by checking `rotation.momentum_factor` against threshold (0.6). Identify `leading_sectors` from `rotation.top_sectors`.
3. **Factor Analysis**: Assess alignment across value, quality, and crowding factors. Determine the dominant factor style (`preferred_style`) by comparing relative factor strengths.
4. **Rotation Detection**: Combine momentum and factor signals to classify the rotation type. Apply transition rules to detect regime shifts. Calculate `rotation_confidence` from factor alignment and data quality.
5. **Sector Allocation**: Identify lagging sectors from `rotation.bottom_sectors`. Generate a recommendation based on rotation type and sector spread.
6. **Output**: Emit structured JSON conforming to `schema.json`.

## Output Format

```json
{
  "rotation_detected": true,
  "rotation_type": "momentum_rotation",
  "leading_sectors": [
    {"sector": "technology", "score": 0.92},
    {"sector": "consumer_discretionary", "score": 0.87}
  ],
  "lagging_sectors": [
    {"sector": "utilities", "score": 0.12},
    {"sector": "real_estate", "score": 0.18}
  ],
  "factor_analysis": {
    "dominant_style": "momentum",
    "momentum_strength": "strong",
    "value_alignment": "weak",
    "quality_alignment": "neutral",
    "crowding_alert": false
  },
  "recommendation": {
    "action": "rotate_into",
    "target_sectors": ["technology", "consumer_discretionary"],
    "avoid_sectors": ["utilities", "real_estate"],
    "rationale": "Strong momentum signal with high factor alignment; rotate into high-momentum growth sectors"
  },
  "confidence": 0.82,
  "reasoning_trace": {
    "momentum_analysis": {
      "input": "momentum_factor=0.85, top_sectors=technology,consumer_discretionary",
      "conclusion": "momentum is strong, driving sector rotation"
    },
    "factor_analysis": {
      "input": "value_factor=0.25, quality_factor=0.55, crowding_factor=0.30",
      "conclusion": "momentum factor dominant, value and crowding not a concern"
    },
    "rotation_detection": {
      "type": "momentum_rotation",
      "condition": "momentum_factor > 0.7, value_factor < 0.4"
    }
  }
}
```

## Error Handling

| Condition | Action | Message |
|-----------|--------|---------|
| `rotation.top_sectors` is empty | `reduce_confidence` | No top sector data available, rotation detection unreliable |
| `rotation.momentum_factor` is null | `reduce_confidence` | Momentum factor missing, cannot assess momentum-driven rotation |
| `regime.confidence < 0.4` | `reduce_confidence` | Regime confidence too low, rotation signal may be noise |
| `rotation.top_sectors` and `rotation.bottom_sectors` both empty | `fail_gracefully` | Insufficient data for sector rotation analysis |
| All factor values are null | `fail_gracefully` | No factor data available, rotation analysis impossible |

When data is stale or missing:
- Emit `rotation_detected: false` with reduced confidence.
- Include a `data_quality_warning` in the output explaining which inputs are stale or missing.
- Never fabricate sector scores or rotation signals from insufficient data.

## Dependencies

This skill has no hard dependencies on other research skills. However, it is commonly used alongside:

- **market-regime-reasoning**: Provides `regime.current` and `regime.confidence` context. Rotation signals are more reliable when the regime is `risk_on` or `neutral`.
- **liquidity-shock**: Liquidity conditions can override rotation signals (e.g., a liquidity shock may halt momentum rotation regardless of sector strength).

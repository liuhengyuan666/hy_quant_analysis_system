---
name: market-regime-reasoning
description: Analyze market regime state and detect transitions between risk_on, neutral, risk_off, and de_risk
version: "1.0.0"
author: "quant-team"

trigger:
  all: []
  any:
    - field: regime.macro_stale_days
      operator: ">"
      value: 2
    - field: breadth.breadth_pct
      operator: "<"
      value: 40
  none: []
  weight:
    regime: 0.9
    risk: 0.8

inputs:
  - regime.current
  - regime.confidence
  - regime.macro_stale_days
  - breadth.breadth_pct
  - breadth.breadth_delta
  - liquidity.pressure
  - market.current_state

outputs:
  - regime_state
  - confidence
  - transition_detected
  - key_drivers
  - risk_assessment

dependencies: []

confidence_model:
  base: 0.7
  factors:
    - data_freshness
    - signal_strength
    - breadth_clarity

failure_modes:
  - condition: "macro_stale_days > 5"
    action: "reduce_confidence"
    message: "Macro data severely stale, regime assessment may be unreliable"
  - condition: "breadth_pct < 20"
    action: "flag_extreme"
    message: "Extreme breadth collapse detected"

evaluation_metrics:
  - regime_accuracy
  - transition_detection_rate
  - false_positive_rate

output_schema: schema.json
priority: high
---

## Overview

Analyze the current market regime by combining macro indicators, breadth conditions, and liquidity state. Detect regime transitions and provide actionable risk assessment.

The analysis follows a structured reasoning process:
1. Assess current regime state
2. Evaluate transition signals
3. Determine confidence level
4. Identify key drivers
5. Generate risk assessment

## Reasoning Graph

```yaml
steps:
  regime_assessment:
    inputs:
      - regime.current
      - regime.confidence
      - regime.macro_stale_days
    checks:
      - is_regime_stale
      - is_confidence_sufficient
    outputs:
      - regime_state
      - regime_confidence

  breadth_analysis:
    inputs:
      - breadth.breadth_pct
      - breadth.breadth_delta
    checks:
      - is_breadth_collapsed
      - is_breadth_weakening
    outputs:
      - breadth_condition
      - breadth_signal

  transition_detection:
    inputs:
      - regime_state
      - breadth_condition
      - liquidity.pressure
    states:
      - risk_on
      - neutral
      - risk_off
      - de_risk
    transitions:
      - from: risk_on
        to: de_risk
        condition: "breadth_pct < 30 && liquidity_pressure == high"
      - from: risk_on
        to: neutral
        condition: "breadth_pct < 50 || macro_stale_days > 3"
      - from: neutral
        to: risk_off
        condition: "breadth_pct < 30 || liquidity_pressure == critical"
      - from: neutral
        to: risk_on
        condition: "breadth_pct > 60 && liquidity_pressure == low"
      - from: risk_off
        to: neutral
        condition: "breadth_pct > 40 && liquidity_pressure != critical"
      - from: de_risk
        to: risk_on
        condition: "breadth_pct > 60 && macro_stale_days <= 2"
    outputs:
      - transition_detected
      - new_state

  risk_assessment:
    inputs:
      - regime_state
      - transition_detected
      - breadth_signal
    checks:
      - calculate_risk_level
      - identify_risk_factors
    outputs:
      - risk_level
      - risk_factors
```

## Output Format

```json
{
  "regime_state": "risk_off",
  "confidence": 0.85,
  "transition_detected": true,
  "previous_state": "neutral",
  "key_drivers": [
    "breadth_collapse",
    "liquidity_fragility"
  ],
  "risk_assessment": {
    "level": "high",
    "factors": [
      "breadth_below_30",
      "liquidity_critical"
    ],
    "recommendation": "reduce_exposure"
  },
  "reasoning_trace": {
    "regime_assessment": {
      "input": "regime.current=risk_off, confidence=0.8",
      "conclusion": "regime is valid"
    },
    "breadth_analysis": {
      "input": "breadth_pct=25, delta=-15",
      "conclusion": "breadth collapsed"
    },
    "transition_detection": {
      "from": "neutral",
      "to": "risk_off",
      "condition": "breadth_pct < 30"
    }
  }
}
```

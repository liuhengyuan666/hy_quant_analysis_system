---
name: volatility-tail
description: Analyze volatility regime and tail risk using VIX, skewness, kurtosis, and EVT methods
version: "1.0.0"
author: "quant-team"

trigger:
  all: []
  any:
    - field: macro.vix
      operator: ">"
      value: 25
    - field: risk.skewness
      operator: "<"
      value: -0.5
    - field: risk.tail_index
      operator: "<"
      value: 2.5
  none: []
  weight:
    regime: 0.85
    risk: 0.9

inputs:
  - regime.current
  - regime.confidence
  - macro.vix
  - risk.skewness
  - risk.kurtosis
  - risk.tail_index

outputs:
  - volatility_regime
  - tail_risk_level
  - vix_analysis
  - tail_analysis
  - recommendation

dependencies: []

confidence_model:
  base: 0.7
  factors:
    - data_freshness
    - vix_signal_clarity
    - tail_metric_consistency

failure_modes:
  - condition: "macro.vix is null or stale > 3 days"
    action: "reduce_confidence"
    message: "VIX data missing or stale, volatility assessment may be unreliable"
  - condition: "risk.skewness is null"
    action: "reduce_confidence"
    message: "Skewness data missing, tail asymmetry cannot be assessed"
  - condition: "risk.tail_index is null"
    action: "fallback_kurtosis"
    message: "Tail index unavailable, falling back to kurtosis-only tail assessment"
  - condition: "regime.confidence < 0.4"
    action: "flag_uncertain"
    message: "Regime confidence too low, volatility context may be ambiguous"

evaluation_metrics:
  - risk_accuracy
  - tail_detection_rate

output_schema: schema.json
priority: high
---

## Overview

Analyze the current volatility regime and tail risk exposure by combining VIX-based volatility signals with distributional risk metrics (skewness, kurtosis, tail index via EVT). The analysis produces actionable risk warnings when markets exhibit elevated volatility and/or fat-tail behavior.

The analysis follows a structured reasoning process:
1. Assess volatility regime using VIX and current market regime context
2. Evaluate tail risk using distribution shape metrics
3. Integrate both assessments into a consolidated risk view
4. Generate confidence-weighted recommendations

## Reasoning Graph

```yaml
steps:
  volatility_analysis:
    inputs:
      - macro.vix
      - regime.current
    checks:
      - is_vix_elevated
      - is_vol_regime_high
    outputs:
      - volatility_regime
      - vix_signal

  tail_analysis:
    inputs:
      - risk.skewness
      - risk.kurtosis
      - risk.tail_index
    checks:
      - is_tail_fat
      - is_skew_negative
    outputs:
      - tail_risk_level
      - tail_signal

  risk_assessment:
    inputs:
      - volatility_regime
      - tail_risk_level
    checks:
      - is_risk_elevated
    outputs:
      - overall_risk
      - recommendation
```

## Execution Instructions

1. **Retrieve inputs**: Fetch `macro.vix`, `regime.current`, `regime.confidence`, `risk.skewness`, `risk.kurtosis`, and `risk.tail_index` from the current analysis snapshot.
2. **Run volatility_analysis step**: Compare VIX against historical thresholds. If VIX is elevated (>25) and regime is risk_off or de_risk, classify volatility_regime as "high". Otherwise, classify as "normal" or "low".
3. **Run tail_analysis step**: Evaluate skewness (negative = downside tail risk), kurtosis (>3 = fat tails vs normal distribution), and tail_index (EVT estimate; <2 = heavy tailed). Combine into tail_risk_level.
4. **Run risk_assessment step**: Merge volatility_regime and tail_risk_level into an overall risk assessment. Generate a recommendation based on the combined severity.
5. **Apply confidence model**: Adjust base confidence by data freshness, signal clarity, and metric consistency.
6. **Return structured output** conforming to `schema.json`.

## Output Format

```json
{
  "volatility_regime": "high",
  "tail_risk_level": "elevated",
  "vix_analysis": {
    "current_vix": 28.5,
    "regime": "high",
    "signal": "elevated_volatility",
    "percentile_rank": 0.85
  },
  "tail_analysis": {
    "skewness": -0.72,
    "kurtosis": 4.3,
    "tail_index": 2.1,
    "tail_risk_level": "elevated",
    "signal": "fat_tail_negative_skew"
  },
  "recommendation": "reduce_exposure",
  "confidence": 0.82,
  "reasoning_trace": {
    "volatility_analysis": {
      "input": "vix=28.5, regime=risk_off",
      "conclusion": "volatility regime is high"
    },
    "tail_analysis": {
      "input": "skew=-0.72, kurt=4.3, tail_index=2.1",
      "conclusion": "elevated tail risk with negative skew"
    },
    "risk_assessment": {
      "input": "vol_regime=high, tail_risk=elevated",
      "conclusion": "overall risk elevated, recommend reducing exposure"
    }
  }
}
```

## Error Handling

- **Stale VIX data** (stale > 3 days): Reduce confidence by 0.2 and flag the volatility assessment. If VIX is unavailable entirely, fall back to using only regime-based volatility inference.
- **Missing skewness**: Tail asymmetry cannot be evaluated. Reduce confidence by 0.15 and rely solely on kurtosis and tail_index for tail assessment.
- **Missing tail_index**: Fall back to kurtosis-only tail detection. Flag the tail analysis as "partial" and reduce confidence by 0.1.
- **Low regime confidence** (<0.4): The volatility context from regime is unreliable. Reduce overall confidence by 0.2 and note the ambiguity.

## Dependencies

- **market-regime-reasoning**: Supplies `regime.current` and `regime.confidence` as context for volatility assessment. A risk_off or de_risk regime amplifies the significance of elevated VIX and tail risk signals.
- **macro-linkage**: Provides `macro.vix` as input. Stale or missing macro data degrades volatility analysis confidence.
- **liquidity-shock**: Tail risk events often correlate with liquidity shocks. When both skills signal elevated risk, the combined severity should be escalated.

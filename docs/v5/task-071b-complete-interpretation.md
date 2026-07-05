# TASK-071B: State Lead/Lag Analysis — Complete Interpretation

**Status:** Analysis Complete  
**Date:** 2026-06-07  
**Scope:** CN (000300) + HK (HSCEI), 2024-01-01 to 2026-03-16

---

## Executive Summary

**The regime is a STATE CLASSIFIER (descriptive), not a PREDICTOR (predictive).**

All three states identify conditions that are ALREADY happening, not conditions that WILL happen:

| State | What It Identifies | Timing Relative to Market Move |
|-------|-------------------|-------------------------------|
| **RiskOn** | "We're in a trend" | AFTER trend has started |
| **Neutral** | "Trend has paused" | AFTER rally has cooled |
| **RiskOff** | "We're in stress" | DURING the decline |

---

## Evidence

### RiskOn: "Trend Follower" — NOT "Trend Predictor"

**CN RiskOn (n=29 episodes, avg 4.0d):**
| Phase | Return | Pattern |
|-------|--------|---------|
| Before 20d | +1.25% | Gains already happening |
| During | +1.02% | Gains continue |
| After 20d | +0.64% | **Gains decelerate** |

**HK RiskOn (n=31 episodes, avg 3.8d):**
| Phase | Return | Pattern |
|-------|--------|---------|
| Before 20d | +4.36% | Strong gains already happening |
| During | +1.67% | Gains continue but slower |
| After 20d | +1.01% | **Gains decelerate** |

**Signature:** Before > During > After

This is the classic pattern of a **momentum signal that enters AFTER the move has started**.

---

### Neutral: "Consolidation Detector"

**CN Neutral (n=60 episodes, avg 3.1d):**
| Phase | Return | Pattern |
|-------|--------|---------|
| Before 60d | +4.16% | Strong rally precedes |
| During | +0.23% | **Flat / sideways** |
| After 20d | +0.53% | Modest rebound |

**HK Neutral (n=59 episodes, avg 3.1d):**
| Phase | Return | Pattern |
|-------|--------|---------|
| Before 60d | +6.55% | Strong rally precedes |
| During | +0.09% | **Flat / sideways** |
| After 20d | +1.12% | Modest rebound |

**Signature:** Strong Before → Flat During → Modest After

Neutral identifies **post-rally consolidation periods**.

---

### RiskOff: "Crisis Confirmer" — NOT "Crisis Predictor"

**CN RiskOff (n=39 episodes, avg 6.5d):**
| Phase | Return | Pattern |
|-------|--------|---------|
| Before 20d | +0.56% | Modest gains (not crash) |
| During | **-0.69%** | **DECLINE** |
| After 20d | +2.70% | Strong rebound |

**HK RiskOff (n=39 episodes, avg 6.6d):**
| Phase | Return | Pattern |
|-------|--------|---------|
| Before 20d | +0.83% | Modest gains (not crash) |
| During | **-0.58%** | **DECLINE** |
| After 20d | +3.16% | Strong rebound |

**Signature:** Modest Before → Negative During → Strong Rebound After

RiskOff identifies **uncertainty that is ALREADY happening**, not uncertainty that is coming.

---

## Three Explanations Revisited

### A. Mean Reversion

**Verdict:** ✅ Partially correct, but incomplete.

Evidence:
- RiskOff After 20d: +2.70% (CN), +3.16% (HK)
- This shows mean reversion AFTER RiskOff

But mean reversion doesn't explain:
- Why RiskOff During is negative (not positive)
- Why RiskOff doesn't appear BEFORE the decline

### B. State Lag

**Verdict:** ✅ **Strong evidence. Primary explanation.**

Evidence:
- RiskOff During = negative (market is declining WHEN RiskOff triggers)
- RiskOff Before = positive (market was fine BEFORE RiskOff)
- This is the signature of a LAGGING indicator

The regime is **describing current conditions**, not predicting future conditions.

### C. RiskOn Definition Wrong

**Verdict:** ⚠️ Partially correct, but not the root cause.

Evidence:
- RiskOn Before > During > After (deceleration pattern)
- This means RiskOn enters AFTER the best part of the trend

But this is actually **expected behavior for a momentum signal**:
- Momentum signals are SUPPOSED to enter after the move starts
- They capture the "middle" of the trend, not the beginning
- The question is whether this is by design or by accident

---

## The Fundamental Question

**Does the user want the regime to be predictive or descriptive?**

### If Predictive (Leading Indicators):

**Required changes:**
- RiskOff should trigger BEFORE decline (Before = negative)
- RiskOn should trigger BEFORE rally (Before = higher than During)
- This requires different features/horizons than current implementation

**Challenge:**
- Predictive regimes are much harder to build
- Higher false positive rates
- May conflict with ADR-061's descriptive semantic contract

### If Descriptive (Current State):

**Current behavior is actually reasonable:**
- RiskOn: "We're in a trend now" ✅
- Neutral: "Trend has paused" ✅
- RiskOff: "Uncertainty is elevated now" ✅

**Value proposition:**
- State classification enables state-dependent strategy selection
- Not about timing the market, about adapting to current conditions
- Consistent with ADR-061's semantic contract

---

## Implications for TASK-004 (Threshold Calibration)

### If Descriptive Regime is Acceptable:

**Calibration direction:**
- RiskOn: May be too strict (only 29-31 episodes out of 128)
- RiskOff: May be too loose (39 episodes, catches too many minor dips)
- Neutral: Could be more selective

**But:** The current thresholds may already be reasonable for a descriptive regime.

### If Predictive Regime is Required:

**Fundamental redesign needed:**
- Different features (leading indicators, not coincident)
- Different horizons (shorter for leading signals)
- Different model architecture (predictive vs. classification)

**This is beyond threshold calibration.**

---

## Recommendation

**Accept the regime as DESCRIPTIVE.**

Rationale:
1. ADR-061 defines states as descriptive ("Uncertainty-elevated state")
2. The implementation correctly identifies current conditions
3. Predictive regime would require fundamental redesign
4. Descriptive regime still has value for strategy adaptation

**Next steps:**
1. Calibrate thresholds to improve state classification accuracy (TASK-004)
2. Build Economic Layer to predict returns WITHIN each state (separate from State Layer)
3. Strategy engine adapts to state, doesn't try to predict state transitions

---

## RiskOff Target Timing: Official Recommendation

| Target | Evidence | Verdict |
|--------|----------|---------|
| **Leading** | Before should be negative | ❌ Not supported by data |
| **Synchronous** | During = negative | ✅ **Best fit** |
| **Lagging** | After should be negative | ❌ After is strongly positive |

**RiskOff should be SYNCHRONOUS.**

It identifies uncertainty AS IT HAPPENS, not before and not after.

This is consistent with:
- ADR-061 semantic contract
- Current implementation behavior
- State Layer's purpose (describe, not predict)

---

*Document prepared for user review. Awaiting decision on regime direction (descriptive vs. predictive).*

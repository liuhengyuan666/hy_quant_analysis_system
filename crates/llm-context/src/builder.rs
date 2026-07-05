use crate::semantic_state::*;
use report_engine::DashboardSnapshot;

/// Builds ResearchContext from DashboardSnapshot
pub struct ContextBuilder;

impl ContextBuilder {
    /// Build ResearchContext from a DashboardSnapshot
    pub fn build(snapshot: &DashboardSnapshot) -> ResearchContext {
        ResearchContext {
            market: Self::build_market_context(snapshot),
            liquidity: Self::build_liquidity_context(snapshot),
            breadth: Self::build_breadth_context(snapshot),
            rotation: Self::build_rotation_context(snapshot),
            regime: Self::build_regime_context(snapshot),
            signals: Self::build_signals_context(snapshot),
            macro_: Self::build_macro_context(snapshot),
            risk: Self::build_risk_context(snapshot),
        }
    }

    fn build_market_context(snapshot: &DashboardSnapshot) -> MarketContext {
        // Confidence derived from score consistency: lower variance = higher confidence
        let scores = [snapshot.trend_score, snapshot.liquidity_score, snapshot.risk_score];
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
        let std_dev = variance.sqrt();
        // Normalize: std_dev 0 -> confidence 1.0, std_dev 30 -> confidence 0.5
        let confidence = (1.0 - std_dev / 60.0).clamp(0.3, 1.0);

        MarketContext {
            current_state: snapshot.regime_label.clone(),
            previous_state: None, // None = not yet tracked (requires history)
            confidence,
            drivers: Vec::new(),
            transition: None, // TODO: detect transitions (requires history)
        }
    }

    fn build_liquidity_context(snapshot: &DashboardSnapshot) -> LiquidityContext {
        let pressure = if snapshot.liquidity_score < 30.0 {
            LiquidityPressure::Critical
        } else if snapshot.liquidity_score < 50.0 {
            LiquidityPressure::High
        } else if snapshot.liquidity_score < 70.0 {
            LiquidityPressure::Moderate
        } else {
            LiquidityPressure::Low
        };

        LiquidityContext {
            pressure,
            spread: None,  // TODO: compute from data when available
            yield_curve_status: None, // None = not yet available
            dollar_strength: None,    // None = not yet available
        }
    }

    fn build_breadth_context(snapshot: &DashboardSnapshot) -> BreadthContext {
        let breadth_pct = snapshot
            .environment
            .as_ref()
            .map(|e| e.breadth_pct)
            .unwrap_or(0.0);

        let breadth_delta = snapshot
            .environment
            .as_ref()
            .and_then(|e| e.breadth_5d_delta)
            .unwrap_or(0.0);

        let condition = if breadth_pct < 30.0 && breadth_delta < -10.0 {
            BreadthCondition::Collapsed
        } else if breadth_pct < 50.0 || breadth_delta < -5.0 {
            BreadthCondition::Weakening
        } else {
            BreadthCondition::Strong
        };

        BreadthContext {
            condition,
            breadth_pct,
            breadth_delta,
        }
    }

    fn build_rotation_context(snapshot: &DashboardSnapshot) -> RotationContext {
        let top_sectors: Vec<String> = snapshot
            .top_rotation
            .iter()
            .map(|r| r.symbol.clone())
            .collect();

        let bottom_sectors: Vec<String> = snapshot
            .bottom_rotation
            .iter()
            .map(|r| r.symbol.clone())
            .collect();

        // Simple heuristic: if top 3 have similar momentum, it's broad
        let state = if snapshot.top_rotation.len() >= 3 {
            let top3_momentum: Vec<f64> = snapshot
                .top_rotation
                .iter()
                .take(3)
                .map(|r| r.momentum_score)
                .collect();
            let min_mm = top3_momentum
                .iter()
                .cloned()
                .fold(f64::MAX, f64::min);
            let max_mm = top3_momentum
                .iter()
                .cloned()
                .fold(f64::MIN, f64::max);
            if max_mm - min_mm < 5.0 {
                RotationState::Broad
            } else {
                RotationState::Concentrated
            }
        } else {
            RotationState::Broad
        };

        // Leadership stability: top-3 momentum score consistency
        let leadership_stability = if snapshot.top_rotation.len() >= 3 {
            let top3: Vec<f64> = snapshot.top_rotation.iter().take(3).map(|r| r.momentum_score).collect();
            let mean = top3.iter().sum::<f64>() / top3.len() as f64;
            let variance = top3.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / top3.len() as f64;
            let std_dev = variance.sqrt();
            (1.0 - std_dev / 20.0).clamp(0.0, 1.0)
        } else {
            0.5
        };

        // Momentum factor: average momentum of top 5
        let momentum_factor = if !snapshot.top_rotation.is_empty() {
            let top5: Vec<f64> = snapshot.top_rotation.iter().take(5).map(|r| r.momentum_score).collect();
            Some(top5.iter().sum::<f64>() / top5.len() as f64)
        } else {
            None
        };

        // Crowding factor: top-1 momentum / top-5 average momentum
        let crowding_factor = if snapshot.top_rotation.len() >= 5 {
            let top1 = snapshot.top_rotation[0].momentum_score;
            let top5_avg = snapshot.top_rotation.iter().take(5).map(|r| r.momentum_score).sum::<f64>() / 5.0;
            if top5_avg > 0.0 {
                Some((top1 / top5_avg).clamp(0.5, 3.0))
            } else {
                None
            }
        } else {
            None
        };

        RotationContext {
            state,
            top_sectors,
            bottom_sectors,
            leadership_stability,
            momentum_factor,
            value_factor: None,     // TODO: requires fundamental data (PE/PB/ROE)
            quality_factor: None,   // TODO: requires fundamental data (earnings quality)
            crowding_factor,
        }
    }

    fn build_regime_context(snapshot: &DashboardSnapshot) -> RegimeContext {
        // Confidence decays with staleness: fresh data = high confidence
        let freshness = (1.0 - snapshot.regime_stale_days as f64 / 10.0).clamp(0.0, 1.0);
        let trend_confidence = snapshot.trend_score / 100.0;
        let confidence = (trend_confidence * freshness).clamp(0.2, 1.0);

        RegimeContext {
            current: snapshot.regime_label.clone(),
            confidence,
            macro_stale_days: snapshot.regime_stale_days.max(0) as i32,
        }
    }

    fn build_signals_context(snapshot: &DashboardSnapshot) -> SignalsContext {
        // Estimate data-starved symbols from trust summary coverage gap
        let data_starved_count = snapshot
            .trust_summary
            .as_ref()
            .and_then(|t| {
                if t.scoped_symbols_expected > 0 {
                    let gap = t.scoped_symbols_expected.saturating_sub(t.scoped_symbols_on_freshest_market_date);
                    Some(gap)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        SignalsContext {
            bullish_count: snapshot.bullish_signals.len(),
            defensive_count: snapshot.defensive_signals.len(),
            data_starved_count,
        }
    }

    fn build_macro_context(_snapshot: &DashboardSnapshot) -> MacroContext {
        MacroContext {
            spread_10y: None,
            dxy_index: None,
            foreign_flow: None,
            vix: None,
        }
    }

    fn build_risk_context(_snapshot: &DashboardSnapshot) -> RiskContext {
        RiskContext {
            skewness: None,
            kurtosis: None,
            tail_index: None,
        }
    }
}

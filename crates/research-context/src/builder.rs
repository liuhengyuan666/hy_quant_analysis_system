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
        MarketContext {
            current_state: snapshot.regime_label.clone(),
            previous_state: None, // None = not yet tracked
            confidence: 0.8,      // TODO: compute from data quality
            drivers: Vec::new(),
            transition: None, // TODO: detect transitions
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

        RotationContext {
            state,
            top_sectors,
            bottom_sectors,
            leadership_stability: 0.7, // TODO: compute from history
            momentum_factor: None,  // TODO: compute from rotation data
            value_factor: None,     // TODO: compute from rotation data
            quality_factor: None,   // TODO: compute from rotation data
            crowding_factor: None,  // TODO: compute from rotation data
        }
    }

    fn build_regime_context(snapshot: &DashboardSnapshot) -> RegimeContext {
        RegimeContext {
            current: snapshot.regime_label.clone(),
            confidence: 0.8, // TODO: compute from regime scores
            macro_stale_days: snapshot.regime_stale_days.max(0) as i32,
        }
    }

    fn build_signals_context(snapshot: &DashboardSnapshot) -> SignalsContext {
        SignalsContext {
            bullish_count: snapshot.bullish_signals.len(),
            defensive_count: snapshot.defensive_signals.len(),
            data_starved_count: 0, // TODO: get from SignalBuildStats
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

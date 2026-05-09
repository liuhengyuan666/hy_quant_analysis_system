use core_domain::{
    DailyBar, IndicatorSnapshot, MarketRegimeSnapshot, RotationRankSnapshot, StrategyKind,
    StrategyPreferenceSnapshot,
};

#[derive(Debug, Clone)]
pub struct AnalysisContext {
    pub bar: DailyBar,
    pub indicators: IndicatorSnapshot,
    pub regime: Option<MarketRegimeSnapshot>,
    pub rotation: Option<RotationRankSnapshot>,
    pub analysis_scope: String,
    pub regime_basis_scope: String,
}

pub trait StrategyScorer {
    fn kind(&self) -> StrategyKind;
    fn score(&self, context: &AnalysisContext) -> f64;
}

fn clamp_score(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

pub struct ValueLeftScorer;
pub struct TrendPullbackScorer;
pub struct TrendBreakoutScorer;
pub struct MomentumRightScorer;

impl StrategyScorer for ValueLeftScorer {
    fn kind(&self) -> StrategyKind {
        StrategyKind::ValueLeft
    }

    fn score(&self, context: &AnalysisContext) -> f64 {
        let rsi = context.indicators.rsi14.unwrap_or(50.0);
        let ma20 = context.indicators.ma20.unwrap_or(context.bar.close);
        let distance = ((ma20 - context.bar.close) / ma20) * 100.0;
        let liquidity = context
            .regime
            .as_ref()
            .map(|row| row.liquidity_score)
            .unwrap_or(50.0);
        let base = (60.0 - rsi).max(0.0) * 1.6 + distance.max(0.0) * 6.0 + liquidity * 0.2;
        clamp_score(base)
    }
}

impl StrategyScorer for TrendPullbackScorer {
    fn kind(&self) -> StrategyKind {
        StrategyKind::TrendPullback
    }

    fn score(&self, context: &AnalysisContext) -> f64 {
        let ma20 = context.indicators.ma20.unwrap_or(context.bar.close);
        let ma60 = context.indicators.ma60.unwrap_or(ma20);
        let rsi = context.indicators.rsi14.unwrap_or(50.0);
        let trend = context
            .regime
            .as_ref()
            .map(|row| row.trend_score)
            .unwrap_or(50.0);
        let in_uptrend = if context.bar.close > ma60 { 25.0 } else { 0.0 };
        let pullback_zone = if context.bar.close < ma20 && context.bar.close > ma60 {
            30.0
        } else {
            0.0
        };
        let rsi_zone = if (40.0..=55.0).contains(&rsi) {
            20.0
        } else {
            (55.0 - (rsi - 47.5).abs()).max(0.0)
        };
        clamp_score(in_uptrend + pullback_zone + rsi_zone + trend * 0.25)
    }
}

impl StrategyScorer for TrendBreakoutScorer {
    fn kind(&self) -> StrategyKind {
        StrategyKind::TrendBreakout
    }

    fn score(&self, context: &AnalysisContext) -> f64 {
        let ma20 = context.indicators.ma20.unwrap_or(context.bar.close);
        let ma60 = context.indicators.ma60.unwrap_or(ma20);
        let macd_hist = context.indicators.macd_hist.unwrap_or(0.0);
        let trend = context
            .regime
            .as_ref()
            .map(|row| row.trend_score)
            .unwrap_or(50.0);
        let rotation = context
            .rotation
            .as_ref()
            .map(|row| row.rs_20)
            .unwrap_or(0.0);
        let price_breakout = if context.bar.close > ma20 && ma20 > ma60 {
            35.0
        } else {
            0.0
        };
        let macd_support = clamp_score(macd_hist * 40.0 + 20.0);
        let rotation_support = clamp_score(rotation * 3.0 + 20.0);
        clamp_score(price_breakout + macd_support * 0.3 + rotation_support * 0.2 + trend * 0.3)
    }
}

impl StrategyScorer for MomentumRightScorer {
    fn kind(&self) -> StrategyKind {
        StrategyKind::MomentumRight
    }

    fn score(&self, context: &AnalysisContext) -> f64 {
        let rotation = context.rotation.as_ref();
        let momentum_score = rotation.map(|row| row.momentum_score).unwrap_or(0.0);
        let rank_bonus = rotation
            .map(|row| match row.rank {
                1..=3 => 35.0,
                4..=5 => 25.0,
                6..=8 => 15.0,
                _ => 0.0,
            })
            .unwrap_or(0.0);
        let trend = context
            .regime
            .as_ref()
            .map(|row| row.trend_score)
            .unwrap_or(50.0);
        let macd = context.indicators.macd.unwrap_or(0.0);
        let base =
            clamp_score(momentum_score * 4.0 + rank_bonus + trend * 0.25 + macd * 0.4 + 10.0);
        clamp_score(base)
    }
}

pub fn build_strategy_preferences(contexts: &[AnalysisContext]) -> Vec<StrategyPreferenceSnapshot> {
    let scorers: Vec<Box<dyn StrategyScorer>> = vec![
        Box::new(ValueLeftScorer),
        Box::new(TrendPullbackScorer),
        Box::new(TrendBreakoutScorer),
        Box::new(MomentumRightScorer),
    ];

    contexts
        .iter()
        .map(|context| {
            let value_left_score = scorers[0].score(context);
            let trend_pullback_score = scorers[1].score(context);
            let trend_breakout_score = scorers[2].score(context);
            let momentum_right_score = scorers[3].score(context);
            let scored = [
                (StrategyKind::ValueLeft, value_left_score),
                (StrategyKind::TrendPullback, trend_pullback_score),
                (StrategyKind::TrendBreakout, trend_breakout_score),
                (StrategyKind::MomentumRight, momentum_right_score),
            ];
            let (best_strategy, confidence) = scored
                .iter()
                .max_by(|left, right| left.1.total_cmp(&right.1))
                .map(|(kind, score)| (kind.clone(), *score))
                .unwrap_or((StrategyKind::ValueLeft, 0.0));
            let alignment = scored.iter().filter(|(_, score)| *score >= 60.0).count() as u8;

            StrategyPreferenceSnapshot {
                date: context.bar.date,
                symbol: context.bar.symbol.clone(),
                analysis_scope: context.analysis_scope.clone(),
                regime_basis_scope: context.regime_basis_scope.clone(),
                value_left_score,
                trend_pullback_score,
                trend_breakout_score,
                momentum_right_score,
                best_strategy,
                confidence,
                alignment,
            }
        })
        .collect()
}

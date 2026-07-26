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

// ─────────────────────────────────────────────────────────────
// RV1 Phase 2: Strategy Preference Exposure (attribution types)
// ─────────────────────────────────────────────────────────────

/// A single contributing factor to a strategy score.
/// Pure computation artifact — never persisted, recomputed at consumption time.
#[derive(Debug, Clone)]
pub struct AttributionDriver {
    pub factor: String,
    pub value: f64,
    pub contribution: f64,
    pub note: String,
}

/// Strategy score plus its explanation.
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub score: f64,
    pub drivers: Vec<AttributionDriver>,
}

pub trait StrategyScorer {
    fn kind(&self) -> StrategyKind;

    /// Full score + attribution. This is the single source of truth for scoring;
    /// `score()` delegates to it so attribution can never drift from production scores.
    fn breakdown(&self, context: &AnalysisContext) -> ScoreBreakdown;

    fn score(&self, context: &AnalysisContext) -> f64 {
        self.breakdown(context).score
    }
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

    fn breakdown(&self, context: &AnalysisContext) -> ScoreBreakdown {
        let rsi = context.indicators.rsi14.unwrap_or(50.0);
        let ma20 = context.indicators.ma20.unwrap_or(context.bar.close);
        let distance = ((ma20 - context.bar.close) / ma20) * 100.0;
        let liquidity = context
            .regime
            .as_ref()
            .map(|row| row.liquidity_score)
            .unwrap_or(50.0);

        let rsi_contrib = (60.0 - rsi).max(0.0) * 1.6;
        let distance_contrib = distance.max(0.0) * 6.0;
        let liquidity_contrib = liquidity * 0.2;

        let drivers = vec![
            AttributionDriver {
                factor: "RSI14".into(),
                value: rsi,
                contribution: rsi_contrib,
                note: if rsi < 40.0 {
                    format!("RSI {:.1} 处于超卖区间，均值回归加分", rsi)
                } else {
                    format!("RSI {:.1} 未进入超卖区间", rsi)
                },
            },
            AttributionDriver {
                factor: "MA20距离".into(),
                value: distance,
                contribution: distance_contrib,
                note: if distance > 0.0 {
                    format!("价格低于 MA20 {:.1}%，左侧存在回归空间", distance)
                } else {
                    format!("价格高于 MA20 {:.1}%，左侧策略无空间", -distance)
                },
            },
            AttributionDriver {
                factor: "流动性".into(),
                value: liquidity,
                contribution: liquidity_contrib,
                note: format!("流动性环境得分 {:.0}", liquidity),
            },
        ];

        ScoreBreakdown {
            score: clamp_score(rsi_contrib + distance_contrib + liquidity_contrib),
            drivers,
        }
    }
}

impl StrategyScorer for TrendPullbackScorer {
    fn kind(&self) -> StrategyKind {
        StrategyKind::TrendPullback
    }

    fn breakdown(&self, context: &AnalysisContext) -> ScoreBreakdown {
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
        let trend_contrib = trend * 0.25;

        let drivers = vec![
            AttributionDriver {
                factor: "趋势结构".into(),
                value: in_uptrend,
                contribution: in_uptrend,
                note: if in_uptrend > 0.0 {
                    "价格位于 MA60 之上，处于上升趋势".to_string()
                } else {
                    "价格跌破 MA60，非上升趋势".to_string()
                },
            },
            AttributionDriver {
                factor: "回调区间".into(),
                value: pullback_zone,
                contribution: pullback_zone,
                note: if pullback_zone > 0.0 {
                    "价格位于 MA20-MA60 回调区间，回踩买入区".to_string()
                } else {
                    "价格不在 MA20-MA60 回调区间内".to_string()
                },
            },
            AttributionDriver {
                factor: "RSI14".into(),
                value: rsi,
                contribution: rsi_zone,
                note: if (40.0..=55.0).contains(&rsi) {
                    format!("RSI {:.1} 处于健康回调带 (40-55)", rsi)
                } else {
                    format!("RSI {:.1} 偏离健康回调带", rsi)
                },
            },
            AttributionDriver {
                factor: "宏观趋势".into(),
                value: trend,
                contribution: trend_contrib,
                note: format!("宏观趋势得分 {:.0}", trend),
            },
        ];

        ScoreBreakdown {
            score: clamp_score(in_uptrend + pullback_zone + rsi_zone + trend_contrib),
            drivers,
        }
    }
}

impl StrategyScorer for TrendBreakoutScorer {
    fn kind(&self) -> StrategyKind {
        StrategyKind::TrendBreakout
    }

    fn breakdown(&self, context: &AnalysisContext) -> ScoreBreakdown {
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
        let macd_contrib = macd_support * 0.3;
        let rotation_contrib = rotation_support * 0.2;
        let trend_contrib = trend * 0.3;

        let drivers = vec![
            AttributionDriver {
                factor: "价格突破".into(),
                value: price_breakout,
                contribution: price_breakout,
                note: if price_breakout > 0.0 {
                    "价格站上 MA20 且 MA20>MA60，突破结构成立".to_string()
                } else {
                    "突破结构未成立（价格未站上 MA20 或 MA20≤MA60）".to_string()
                },
            },
            AttributionDriver {
                factor: "MACD".into(),
                value: macd_hist,
                contribution: macd_contrib,
                note: format!("MACD hist {:.4}，动能确认度 {:.0}", macd_hist, macd_support),
            },
            AttributionDriver {
                factor: "相对强弱".into(),
                value: rotation,
                contribution: rotation_contrib,
                note: format!("RS20 {:.2}，轮动支撑度 {:.0}", rotation, rotation_support),
            },
            AttributionDriver {
                factor: "宏观趋势".into(),
                value: trend,
                contribution: trend_contrib,
                note: format!("宏观趋势得分 {:.0}", trend),
            },
        ];

        ScoreBreakdown {
            score: clamp_score(price_breakout + macd_contrib + rotation_contrib + trend_contrib),
            drivers,
        }
    }
}

impl StrategyScorer for MomentumRightScorer {
    fn kind(&self) -> StrategyKind {
        StrategyKind::MomentumRight
    }

    fn breakdown(&self, context: &AnalysisContext) -> ScoreBreakdown {
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

        let momentum_contrib = momentum_score * 4.0;
        let trend_contrib = trend * 0.25;
        let macd_contrib = macd * 0.4;
        let base_bonus = 10.0;

        let drivers = vec![
            AttributionDriver {
                factor: "动量得分".into(),
                value: momentum_score,
                contribution: momentum_contrib,
                note: format!("合成动量 {:.2}", momentum_score),
            },
            AttributionDriver {
                factor: "轮动排名".into(),
                value: rotation.map(|row| row.rank as f64).unwrap_or(0.0),
                contribution: rank_bonus,
                note: match rank_bonus {
                    x if x == 35.0 => "轮动排名 Top3，龙头加分".to_string(),
                    x if x == 25.0 => "轮动排名 Top5".to_string(),
                    x if x == 15.0 => "轮动排名 Top8".to_string(),
                    _ => "轮动排名靠后，无排名加分".to_string(),
                },
            },
            AttributionDriver {
                factor: "宏观趋势".into(),
                value: trend,
                contribution: trend_contrib,
                note: format!("宏观趋势得分 {:.0}", trend),
            },
            AttributionDriver {
                factor: "MACD".into(),
                value: macd,
                contribution: macd_contrib,
                note: format!("MACD {:.4}", macd),
            },
        ];

        ScoreBreakdown {
            score: clamp_score(momentum_contrib + rank_bonus + trend_contrib + macd_contrib + base_bonus),
            drivers,
        }
    }
}

pub fn default_scorers() -> Vec<Box<dyn StrategyScorer>> {
    vec![
        Box::new(ValueLeftScorer),
        Box::new(TrendPullbackScorer),
        Box::new(TrendBreakoutScorer),
        Box::new(MomentumRightScorer),
    ]
}

/// Compute full attribution for all four strategies over one context.
/// Used by the consumption layer (strategy-perspectives detail mode).
pub fn build_strategy_attributions(context: &AnalysisContext) -> Vec<(StrategyKind, ScoreBreakdown)> {
    default_scorers()
        .iter()
        .map(|scorer| (scorer.kind(), scorer.breakdown(context)))
        .collect()
}

pub fn build_strategy_preferences(contexts: &[AnalysisContext]) -> Vec<StrategyPreferenceSnapshot> {
    let scorers = default_scorers();

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_context() -> AnalysisContext {
        AnalysisContext {
            bar: DailyBar {
                date: NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                symbol: "000300".into(),
                open: 100.0,
                high: 103.0,
                low: 99.0,
                close: 102.0,
                volume: 1_000_000.0,
                turnover: Some(0.5),
            },
            indicators: IndicatorSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                symbol: "000300".into(),
                ma10: Some(100.0),
                ma20: Some(101.0),
                ma30: Some(100.0),
                ma60: Some(99.0),
                ma120: Some(98.0),
                ema12: Some(101.0),
                ema26: Some(100.0),
                macd: Some(1.0),
                macd_signal: Some(0.8),
                macd_hist: Some(0.2),
                rsi14: Some(45.0),
                atr14: Some(1.5),
                vol_ma20: Some(900_000.0),
                vol_ma60: Some(850_000.0),
            },
            regime: Some(MarketRegimeSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                macro_as_of_date: NaiveDate::from_ymd_opt(2026, 7, 18).unwrap(),
                market: "CN".into(),
                trend_score: 65.0,
                liquidity_score: 55.0,
                risk_score: 45.0,
                regime_label: "neutral".into(),
            }),
            rotation: Some(RotationRankSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 7, 20).unwrap(),
                symbol: "000300".into(),
                rs_20: 2.5,
                rs_60: 3.0,
                rs_120: 4.0,
                momentum_score: 5.0,
                rank: 2,
            }),
            analysis_scope: "CN".into(),
            regime_basis_scope: "CN".into(),
        }
    }

    /// Gate 1 lock: score() must always equal breakdown().score.
    /// This guarantees attribution can never drift from production scores.
    #[test]
    fn score_equals_breakdown_score_for_all_scorers() {
        let context = make_context();
        for scorer in default_scorers() {
            let direct = scorer.score(&context);
            let via_breakdown = scorer.breakdown(&context).score;
            assert_eq!(
                direct, via_breakdown,
                "{:?} score drift: {} vs {}",
                scorer.kind(),
                direct,
                via_breakdown
            );
        }
    }

    /// Gate 1 lock: scores must match the original formula outputs exactly.
    #[test]
    fn scores_match_original_formulas() {
        let context = make_context();

        // ValueLeft: (60-45)*1.6 + ((101-102)/101*100).max(0)*6 + 55*0.2
        //          = 24.0 + 0.0 + 11.0 = 35.0
        let vl = ValueLeftScorer.score(&context);
        assert!((vl - 35.0).abs() < 1e-9, "ValueLeft expected 35.0, got {}", vl);

        // TrendPullback: in_uptrend(102>99 → 25) + pullback(102<101? no → 0)
        //   + rsi(45 in 40..=55 → 20) + 65*0.25 = 25+0+20+16.25 = 61.25
        let tp = TrendPullbackScorer.score(&context);
        assert!((tp - 61.25).abs() < 1e-9, "TrendPullback expected 61.25, got {}", tp);

        // TrendBreakout: breakout(102>101 && 101>99 → 35) + clamp(0.2*40+20)*0.3
        //   + clamp(2.5*3+20)*0.2 + 65*0.3 = 35 + 28*0.3 + 27.5*0.2 + 19.5 = 68.4
        let tb = TrendBreakoutScorer.score(&context);
        assert!((tb - 68.4).abs() < 1e-9, "TrendBreakout expected 68.4, got {}", tb);

        // MomentumRight: 5*4 + rank2→35 + 65*0.25 + 1.0*0.4 + 10 = 20+35+16.25+0.4+10 = 81.65
        let mr = MomentumRightScorer.score(&context);
        assert!((mr - 81.65).abs() < 1e-9, "MomentumRight expected 81.65, got {}", mr);
    }

    /// Gate 2 lock: every scorer produces non-empty attribution.
    #[test]
    fn attribution_drivers_are_explainable() {
        let context = make_context();
        for scorer in default_scorers() {
            let breakdown = scorer.breakdown(&context);
            assert!(
                !breakdown.drivers.is_empty(),
                "{:?} produced no drivers",
                scorer.kind()
            );
            for driver in &breakdown.drivers {
                assert!(!driver.factor.is_empty());
                assert!(!driver.note.is_empty());
            }
        }
    }
}

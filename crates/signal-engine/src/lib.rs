use core_domain::{
    MarketRegimeSnapshot, RegimeReason, RotationRankSnapshot, RotationReason, SignalBuildStats,
    SignalLabel, SignalReason, SignalSnapshot, StrategyKind, StrategyPreferenceSnapshot,
};

fn clamp_score(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

fn best_strategy_score(row: &StrategyPreferenceSnapshot) -> f64 {
    match row.best_strategy {
        StrategyKind::ValueLeft => row.value_left_score,
        StrategyKind::TrendPullback => row.trend_pullback_score,
        StrategyKind::TrendBreakout => row.trend_breakout_score,
        StrategyKind::MomentumRight => row.momentum_right_score,
    }
}

fn aligned_strategies(row: &StrategyPreferenceSnapshot) -> Vec<StrategyKind> {
    [
        (StrategyKind::ValueLeft, row.value_left_score),
        (StrategyKind::TrendPullback, row.trend_pullback_score),
        (StrategyKind::TrendBreakout, row.trend_breakout_score),
        (StrategyKind::MomentumRight, row.momentum_right_score),
    ]
    .into_iter()
    .filter_map(|(strategy, score)| (score >= 60.0).then_some(strategy))
    .collect()
}

fn label_from_score(score: f64) -> SignalLabel {
    if score >= 80.0 {
        SignalLabel::StrongBuy
    } else if score >= 65.0 {
        SignalLabel::Buy
    } else if score >= 50.0 {
        SignalLabel::Watch
    } else if score >= 35.0 {
        SignalLabel::Hold
    } else if score >= 20.0 {
        SignalLabel::Reduce
    } else {
        SignalLabel::Sell
    }
}

pub fn build_signal_snapshots(
    strategies: &[StrategyPreferenceSnapshot],
    regimes: &[MarketRegimeSnapshot],
    rotations: &[RotationRankSnapshot],
) -> (Vec<SignalSnapshot>, SignalBuildStats) {
    let regime_by_date = regimes
        .iter()
        .map(|row| ((row.date, row.market.clone()), row))
        .collect::<std::collections::BTreeMap<_, _>>();
    let rotation_by_key = rotations
        .iter()
        .map(|row| ((row.date, row.symbol.clone()), row))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut stats = SignalBuildStats::default();
    let snapshots: Vec<SignalSnapshot> = strategies
        .iter()
        .map(|row| {
            stats.total += 1;
            let strategy_score = best_strategy_score(row);
            let regime = regime_by_date
                .get(&(row.date, row.regime_basis_scope.clone()))
                .copied();
            let rotation = rotation_by_key
                .get(&(row.date, row.symbol.clone()))
                .copied();

            let alignment_score = row.alignment as f64 * 20.0;
            let market_regime_score = if let Some(item) = regime {
                (item.trend_score + item.risk_score) / 2.0
            } else {
                stats.regime_missing += 1;
                50.0
            };
            let rotation_score = if let Some(item) = rotation {
                clamp_score(
                    item.momentum_score * 4.0 + (11_u32.saturating_sub(item.rank)) as f64 * 4.0,
                )
            } else {
                stats.rotation_missing += 1;
                40.0
            };

            let final_score = clamp_score(
                strategy_score * 0.45
                    + alignment_score * 0.15
                    + market_regime_score * 0.20
                    + rotation_score * 0.20,
            );

            let label = label_from_score(final_score);
            let reason = SignalReason {
                best_strategy: row.best_strategy.clone(),
                strategy_score,
                strategy_contribution: strategy_score * 0.45,
                alignment: row.alignment,
                aligned_strategies: aligned_strategies(row),
                alignment_contribution: alignment_score * 0.15,
                regime: RegimeReason {
                    trend_score: regime.map(|item| item.trend_score).unwrap_or(50.0),
                    risk_score: regime.map(|item| item.risk_score).unwrap_or(50.0),
                    combined_score: market_regime_score,
                    contribution: market_regime_score * 0.20,
                },
                rotation: RotationReason {
                    momentum_score: rotation.map(|item| item.momentum_score).unwrap_or(40.0),
                    rank: rotation.map(|item| item.rank),
                    combined_score: rotation_score,
                    contribution: rotation_score * 0.20,
                },
                final_score,
                label: label.clone(),
                summary: format!(
                    "动量最强策略{:?}得分{:.1}，趋势分{:.1}，轮动分{:.1}，最终信号{:?}",
                    row.best_strategy, strategy_score, market_regime_score, rotation_score, label
                ),
            };

            SignalSnapshot {
                date: row.date,
                symbol: row.symbol.clone(),
                final_score,
                signal_label: label,
                analysis_scope: row.analysis_scope.clone(),
                regime_basis_scope: row.regime_basis_scope.clone(),
                reason,
            }
        })
        .collect();
    (snapshots, stats)
}

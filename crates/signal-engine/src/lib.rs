use core_domain::{
    MarketRegimeSnapshot, RotationRankSnapshot, SignalLabel, SignalSnapshot, StrategyKind,
    StrategyPreferenceSnapshot,
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
) -> Vec<SignalSnapshot> {
    let regime_by_date = regimes
        .iter()
        .map(|row| (row.date, row))
        .collect::<std::collections::BTreeMap<_, _>>();
    let rotation_by_key = rotations
        .iter()
        .map(|row| ((row.date, row.symbol.clone()), row))
        .collect::<std::collections::BTreeMap<_, _>>();

    strategies
        .iter()
        .map(|row| {
            let strategy_score = best_strategy_score(row);
            let regime = regime_by_date.get(&row.date).copied();
            let rotation = rotation_by_key
                .get(&(row.date, row.symbol.clone()))
                .copied();

            let alignment_score = row.alignment as f64 * 20.0;
            let market_regime_score = regime
                .map(|item| (item.trend_score + item.risk_score) / 2.0)
                .unwrap_or(50.0);
            let rotation_score = rotation
                .map(|item| {
                    clamp_score(
                        item.momentum_score * 4.0 + (11_u32.saturating_sub(item.rank)) as f64 * 4.0,
                    )
                })
                .unwrap_or(40.0);

            let final_score = clamp_score(
                strategy_score * 0.45
                    + alignment_score * 0.15
                    + market_regime_score * 0.20
                    + rotation_score * 0.20,
            );

            let label = label_from_score(final_score);
            let explanation = format!(
                "best={:?}({:.1}), alignment={}, regime={:.1}, rotation={:.1}",
                row.best_strategy,
                strategy_score,
                row.alignment,
                market_regime_score,
                rotation_score
            );

            SignalSnapshot {
                date: row.date,
                symbol: row.symbol.clone(),
                final_score,
                signal_label: label,
                explanation,
            }
        })
        .collect()
}

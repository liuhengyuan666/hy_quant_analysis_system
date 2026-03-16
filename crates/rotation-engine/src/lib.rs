use std::collections::BTreeMap;

use core_domain::{DailyBar, RotationRankSnapshot};

fn pct_change(current: f64, previous: f64) -> f64 {
    if previous.abs() < f64::EPSILON {
        0.0
    } else {
        ((current / previous) - 1.0) * 100.0
    }
}

fn compute_rs_window(closes: &[f64], index: usize, period: usize) -> Option<f64> {
    if index < period {
        return None;
    }
    Some(pct_change(closes[index], closes[index - period]))
}

pub fn build_rotation_ranks(
    series_by_symbol: &BTreeMap<String, Vec<DailyBar>>,
) -> Vec<RotationRankSnapshot> {
    let mut daily_rows: BTreeMap<chrono::NaiveDate, Vec<RotationRankSnapshot>> = BTreeMap::new();

    for (symbol, bars) in series_by_symbol {
        let closes: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
        for (index, bar) in bars.iter().enumerate() {
            let Some(rs_20) = compute_rs_window(&closes, index, 20) else {
                continue;
            };
            let rs_60 = compute_rs_window(&closes, index, 60).unwrap_or(rs_20);
            let rs_120 = compute_rs_window(&closes, index, 120).unwrap_or(rs_60);
            let momentum_score = rs_20 * 0.5 + rs_60 * 0.3 + rs_120 * 0.2;
            daily_rows
                .entry(bar.date)
                .or_default()
                .push(RotationRankSnapshot {
                    date: bar.date,
                    symbol: symbol.clone(),
                    rs_20,
                    rs_60,
                    rs_120,
                    momentum_score,
                    rank: 0,
                });
        }
    }

    let mut ranked = Vec::new();
    for (_date, mut rows) in daily_rows {
        rows.sort_by(|left, right| {
            right
                .momentum_score
                .total_cmp(&left.momentum_score)
                .then_with(|| left.symbol.cmp(&right.symbol))
        });
        for (index, row) in rows.iter_mut().enumerate() {
            row.rank = (index + 1) as u32;
        }
        ranked.extend(rows);
    }
    ranked
}

pub fn latest_rotation_view(rows: &[RotationRankSnapshot]) -> Vec<RotationRankSnapshot> {
    let Some(latest_date) = rows.iter().map(|row| row.date).max() else {
        return Vec::new();
    };
    let mut latest = rows
        .iter()
        .filter(|row| row.date == latest_date)
        .cloned()
        .collect::<Vec<_>>();
    latest.sort_by(|left, right| left.rank.cmp(&right.rank));
    latest
}

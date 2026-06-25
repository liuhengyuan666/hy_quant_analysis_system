use std::collections::BTreeMap;

use core_domain::{DailyBar, RotationRankSnapshot};
use rayon::prelude::*;

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

/// Parallel version: compute rotation ranks for multiple symbols concurrently.
/// This is a safe Rayon entry point; caller must invoke via `spawn_blocking` if in async context.
pub fn build_rotation_ranks_parallel(
    series_by_symbol: &BTreeMap<String, Vec<DailyBar>>,
) -> Vec<RotationRankSnapshot> {
    // Step 1: per-symbol RS computation (parallel)
    let per_symbol_results: Vec<Vec<RotationRankSnapshot>> = series_by_symbol
        .par_iter()
        .map(|(symbol, bars)| {
            let mut rows = Vec::new();
            let closes: Vec<f64> = bars.iter().map(|bar| bar.close).collect();
            for (index, bar) in bars.iter().enumerate() {
                let Some(rs_20) = compute_rs_window(&closes, index, 20) else {
                    continue;
                };
                let rs_60 = compute_rs_window(&closes, index, 60).unwrap_or(rs_20);
                let rs_120 = compute_rs_window(&closes, index, 120).unwrap_or(rs_60);
                let momentum_score = rs_20 * 0.5 + rs_60 * 0.3 + rs_120 * 0.2;
                rows.push(RotationRankSnapshot {
                    date: bar.date,
                    symbol: symbol.clone(),
                    rs_20,
                    rs_60,
                    rs_120,
                    momentum_score,
                    rank: 0,
                });
            }
            rows
        })
        .collect();

    // Step 2: merge into daily rows (serial, BTreeMap not Send)
    let mut daily_rows: BTreeMap<chrono::NaiveDate, Vec<RotationRankSnapshot>> = BTreeMap::new();
    for rows in per_symbol_results {
        for row in rows {
            daily_rows.entry(row.date).or_default().push(row);
        }
    }

    // Step 3: per-day ranking (parallel)
    let ranked: Vec<RotationRankSnapshot> = daily_rows
        .into_par_iter()
        .flat_map(|(_date, mut rows)| {
            rows.sort_by(|left, right| {
                right
                    .momentum_score
                    .total_cmp(&left.momentum_score)
                    .then_with(|| left.symbol.cmp(&right.symbol))
            });
            for (index, row) in rows.iter_mut().enumerate() {
                row.rank = (index + 1) as u32;
            }
            rows
        })
        .collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_bars(symbol: &str, n: usize) -> Vec<DailyBar> {
        let mut bars = Vec::new();
        for i in 0..n {
            bars.push(DailyBar {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap() + chrono::Duration::days(i as i64),
                symbol: symbol.to_string(),
                open: 100.0 + i as f64,
                high: 102.0 + i as f64,
                low: 99.0 + i as f64,
                close: 101.0 + i as f64,
                volume: 1000.0 + i as f64,
                turnover: None,
            });
        }
        bars
    }

    #[test]
    fn parallel_produces_same_output_as_serial() {
        let mut series_by_symbol = BTreeMap::new();
        series_by_symbol.insert("A".to_string(), make_bars("A", 65));
        series_by_symbol.insert("B".to_string(), make_bars("B", 65));
        series_by_symbol.insert("C".to_string(), make_bars("C", 65));

        let serial = build_rotation_ranks(&series_by_symbol);
        let parallel = build_rotation_ranks_parallel(&series_by_symbol);

        assert_eq!(serial.len(), parallel.len(), "serial and parallel should produce same number of rows");

        let mut serial_sorted = serial.clone();
        let mut parallel_sorted = parallel.clone();
        serial_sorted.sort_by(|a, b| (a.date, &a.symbol).cmp(&(b.date, &b.symbol)));
        parallel_sorted.sort_by(|a, b| (a.date, &a.symbol).cmp(&(b.date, &b.symbol)));

        for (s, p) in serial_sorted.iter().zip(parallel_sorted.iter()) {
            assert_eq!(s.date, p.date, "date mismatch");
            assert_eq!(s.symbol, p.symbol, "symbol mismatch");
            assert!((s.momentum_score - p.momentum_score).abs() < 1e-6, "momentum_score mismatch for {}@{}", s.symbol, s.date);
            assert_eq!(s.rank, p.rank, "rank mismatch for {}@{}", s.symbol, s.date);
        }
    }
}

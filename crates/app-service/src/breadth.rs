use chrono::NaiveDate;
use core_domain::AnalysisScope as ReportScope;
use report_engine::WatchlistBreadthMarketSnapshot;

use crate::{
    ParticipationMetrics, ParticipationPoint, TrackedInstrumentSeries,
};

pub(crate) fn compute_participation_point(
    series: &[TrackedInstrumentSeries],
    date: NaiveDate,
) -> ParticipationPoint {
    let mut eligible_count = 0usize;
    let mut above_count = 0usize;
    let mut liquidity_eligible_count = 0usize;
    let mut volume_expansion_count = 0usize;
    let mut turnover_present_count = 0usize;

    for item in series {
        let Some(close) = item.close_by_date.get(&date).copied() else {
            continue;
        };
        let Some(ma30) = item.ma30_by_date.get(&date).copied() else {
            continue;
        };

        eligible_count += 1;
        if close > ma30 {
            above_count += 1;
        }
        if let (Some(volume), Some(vol_ma20)) = (
            item.volume_by_date.get(&date).copied(),
            item.vol_ma20_by_date.get(&date).copied(),
        ) {
            liquidity_eligible_count += 1;
            if volume > vol_ma20 {
                volume_expansion_count += 1;
            }
        }
        if item
            .turnover_present_by_date
            .get(&date)
            .copied()
            .unwrap_or(false)
        {
            turnover_present_count += 1;
        }
    }

    let breadth_pct = if eligible_count > 0 {
        above_count as f64 / eligible_count as f64 * 100.0
    } else {
        0.0
    };

    let volume_expansion_pct = (liquidity_eligible_count > 0)
        .then(|| volume_expansion_count as f64 / liquidity_eligible_count as f64 * 100.0);
    let turnover_coverage_pct =
        (eligible_count > 0).then(|| turnover_present_count as f64 / eligible_count as f64 * 100.0);
    let liquidity_proxy_score = match (volume_expansion_pct, turnover_coverage_pct) {
        (Some(volume_pct), Some(turnover_pct)) => volume_pct * 0.7 + turnover_pct * 0.3,
        (Some(volume_pct), None) => volume_pct,
        (None, Some(turnover_pct)) => turnover_pct,
        (None, None) => 50.0,
    };

    ParticipationPoint {
        breadth_pct,
        eligible_count,
        above_count,
        volume_expansion_pct,
        turnover_coverage_pct,
        liquidity_proxy_score,
    }
}

pub(crate) fn compute_watchlist_breadth_status(
    eligible_count: usize,
    breadth_pct: f64,
    range_position_60d: Option<f64>,
    breadth_5d_delta: Option<f64>,
) -> String {
    if eligible_count == 0 {
        return "unavailable".to_string();
    }
    if let Some(position) = range_position_60d {
        if position <= 0.20 {
            return "near_local_low".to_string();
        }
        if position >= 0.80 {
            return "near_local_high".to_string();
        }
    }
    if let Some(delta) = breadth_5d_delta {
        if delta >= 10.0 {
            return "improving".to_string();
        }
        if delta <= -10.0 {
            return "weakening".to_string();
        }
    }
    if breadth_pct < 35.0 {
        "weak".to_string()
    } else if breadth_pct > 65.0 {
        "strong".to_string()
    } else {
        "neutral".to_string()
    }
}

pub(crate) fn build_market_watchlist_breadth_snapshot(
    scope: ReportScope,
    series: &[TrackedInstrumentSeries],
    report_date: NaiveDate,
    relevant_dates: &[NaiveDate],
) -> WatchlistBreadthMarketSnapshot {
    use crate::{scope_label, scope_universe_label};

    let metrics = compute_participation_metrics(series, report_date, relevant_dates);

    WatchlistBreadthMarketSnapshot {
        market: scope_label(scope).to_string(),
        universe_label: scope_universe_label(scope).to_string(),
        eligible_count: metrics.current.eligible_count,
        above_count: metrics.current.above_count,
        breadth_pct: metrics.current.breadth_pct,
        breadth_pct_sma5: metrics.breadth_pct_sma5,
        breadth_5d_delta: metrics.breadth_5d_delta,
        range_low_60d: metrics.range_low_60d,
        range_high_60d: metrics.range_high_60d,
        range_position_60d: metrics.range_position_60d,
        status_label: metrics.breadth_state,
    }
}

pub(crate) fn compute_participation_metrics(
    series: &[TrackedInstrumentSeries],
    report_date: NaiveDate,
    relevant_dates: &[NaiveDate],
) -> ParticipationMetrics {
    let current = compute_participation_point(series, report_date);
    let history = relevant_dates
        .iter()
        .copied()
        .filter(|date| *date <= report_date)
        .filter_map(|date| {
            let point = compute_participation_point(series, date);
            (point.eligible_count > 0).then_some(point)
        })
        .collect::<Vec<_>>();

    let breadth_pct_sma5 = (history.len() >= 5).then(|| {
        let window = &history[history.len() - 5..];
        window.iter().map(|point| point.breadth_pct).sum::<f64>() / window.len() as f64
    });
    let breadth_5d_delta = (history.len() >= 6).then(|| {
        let current = history[history.len() - 1].breadth_pct;
        let previous = history[history.len() - 6].breadth_pct;
        current - previous
    });
    let (range_low_60d, range_high_60d, range_position_60d) = if history.len() >= 60 {
        let window = &history[history.len() - 60..];
        let range_low = window
            .iter()
            .map(|point| point.breadth_pct)
            .fold(f64::INFINITY, f64::min);
        let range_high = window
            .iter()
            .map(|point| point.breadth_pct)
            .fold(f64::NEG_INFINITY, f64::max);
        let position = if (range_high - range_low).abs() < f64::EPSILON {
            Some(0.5)
        } else {
            Some(((current.breadth_pct - range_low) / (range_high - range_low)).clamp(0.0, 1.0))
        };
        (Some(range_low), Some(range_high), position)
    } else {
        (None, None, None)
    };

    let breadth_state = compute_watchlist_breadth_status(
        current.eligible_count,
        current.breadth_pct,
        range_position_60d,
        breadth_5d_delta,
    );

    ParticipationMetrics {
        current,
        breadth_pct_sma5,
        breadth_5d_delta,
        range_low_60d,
        range_high_60d,
        range_position_60d,
        breadth_state,
    }
}

/// Score breadth momentum from 5-day delta for environment calculation.
pub(crate) fn breadth_momentum_score(delta: Option<f64>) -> f64 {
    match delta {
        Some(value) if value >= 10.0 => 70.0,
        Some(value) if value >= 3.0 => 60.0,
        Some(value) if value <= -10.0 => 25.0,
        Some(value) if value <= -3.0 => 40.0,
        Some(_) => 50.0,
        None => 45.0,
    }
}

/// Map a composite environment score to a human-readable label.
pub(crate) fn environment_label(score: f64) -> &'static str {
    if score >= 70.0 {
        "supportive"
    } else if score >= 55.0 {
        "constructive"
    } else if score >= 40.0 {
        "mixed"
    } else if score >= 25.0 {
        "fragile"
    } else {
        "stressed"
    }
}

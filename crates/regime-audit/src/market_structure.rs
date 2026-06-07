use chrono::{Datelike, NaiveDate};
use core_domain::DailyBar;
use std::collections::HashMap;

// ============================================================
// TASK-024: HK Structural Market Audit
// Determines whether HK's 69% RiskOff is model bias or market reality.
// ============================================================

#[derive(Debug, Clone)]
pub struct MarketStructureReport {
    pub scope: String,
    pub anchor_symbol: String,
    pub window_from: NaiveDate,
    pub window_to: NaiveDate,
    pub total_days: usize,
    pub total_months: usize,
    pub total_quarters: usize,
    pub annualized_return: f64,
    pub annualized_volatility: f64,
    pub max_drawdown: f64,
    pub positive_month_ratio: f64,
    pub positive_quarter_ratio: f64,
    pub up_months: usize,
    pub down_months: usize,
    pub up_quarters: usize,
    pub down_quarters: usize,
    pub drawdown_profile: DrawdownProfile,
    pub monthly_returns: Vec<(String, f64)>,
    pub quarterly_returns: Vec<(String, f64)>,
}

#[derive(Debug, Clone)]
pub struct DrawdownProfile {
    pub dd_over_10_pct: f64,
    pub dd_over_20_pct: f64,
    pub dd_over_30_pct: f64,
    pub avg_drawdown: f64,
    pub max_drawdown: f64,
    pub max_drawdown_date: NaiveDate,
}

/// Compute monthly returns from daily bars.
fn monthly_returns(bars: &[DailyBar]) -> Vec<(String, f64)> {
    let mut month_closes: HashMap<String, (f64, f64)> = HashMap::new(); // month_key -> (first_close, last_close)

    for bar in bars {
        let month_key = format!("{}-{:02}", bar.date.year(), bar.date.month());
        month_closes
            .entry(month_key)
            .and_modify(|e| e.1 = bar.close)
            .or_insert((bar.close, bar.close));
    }

    let mut months: Vec<(String, f64)> = month_closes
        .iter()
        .map(|(k, (first, last))| {
            let ret = if *first > 0.0 {
                (last - first) / first
            } else {
                0.0
            };
            (k.clone(), ret)
        })
        .collect();

    months.sort_by(|a, b| a.0.cmp(&b.0));
    months
}

/// Compute quarterly returns from monthly returns.
fn quarterly_returns(monthly: &[(String, f64)]) -> Vec<(String, f64)> {
    let mut quarter_rets: HashMap<String, Vec<f64>> = HashMap::new();

    for (month_key, ret) in monthly {
        let parts: Vec<&str> = month_key.split('-').collect();
        if parts.len() != 2 {
            continue;
        }
        let year: i32 = parts[0].parse().unwrap_or(0);
        let month: u32 = parts[1].parse().unwrap_or(1);
        let quarter = (month - 1) / 3 + 1;
        let quarter_key = format!("{}-Q{}", year, quarter);
        quarter_rets.entry(quarter_key).or_default().push(*ret);
    }

    let mut quarters: Vec<(String, f64)> = quarter_rets
        .iter()
        .map(|(k, rets)| {
            // Compound quarterly return
            let compound = rets.iter().fold(1.0, |acc, r| acc * (1.0 + r)) - 1.0;
            (k.clone(), compound)
        })
        .collect();

    quarters.sort_by(|a, b| a.0.cmp(&b.0));
    quarters
}

/// Compute drawdown profile.
fn compute_drawdown_profile(bars: &[DailyBar]) -> DrawdownProfile {
    if bars.is_empty() {
        return DrawdownProfile {
            dd_over_10_pct: 0.0,
            dd_over_20_pct: 0.0,
            dd_over_30_pct: 0.0,
            avg_drawdown: 0.0,
            max_drawdown: 0.0,
            max_drawdown_date: NaiveDate::MIN,
        };
    }

    let mut max_dd = 0.0;
    let mut peak = bars[0].close;
    let mut peak_date = bars[0].date;
    let mut max_dd_date = bars[0].date;
    let mut drawdowns: Vec<f64> = Vec::new();
    let mut dd_over_10 = 0usize;
    let mut dd_over_20 = 0usize;
    let mut dd_over_30 = 0usize;

    for bar in bars {
        if bar.close > peak {
            peak = bar.close;
            peak_date = bar.date;
        }
        let dd = if peak > 0.0 {
            (bar.close - peak) / peak
        } else {
            0.0
        };
        drawdowns.push(dd);

        if dd < -0.10 {
            dd_over_10 += 1;
        }
        if dd < -0.20 {
            dd_over_20 += 1;
        }
        if dd < -0.30 {
            dd_over_30 += 1;
        }

        if dd < max_dd {
            max_dd = dd;
            max_dd_date = bar.date;
        }
    }

    let avg_dd = if !drawdowns.is_empty() {
        drawdowns.iter().sum::<f64>() / drawdowns.len() as f64
    } else {
        0.0
    };

    let total = bars.len() as f64;

    DrawdownProfile {
        dd_over_10_pct: dd_over_10 as f64 / total,
        dd_over_20_pct: dd_over_20 as f64 / total,
        dd_over_30_pct: dd_over_30 as f64 / total,
        avg_drawdown: avg_dd,
        max_drawdown: max_dd,
        max_drawdown_date: max_dd_date,
    }
}

/// Compute annualized return and volatility.
fn compute_annualized_metrics(bars: &[DailyBar]) -> (f64, f64) {
    if bars.len() < 2 {
        return (0.0, 0.0);
    }

    let total_return = if bars[0].close > 0.0 {
        (bars.last().unwrap().close - bars[0].close) / bars[0].close
    } else {
        0.0
    };

    let years = bars.len() as f64 / 252.0;
    let annualized_return = if years > 0.0 {
        (1.0 + total_return).powf(1.0 / years) - 1.0
    } else {
        0.0
    };

    let daily_returns: Vec<f64> = bars
        .windows(2)
        .map(|w| {
            if w[0].close > 0.0 {
                (w[1].close - w[0].close) / w[0].close
            } else {
                0.0
            }
        })
        .collect();

    let mean = daily_returns.iter().sum::<f64>() / daily_returns.len() as f64;
    let variance = daily_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / daily_returns.len() as f64;
    let daily_vol = variance.sqrt();
    let annualized_vol = daily_vol * (252.0_f64).sqrt();

    (annualized_return, annualized_vol)
}

/// Run full structural market audit.
pub fn audit_market_structure(
    bars: &[DailyBar],
    scope: &str,
    anchor_symbol: &str,
) -> MarketStructureReport {
    let window_from = bars.first().map(|b| b.date).unwrap_or(NaiveDate::MIN);
    let window_to = bars.last().map(|b| b.date).unwrap_or(NaiveDate::MAX);

    let monthly = monthly_returns(bars);
    let quarterly = quarterly_returns(&monthly);

    let up_months = monthly.iter().filter(|(_, r)| *r > 0.0).count();
    let down_months = monthly.iter().filter(|(_, r)| *r <= 0.0).count();
    let up_quarters = quarterly.iter().filter(|(_, r)| *r > 0.0).count();
    let down_quarters = quarterly.iter().filter(|(_, r)| *r <= 0.0).count();

    let total_months = monthly.len();
    let total_quarters = quarterly.len();

    let positive_month_ratio = if total_months > 0 {
        up_months as f64 / total_months as f64
    } else {
        0.0
    };

    let positive_quarter_ratio = if total_quarters > 0 {
        up_quarters as f64 / total_quarters as f64
    } else {
        0.0
    };

    let (annualized_return, annualized_volatility) = compute_annualized_metrics(bars);
    let drawdown_profile = compute_drawdown_profile(bars);

    MarketStructureReport {
        scope: scope.to_string(),
        anchor_symbol: anchor_symbol.to_string(),
        window_from,
        window_to,
        total_days: bars.len(),
        total_months,
        total_quarters,
        annualized_return,
        annualized_volatility,
        max_drawdown: drawdown_profile.max_drawdown,
        positive_month_ratio,
        positive_quarter_ratio,
        up_months,
        down_months,
        up_quarters,
        down_quarters,
        drawdown_profile,
        monthly_returns: monthly,
        quarterly_returns: quarterly,
    }
}

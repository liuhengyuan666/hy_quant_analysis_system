use chrono::NaiveDate;
use core_domain::{DailyBar, MarketRegimeSnapshot};

// ============================================================
// TASK-071B: State Lead/Lag Analysis
// For each episode, computes returns in three phases:
// - Before: 20d/60d before episode starts
// - During: returns during the episode
// - After: 20d/60d after episode ends
//
// This answers: Does RiskOff appear before, during, or after market stress?
// ============================================================

#[derive(Debug, Clone)]
pub struct EpisodeTiming {
    pub state: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub duration_days: usize,
    pub before_20d_return: f64,
    pub before_60d_return: f64,
    pub during_return: f64,
    pub after_20d_return: f64,
    pub after_60d_return: f64,
}

#[derive(Debug, Clone)]
pub struct LeadLagReport {
    pub market: String,
    pub episodes: Vec<EpisodeTiming>,
}

fn extract_episodes(regimes: &[MarketRegimeSnapshot]) -> Vec<(String, NaiveDate, NaiveDate, usize)> {
    if regimes.is_empty() {
        return Vec::new();
    }

    let mut episodes = Vec::new();
    let mut current_state = regimes[0].regime_label.clone();
    let mut start_date = regimes[0].date;
    let mut count = 1;

    for i in 1..regimes.len() {
        if regimes[i].regime_label == current_state {
            count += 1;
        } else {
            episodes.push((
                current_state.clone(),
                start_date,
                regimes[i - 1].date,
                count,
            ));
            current_state = regimes[i].regime_label.clone();
            start_date = regimes[i].date;
            count = 1;
        }
    }

    episodes.push((current_state, start_date, regimes.last().unwrap().date, count));
    episodes
}

fn get_return_before(
    bars: &[DailyBar],
    target_date: NaiveDate,
    days: usize,
) -> f64 {
    let idx = match bars.iter().position(|b| b.date == target_date) {
        Some(i) => i,
        None => return 0.0,
    };

    if idx < days {
        return 0.0;
    }

    let start_close = bars[idx - days].close;
    let end_close = bars[idx].close;
    (end_close - start_close) / start_close
}

fn get_return_after(
    bars: &[DailyBar],
    target_date: NaiveDate,
    days: usize,
) -> f64 {
    let idx = match bars.iter().position(|b| b.date == target_date) {
        Some(i) => i,
        None => return 0.0,
    };

    if idx + days >= bars.len() {
        return 0.0;
    }

    let start_close = bars[idx].close;
    let end_close = bars[idx + days].close;
    (end_close - start_close) / start_close
}

fn get_during_return(
    bars: &[DailyBar],
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> f64 {
    let start_idx = match bars.iter().position(|b| b.date == start_date) {
        Some(i) => i,
        None => return 0.0,
    };
    let end_idx = match bars.iter().position(|b| b.date == end_date) {
        Some(i) => i,
        None => return 0.0,
    };

    let start_close = bars[start_idx].close;
    let end_close = bars[end_idx].close;
    (end_close - start_close) / start_close
}

pub fn analyze_lead_lag(
    market: &str,
    regimes: &[MarketRegimeSnapshot],
    bars: &[DailyBar],
) -> LeadLagReport {
    let raw_episodes = extract_episodes(regimes);

    let mut episodes = Vec::new();
    for (state, start_date, end_date, duration) in raw_episodes {
        let before_20d = get_return_before(bars, start_date, 20);
        let before_60d = get_return_before(bars, start_date, 60);
        let during = get_during_return(bars, start_date, end_date);
        let after_20d = get_return_after(bars, end_date, 20);
        let after_60d = get_return_after(bars, end_date, 60);

        episodes.push(EpisodeTiming {
            state,
            start_date,
            end_date,
            duration_days: duration,
            before_20d_return: before_20d,
            before_60d_return: before_60d,
            during_return: during,
            after_20d_return: after_20d,
            after_60d_return: after_60d,
        });
    }

    LeadLagReport {
        market: market.to_string(),
        episodes,
    }
}

pub fn aggregate_by_state(episodes: &[EpisodeTiming]) -> Vec<StateLeadLagSummary> {
    let mut by_state: std::collections::HashMap<String, Vec<&EpisodeTiming>> = std::collections::HashMap::new();
    for ep in episodes {
        by_state.entry(ep.state.clone()).or_insert_with(Vec::new).push(ep);
    }

    let mut summaries = Vec::new();
    for (state, eps) in by_state {
        let n = eps.len() as f64;
        if n == 0.0 {
            continue;
        }

        let avg_before_20d = eps.iter().map(|e| e.before_20d_return).sum::<f64>() / n;
        let avg_before_60d = eps.iter().map(|e| e.before_60d_return).sum::<f64>() / n;
        let avg_during = eps.iter().map(|e| e.during_return).sum::<f64>() / n;
        let avg_after_20d = eps.iter().map(|e| e.after_20d_return).sum::<f64>() / n;
        let avg_after_60d = eps.iter().map(|e| e.after_60d_return).sum::<f64>() / n;
        let avg_duration = eps.iter().map(|e| e.duration_days as f64).sum::<f64>() / n;

        summaries.push(StateLeadLagSummary {
            state,
            episode_count: eps.len(),
            avg_duration_days: avg_duration,
            avg_before_20d,
            avg_before_60d,
            avg_during,
            avg_after_20d,
            avg_after_60d,
        });
    }

    summaries.sort_by(|a, b| a.state.cmp(&b.state));
    summaries
}

#[derive(Debug, Clone)]
pub struct StateLeadLagSummary {
    pub state: String,
    pub episode_count: usize,
    pub avg_duration_days: f64,
    pub avg_before_20d: f64,
    pub avg_before_60d: f64,
    pub avg_during: f64,
    pub avg_after_20d: f64,
    pub avg_after_60d: f64,
}

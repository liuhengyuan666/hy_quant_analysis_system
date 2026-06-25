use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use core_domain::{DailyBar, SignalLabel, SignalSnapshot, StrategyState, StrategyStateSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub strategy_name: String,
    pub initial_capital: f64,
    pub max_holdings: usize,
    pub fee_rate: f64,
    pub slippage_rate: f64,
    pub analysis_scope: String,
    pub signal_scope: String,
    pub regime_basis_scope: String,
    pub use_strategy_state: bool,
    pub drawdown_limit_pct: Option<f64>,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            strategy_name: "SIGNAL_PORTFOLIO_V1".to_string(),
            initial_capital: 1_000_000.0,
            max_holdings: 3,
            fee_rate: 0.001,
            slippage_rate: 0.0005,
            analysis_scope: "GLOBAL".to_string(),
            signal_scope: "GLOBAL".to_string(),
            regime_basis_scope: "GLOBAL".to_string(),
            use_strategy_state: false,
            drawdown_limit_pct: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestTrade {
    pub run_id: String,
    pub trade_date: NaiveDate,
    pub symbol: String,
    pub action: String,
    pub price: f64,
    pub quantity: f64,
    pub trade_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestEquityPoint {
    pub run_id: String,
    pub date: NaiveDate,
    pub equity: f64,
    pub drawdown: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestSummary {
    pub run_id: String,
    pub strategy_name: String,
    pub analysis_scope: String,
    pub signal_scope: String,
    pub regime_basis_scope: String,
    pub signal_start_date: Option<NaiveDate>,
    pub signal_end_date: Option<NaiveDate>,
    pub config_summary: String,
    pub cagr: f64,
    pub max_drawdown: f64,
    pub sharpe: f64,
    pub final_equity: f64,
    pub trades: usize,
    pub trading_days: usize,
    pub drawdown_events: usize,
    pub state_trajectory: Vec<(NaiveDate, String)>,
    #[serde(default = "default_run_version")]
    pub run_version: String,
    #[serde(default = "default_git_commit")]
    pub git_commit: String,
    #[serde(default = "default_generated_at")]
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub summary: BacktestSummary,
    pub trades: Vec<BacktestTrade>,
    pub equity_curve: Vec<BacktestEquityPoint>,
}

#[derive(Debug, Clone)]
struct Position {
    quantity: f64,
    last_price: f64,
}

fn default_run_version() -> String {
    "legacy".to_string()
}

fn default_git_commit() -> String {
    option_env!("BACKTEST_GIT_COMMIT")
        .unwrap_or("unknown")
        .to_string()
}

fn default_generated_at() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn annualized_cagr(initial: f64, final_equity: f64, trading_days: usize) -> f64 {
    if initial <= 0.0 || final_equity <= 0.0 || trading_days == 0 {
        return 0.0;
    }
    (final_equity / initial).powf(252.0 / trading_days as f64) - 1.0
}

fn max_drawdown(points: &[f64]) -> f64 {
    let mut peak = f64::NEG_INFINITY;
    let mut worst: f64 = 0.0;
    for equity in points {
        peak = peak.max(*equity);
        if peak > 0.0 {
            let drawdown = (equity / peak) - 1.0;
            worst = worst.min(drawdown);
        }
    }
    worst.abs()
}

fn sharpe_ratio(points: &[f64]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let returns = points
        .windows(2)
        .filter_map(|window| {
            let previous = window[0];
            let current = window[1];
            if previous <= 0.0 {
                None
            } else {
                Some((current / previous) - 1.0)
            }
        })
        .collect::<Vec<_>>();
    if returns.len() < 2 {
        return 0.0;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (returns.len() as f64 - 1.0);
    let stddev = variance.sqrt();
    if stddev <= f64::EPSILON {
        0.0
    } else {
        (mean / stddev) * 252.0_f64.sqrt()
    }
}

fn tradable_buy(label: &SignalLabel) -> bool {
    matches!(label, SignalLabel::StrongBuy | SignalLabel::Buy)
}

fn exit_signal(label: &SignalLabel) -> bool {
    matches!(label, SignalLabel::Reduce | SignalLabel::Sell)
}

fn format_config_summary(config: &BacktestConfig) -> String {
    let base = format!(
        "initial_capital={:.0}, max_holdings={}, fee_rate={:.4}, slippage_rate={:.4}",
        config.initial_capital, config.max_holdings, config.fee_rate, config.slippage_rate
    );
    if !config.use_strategy_state && config.drawdown_limit_pct.is_none() {
        return base;
    }
    format!(
        "{}, use_strategy_state={}, drawdown_limit_pct={}",
        base,
        config.use_strategy_state,
        config
            .drawdown_limit_pct
            .map(|value| format!("{value:.4}"))
            .unwrap_or_else(|| "none".to_string())
    )
}

fn state_limits(state: &StrategyState, max_holdings: usize) -> (usize, f64) {
    match state {
        StrategyState::NoTrade => (0, 0.0),
        StrategyState::LeftProbe => (1, 0.2),
        StrategyState::ConfirmAdd => (2, 0.6),
        StrategyState::FullTrend => (max_holdings, 1.0),
        StrategyState::DeRisk => (1, 0.3),
    }
}

fn position_value_at_open(
    positions: &BTreeMap<String, Position>,
    bar_lookup: &BTreeMap<String, BTreeMap<NaiveDate, DailyBar>>,
    trade_date: NaiveDate,
) -> f64 {
    positions
        .iter()
        .map(|(symbol, position)| {
            let price = bar_lookup
                .get(symbol)
                .and_then(|rows| rows.get(&trade_date))
                .map(|bar| bar.open)
                .unwrap_or(position.last_price);
            position.quantity * price
        })
        .sum()
}

fn sell_positions_at_open(
    run_id: &str,
    symbols: Vec<String>,
    trade_date: NaiveDate,
    positions: &mut BTreeMap<String, Position>,
    bar_lookup: &BTreeMap<String, BTreeMap<NaiveDate, DailyBar>>,
    config: &BacktestConfig,
    trades: &mut Vec<BacktestTrade>,
    cash: &mut f64,
) {
    for symbol in symbols {
        let Some(bar) = bar_lookup
            .get(&symbol)
            .and_then(|rows| rows.get(&trade_date))
        else {
            continue;
        };
        if let Some(position) = positions.remove(&symbol) {
            let price = bar.open * (1.0 - config.slippage_rate);
            let gross = position.quantity * price;
            let fee = gross * config.fee_rate;
            *cash += gross - fee;
            trades.push(BacktestTrade {
                run_id: run_id.to_string(),
                trade_date,
                symbol,
                action: "SELL".to_string(),
                price,
                quantity: position.quantity,
                trade_value: gross,
            });
        }
    }
}

pub fn run_signal_backtest(
    run_id: &str,
    config: &BacktestConfig,
    signals: &[SignalSnapshot],
    bars_by_symbol: &BTreeMap<String, Vec<DailyBar>>,
    strategy_states: &[StrategyStateSnapshot],
) -> BacktestResult {
    let mut signal_dates = signals
        .iter()
        .map(|row| row.date)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    signal_dates.sort();

    let signal_by_date = signals.iter().fold(
        BTreeMap::<NaiveDate, Vec<&SignalSnapshot>>::new(),
        |mut acc, row| {
            acc.entry(row.date).or_default().push(row);
            acc
        },
    );

    let bar_lookup = bars_by_symbol
        .iter()
        .map(|(symbol, bars)| {
            let map = bars
                .iter()
                .map(|bar| (bar.date, bar.clone()))
                .collect::<BTreeMap<_, _>>();
            (symbol.clone(), map)
        })
        .collect::<BTreeMap<_, _>>();

    let mut cash = config.initial_capital;
    let mut positions = BTreeMap::<String, Position>::new();
    let mut trades = Vec::new();
    let mut equity_curve = Vec::new();
    let mut drawdown_events = 0;
    let mut drawdown_triggered = false;
    let mut liquidate_next_open = false;
    let mut peak_equity = config.initial_capital;
    let mut state_trajectory = Vec::new();
    let strategy_state_by_date = strategy_states
        .iter()
        .map(|row| (row.date, row))
        .collect::<BTreeMap<_, _>>();

    if signal_dates.is_empty() {
        return BacktestResult {
            summary: BacktestSummary {
                run_id: run_id.to_string(),
                strategy_name: config.strategy_name.clone(),
                analysis_scope: config.analysis_scope.clone(),
                signal_scope: config.signal_scope.clone(),
                regime_basis_scope: config.regime_basis_scope.clone(),
                signal_start_date: None,
                signal_end_date: None,
                config_summary: format_config_summary(config),
                cagr: 0.0,
                max_drawdown: 0.0,
                sharpe: 0.0,
                final_equity: config.initial_capital,
                trades: 0,
                trading_days: 0,
                drawdown_events,
                state_trajectory,
                run_version: "v1".to_string(),
                git_commit: option_env!("BACKTEST_GIT_COMMIT").unwrap_or("unknown").to_string(),
                generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            },
            trades,
            equity_curve,
        };
    }

    equity_curve.push(BacktestEquityPoint {
        run_id: run_id.to_string(),
        date: signal_dates[0],
        equity: cash,
        drawdown: 0.0,
    });

    for index in 1..signal_dates.len() {
        let decision_date = signal_dates[index - 1];
        let trade_date = signal_dates[index];
        let decision_signals = signal_by_date
            .get(&decision_date)
            .cloned()
            .unwrap_or_default();

        if liquidate_next_open {
            let symbols = positions.keys().cloned().collect::<Vec<_>>();
            sell_positions_at_open(
                run_id,
                symbols,
                trade_date,
                &mut positions,
                &bar_lookup,
                config,
                &mut trades,
                &mut cash,
            );
            liquidate_next_open = false;
        }

        let mut effective_max_holdings = config.max_holdings;
        let mut capital_multiplier = 1.0;
        if config.use_strategy_state {
            if let Some(state) = strategy_state_by_date
                .range(..=decision_date)
                .next_back()
                .map(|(_, row)| *row)
            {
                let limits = state_limits(&state.state, config.max_holdings);
                effective_max_holdings = limits.0;
                capital_multiplier = limits.1;
                state_trajectory.push((decision_date, state.state.as_str().to_string()));
            }
        }

        let signal_map = decision_signals
            .iter()
            .map(|row| (row.symbol.clone(), *row))
            .collect::<BTreeMap<_, _>>();

        let held_symbols = positions.keys().cloned().collect::<Vec<_>>();
        for symbol in held_symbols {
            let Some(bar) = bar_lookup
                .get(&symbol)
                .and_then(|rows| rows.get(&trade_date))
            else {
                continue;
            };
            if let Some(signal) = signal_map.get(&symbol) {
                if exit_signal(&signal.signal_label) {
                    if let Some(position) = positions.remove(&symbol) {
                        let price = bar.open * (1.0 - config.slippage_rate);
                        let gross = position.quantity * price;
                        let fee = gross * config.fee_rate;
                        cash += gross - fee;
                        trades.push(BacktestTrade {
                            run_id: run_id.to_string(),
                            trade_date,
                            symbol: symbol.clone(),
                            action: "SELL".to_string(),
                            price,
                            quantity: position.quantity,
                            trade_value: gross,
                        });
                    }
                }
            }
        }

        let slots = effective_max_holdings.saturating_sub(positions.len());
        if slots > 0 && !drawdown_triggered {
            let mut candidates = decision_signals
                .into_iter()
                .filter(|row| {
                    tradable_buy(&row.signal_label) && !positions.contains_key(&row.symbol)
                })
                .collect::<Vec<_>>();
            candidates.sort_by(|left, right| {
                right
                    .final_score
                    .total_cmp(&left.final_score)
                    .then_with(|| left.symbol.cmp(&right.symbol))
            });

            let mut remaining_slots = slots;
            let available_capital = config.initial_capital * capital_multiplier;
            let invested_at_open = position_value_at_open(&positions, &bar_lookup, trade_date);
            let mut buy_cash = cash.min((available_capital - invested_at_open).max(0.0));
            for signal in candidates.into_iter().take(slots) {
                if remaining_slots == 0 {
                    break;
                }
                let Some(bar) = bar_lookup
                    .get(&signal.symbol)
                    .and_then(|rows| rows.get(&trade_date))
                else {
                    continue;
                };
                let price = bar.open * (1.0 + config.slippage_rate);
                if price <= 0.0 {
                    continue;
                }
                let budget = buy_cash / remaining_slots as f64;
                let quantity = (budget / (price * (1.0 + config.fee_rate))).floor();
                if quantity < 1.0 {
                    continue;
                }
                let gross = quantity * price;
                let fee = gross * config.fee_rate;
                let total = gross + fee;
                if total > cash {
                    continue;
                }
                cash -= total;
                buy_cash = (buy_cash - total).max(0.0);
                positions.insert(
                    signal.symbol.clone(),
                    Position {
                        quantity,
                        last_price: bar.close,
                    },
                );
                trades.push(BacktestTrade {
                    run_id: run_id.to_string(),
                    trade_date,
                    symbol: signal.symbol.clone(),
                    action: "BUY".to_string(),
                    price,
                    quantity,
                    trade_value: gross,
                });
                remaining_slots = remaining_slots.saturating_sub(1);
            }
        }

        let mut equity = cash;
        for (symbol, position) in &mut positions {
            if let Some(bar) = bar_lookup
                .get(symbol)
                .and_then(|rows| rows.get(&trade_date))
            {
                position.last_price = bar.close;
            }
            equity += position.quantity * position.last_price;
        }
        peak_equity = peak_equity.max(equity);
        let drawdown = if peak_equity <= 0.0 {
            0.0
        } else {
            (peak_equity - equity) / peak_equity
        };
        if let Some(limit) = config.drawdown_limit_pct {
            if drawdown_triggered && drawdown < limit * 0.5 {
                drawdown_triggered = false;
            }
            if !drawdown_triggered && drawdown > limit {
                drawdown_events += 1;
                drawdown_triggered = true;
                liquidate_next_open = true;
            }
        }
        equity_curve.push(BacktestEquityPoint {
            run_id: run_id.to_string(),
            date: trade_date,
            equity,
            drawdown,
        });
    }

    let equities = equity_curve
        .iter()
        .map(|point| point.equity)
        .collect::<Vec<_>>();
    let mdd = max_drawdown(&equities);
    let final_equity = equities.last().copied().unwrap_or(config.initial_capital);
    let cagr = annualized_cagr(
        config.initial_capital,
        final_equity,
        equities.len().saturating_sub(1),
    );
    let sharpe = sharpe_ratio(&equities);

    let mut peak = f64::NEG_INFINITY;
    for point in &mut equity_curve {
        peak = peak.max(point.equity);
        point.drawdown = if peak <= 0.0 {
            0.0
        } else {
            ((point.equity / peak) - 1.0).abs()
        };
    }

    BacktestResult {
        summary: BacktestSummary {
            run_id: run_id.to_string(),
            strategy_name: config.strategy_name.clone(),
            analysis_scope: config.analysis_scope.clone(),
            signal_scope: config.signal_scope.clone(),
            regime_basis_scope: config.regime_basis_scope.clone(),
            signal_start_date: signal_dates.first().copied(),
            signal_end_date: signal_dates.last().copied(),
            config_summary: format_config_summary(config),
            cagr,
            max_drawdown: mdd,
            sharpe,
            final_equity,
            trades: trades.len(),
            trading_days: equities.len().saturating_sub(1),
            drawdown_events,
            state_trajectory,
            run_version: "v1".to_string(),
            git_commit: option_env!("BACKTEST_GIT_COMMIT").unwrap_or("unknown").to_string(),
            generated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        },
        trades,
        equity_curve,
    }
}

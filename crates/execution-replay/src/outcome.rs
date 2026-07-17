use anyhow::{Context, Result};
use chrono::NaiveDate;
use market_store::{fetch_daily_bars_for_symbols_in_range, StorageConfig};

use execution_engine::v2::event::ExecutionEvent;

use crate::{ExecutionOutcome, ReplayOutcomeResolver};

/// Outcome resolver backed by `market-store` daily bars.
///
/// Computes forward returns (T+20, T+60, T+120), MFE, MAE, and maximum drawdown
/// using daily OHLCV data. Holding periods are measured in calendar days, not
/// trading days, because this resolver does not require a calendar service. A
/// production resolver may use `TradingCalendar` to convert horizons to trading
/// days.
#[derive(Debug, Clone)]
pub struct MarketStoreOutcomeResolver {
    config: StorageConfig,
}

impl MarketStoreOutcomeResolver {
    pub fn new(config: StorageConfig) -> Self {
        Self { config }
    }
}

impl ReplayOutcomeResolver for MarketStoreOutcomeResolver {
    fn resolve(&self, event: &ExecutionEvent, _as_of: NaiveDate) -> Result<ExecutionOutcome> {
        let symbol = event.symbol().to_string();
        let entry_date = event.date();
        let entry_price = event.request.quote.close;

        // We need bars strictly after the entry date up to a reasonable horizon.
        // Using 180 calendar days to cover T+120 even with weekends/holidays.
        let end_date = entry_date
            .checked_add_signed(chrono::Duration::days(180))
            .context("failed to compute end date")?;

        let bars = fetch_daily_bars_for_symbols_in_range(
            &self.config,
            &[symbol],
            entry_date,
            end_date,
        )
        .context("failed to fetch daily bars for outcome")?;

        // Filter bars after entry date and sort by date.
        let mut forward_bars: Vec<_> = bars
            .into_iter()
            .filter(|bar| bar.date > entry_date)
            .collect();
        forward_bars.sort_by(|a, b| a.date.cmp(&b.date));

        if forward_bars.is_empty() {
            return Ok(ExecutionOutcome::default());
        }

        let mut max_gain = 0.0_f64;
        let mut max_loss = 0.0_f64;
        let mut peak_price = entry_price;
        let mut max_drawdown = 0.0_f64;

        for (idx, bar) in forward_bars.iter().enumerate() {
            let day = (idx + 1) as u32;
            let high_ret = (bar.high - entry_price) / entry_price;
            let low_ret = (bar.low - entry_price) / entry_price;

            if high_ret > max_gain {
                max_gain = high_ret;
            }
            if low_ret < max_loss {
                max_loss = low_ret;
            }

            // Drawdown from peak close (using close for simplicity).
            if bar.close > peak_price {
                peak_price = bar.close;
            }
            let dd = (bar.close - peak_price) / peak_price;
            if dd < max_drawdown {
                max_drawdown = dd;
            }

            // Stop/take-profit heuristic: hit if MFE/MAE crosses a 5% threshold.
            // This is a placeholder; a real policy would use ExecutionPolicy.
            let _ = day;
        }

        let t20_return = forward_bars.get(19).map(|b| (b.close - entry_price) / entry_price);
        let t60_return = forward_bars.get(59).map(|b| (b.close - entry_price) / entry_price);
        let t120_return = forward_bars.get(119).map(|b| (b.close - entry_price) / entry_price);

        let holding_days = Some(forward_bars.len() as u32);

        Ok(ExecutionOutcome {
            t20_return,
            t60_return,
            t120_return,
            mfe: Some(max_gain),
            mae: Some(max_loss),
            max_drawdown: Some(max_drawdown),
            holding_days,
            benchmark_return: None, // TODO: fetch benchmark return for the same horizon
            alpha: None,          // alpha = return - benchmark_return
            stop_loss_hit: Some(max_loss <= -0.05),
            take_profit_hit: Some(max_gain >= 0.10),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use execution_engine::v2::request::{ExecutionPolicy, ExecutionRequest, QuoteSnapshot};
    use execution_engine::v2::{DefaultExecutionPipeline, ExecutionPipeline};

    fn make_request(close: f64) -> ExecutionRequest {
        use chrono::Utc;
        use core_domain::{SignalLabel, StrategyKind, StrategyState};
        use research_context::{
            BreadthSummary, ConfirmationDimension, ConfirmationSummary, RecoverySummary,
        };

        ExecutionRequest {
            symbol: "000001".into(),
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            signal: core_domain::SignalSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                symbol: "000001".into(),
                final_score: 85.0,
                signal_label: SignalLabel::StrongBuy,
                analysis_scope: "CN".into(),
                regime_basis_scope: "CN".into(),
                reason: core_domain::SignalReason {
                    best_strategy: StrategyKind::MomentumRight,
                    strategy_score: 0.0,
                    strategy_contribution: 0.0,
                    alignment: 0,
                    aligned_strategies: vec![],
                    alignment_contribution: 0.0,
                    regime: core_domain::RegimeReason {
                        trend_score: 0.0,
                        risk_score: 0.0,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    rotation: core_domain::RotationReason {
                        momentum_score: 0.0,
                        rank: None,
                        combined_score: 0.0,
                        contribution: 0.0,
                    },
                    final_score: 85.0,
                    label: SignalLabel::StrongBuy,
                    summary: "test".into(),
                },
            },
            strategy_state: core_domain::StrategyStateSnapshot {
                date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                scope: "CN".into(),
                state: StrategyState::FullTrend,
                state_score: 75.0,
                transition_reason: "test".into(),
                recommended_position_pct: 100.0,
            },
            quote: QuoteSnapshot {
                symbol: "000001".into(),
                ts: Utc::now(),
                open: close * 0.99,
                high: close * 1.01,
                low: close * 0.98,
                close,
                volume: 1_000_000.0,
                prev_close: close * 0.98,
            },
            volume_ma20: 500_000.0,
            market_view: execution_engine::v2::request::ExecutionMarketView {
                research_version: "1".into(),
                market_regime_label: "Bullish".into(),
                confirmation: ConfirmationSummary {
                    trend: ConfirmationDimension { score: 70.0, label: "Strong".into() },
                    participation: ConfirmationDimension { score: 60.0, label: "Moderate".into() },
                    risk: ConfirmationDimension { score: 35.0, label: "Low".into() },
                    overall: "Strong".into(),
                },
                breadth: BreadthSummary { breadth_pct: 60.0, sma5: None, delta_5d: Some(0.0), condition: "strong".into() },
                recovery: RecoverySummary { score: 60.0, drivers: vec![] },
                rotation_state: "broad".into(),
                leadership_stability: 0.7,
            },
            policy: ExecutionPolicy::default(),
        }
    }

    #[test]
    fn market_store_resolver_trait_compiles() {
        // This test only verifies that the resolver can be constructed and the
        // trait interface is satisfied. Full integration tests require a live
        // ClickHouse instance with seeded bars.
        let config = StorageConfig::default();
        let _resolver = MarketStoreOutcomeResolver::new(config);
    }

    #[test]
    fn event_produces_decision() {
        let request = make_request(100.0);
        let event = DefaultExecutionPipeline.execute(request);
        assert!(!event.execution_id.is_empty());
        assert!(event.decision.confidence > 0.0);
    }
}

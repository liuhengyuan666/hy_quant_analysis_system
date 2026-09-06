//! Signal-related research domain helpers.
//!
//! Reserved for future domain extraction. When pure signal-domain computations
//! (e.g. divergence detection, signal concentration metrics) become shared across
//! consumers, they belong here rather than in Builder, CLI, or AppService.

use crate::DailyBar;
use chrono::NaiveDate;

/// Forward-return facts at the maturity bar of a trading-bar horizon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradingBarForwardReturn {
    pub maturity_date: NaiveDate,
    pub maturity_close: f64,
    pub forward_return: f64,
}

/// Compute T+N maturity from the Nth persisted bar strictly after an observation.
///
/// Input order does not matter. Bars for other symbols are ignored, and the
/// observation must have an exact persisted bar. Returns `None` when `horizon`
/// is zero, prices are non-finite or non-positive, or maturity is unavailable.
pub fn trading_bar_forward_return(
    bars: &[DailyBar],
    symbol: &str,
    observation_date: NaiveDate,
    horizon: usize,
) -> Option<TradingBarForwardReturn> {
    if horizon == 0 {
        return None;
    }

    let observation_close = bars
        .iter()
        .find(|bar| bar.symbol == symbol && bar.date == observation_date)?
        .close;
    if !valid_price(observation_close) {
        return None;
    }

    let mut subsequent_bars = bars
        .iter()
        .filter(|bar| bar.symbol == symbol && bar.date > observation_date)
        .collect::<Vec<_>>();
    subsequent_bars.sort_by_key(|bar| bar.date);

    let maturity_bar = subsequent_bars.get(horizon - 1)?;
    if !valid_price(maturity_bar.close) {
        return None;
    }

    Some(TradingBarForwardReturn {
        maturity_date: maturity_bar.date,
        maturity_close: maturity_bar.close,
        forward_return: (maturity_bar.close - observation_close) / observation_close,
    })
}

fn valid_price(price: f64) -> bool {
    price.is_finite() && price > 0.0
}

#[cfg(test)]
mod tests {
    use super::{trading_bar_forward_return, TradingBarForwardReturn};
    use crate::DailyBar;
    use chrono::{Days, NaiveDate};

    fn date(day: u64) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .checked_add_days(Days::new(day))
            .unwrap()
    }

    fn bar(symbol: &str, day: u64, close: f64) -> DailyBar {
        DailyBar {
            date: date(day),
            symbol: symbol.to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
            turnover: None,
        }
    }

    #[test]
    fn uses_twentieth_strictly_subsequent_bar_for_t20() {
        let bars = (0..=20)
            .map(|day| bar("000300", day, 100.0 + day as f64))
            .collect::<Vec<_>>();

        let result = trading_bar_forward_return(&bars, "000300", date(0), 20);

        assert_eq!(
            result,
            Some(TradingBarForwardReturn {
                maturity_date: date(20),
                maturity_close: 120.0,
                forward_return: 0.2,
            })
        );
    }

    #[test]
    fn returns_none_when_subsequent_bars_are_insufficient() {
        let bars = (0..20)
            .map(|day| bar("000300", day, 100.0 + day as f64))
            .collect::<Vec<_>>();

        assert_eq!(
            trading_bar_forward_return(&bars, "000300", date(0), 20),
            None
        );
    }

    #[test]
    fn sorts_unordered_input_by_persisted_bar_date() {
        let bars = vec![
            bar("000300", 3, 130.0),
            bar("000300", 0, 100.0),
            bar("000300", 2, 120.0),
            bar("000300", 1, 110.0),
        ];

        assert_eq!(
            trading_bar_forward_return(&bars, "000300", date(0), 2),
            Some(TradingBarForwardReturn {
                maturity_date: date(2),
                maturity_close: 120.0,
                forward_return: 0.2,
            })
        );
    }

    #[test]
    fn excludes_observation_date_from_maturity_count() {
        let bars = vec![
            bar("000300", 0, 100.0),
            bar("000300", 1, 110.0),
            bar("000300", 2, 120.0),
        ];

        let result = trading_bar_forward_return(&bars, "000300", date(0), 1).unwrap();

        assert_eq!(result.maturity_date, date(1));
        assert_eq!(result.maturity_close, 110.0);
    }

    #[test]
    fn returns_none_without_exact_observation_bar() {
        let bars = vec![bar("000300", 1, 110.0), bar("000300", 2, 120.0)];

        assert_eq!(
            trading_bar_forward_return(&bars, "000300", date(0), 1),
            None
        );
    }

    #[test]
    fn returns_none_for_invalid_observation_or_maturity_price() {
        for invalid_close in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let invalid_observation =
                vec![bar("000300", 0, invalid_close), bar("000300", 1, 110.0)];
            let invalid_maturity = vec![bar("000300", 0, 100.0), bar("000300", 1, invalid_close)];

            assert_eq!(
                trading_bar_forward_return(&invalid_observation, "000300", date(0), 1),
                None
            );
            assert_eq!(
                trading_bar_forward_return(&invalid_maturity, "000300", date(0), 1),
                None
            );
        }
    }

    #[test]
    fn ignores_bars_from_other_symbols() {
        let bars = vec![
            bar("000300", 0, 100.0),
            bar("HSCEI", 1, 500.0),
            bar("000300", 2, 120.0),
            bar("HSCEI", 3, 600.0),
            bar("000300", 4, 140.0),
        ];

        assert_eq!(
            trading_bar_forward_return(&bars, "000300", date(0), 2),
            Some(TradingBarForwardReturn {
                maturity_date: date(4),
                maturity_close: 140.0,
                forward_return: 0.4,
            })
        );
    }
}

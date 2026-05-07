use chrono::Datelike;
use chrono::NaiveDate;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct TradingCalendar {
    cn_holidays: HashSet<NaiveDate>,
    hk_holidays: HashSet<NaiveDate>,
}

impl Default for TradingCalendar {
    fn default() -> Self {
        Self {
            cn_holidays: HashSet::new(),
            hk_holidays: HashSet::new(),
        }
    }
}

impl TradingCalendar {
    pub fn new(cn_holidays: HashSet<NaiveDate>, hk_holidays: HashSet<NaiveDate>) -> Self {
        Self {
            cn_holidays,
            hk_holidays,
        }
    }

    pub fn is_trading_day(&self, market: &super::Market, date: NaiveDate) -> bool {
        let holidays = match market {
            super::Market::Cn => &self.cn_holidays,
            super::Market::Hk => &self.hk_holidays,
        };
        !holidays.contains(&date) && !Self::is_weekend(date)
    }

    pub fn next_trading_day(&self, market: &super::Market, date: NaiveDate) -> Option<NaiveDate> {
        let mut candidate = date + chrono::Duration::days(1);
        for _ in 0..30 {
            if self.is_trading_day(market, candidate) {
                return Some(candidate);
            }
            candidate += chrono::Duration::days(1);
        }
        None
    }

    pub fn prev_trading_day(&self, market: &super::Market, date: NaiveDate) -> Option<NaiveDate> {
        let mut candidate = date - chrono::Duration::days(1);
        for _ in 0..30 {
            if self.is_trading_day(market, candidate) {
                return Some(candidate);
            }
            candidate -= chrono::Duration::days(1);
        }
        None
    }

    pub fn trading_days_between(
        &self,
        market: &super::Market,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Vec<NaiveDate> {
        let mut days = Vec::new();
        let mut current = from;
        while current <= to {
            if self.is_trading_day(market, current) {
                days.push(current);
            }
            current += chrono::Duration::days(1);
        }
        days
    }

    pub fn non_trading_symbols_count(
        &self,
        date: NaiveDate,
        instruments: &[super::Instrument],
    ) -> usize {
        instruments
            .iter()
            .filter(|i| !self.is_trading_day(&i.market, date))
            .count()
    }

    pub fn trading_symbols<'a>(
        &'a self,
        date: NaiveDate,
        instruments: &'a [super::Instrument],
    ) -> impl Iterator<Item = &'a super::Instrument> + 'a {
        instruments
            .iter()
            .filter(move |i| self.is_trading_day(&i.market, date))
    }

    fn is_weekend(date: NaiveDate) -> bool {
        matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_calendar() -> TradingCalendar {
        let mut cn = HashSet::new();
        cn.insert(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap());
        cn.insert(NaiveDate::from_ymd_opt(2026, 5, 2).unwrap());
        cn.insert(NaiveDate::from_ymd_opt(2026, 5, 3).unwrap());

        let mut hk = HashSet::new();
        hk.insert(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap());

        TradingCalendar {
            cn_holidays: cn,
            hk_holidays: hk,
        }
    }

    #[test]
    fn weekend_is_not_trading_day() {
        let cal = test_calendar();
        let sat = NaiveDate::from_ymd_opt(2026, 5, 2).unwrap();
        assert!(!cal.is_trading_day(&super::super::Market::Cn, sat));
        assert!(!cal.is_trading_day(&super::super::Market::Hk, sat));
    }

    #[test]
    fn cn_labor_day_is_not_trading() {
        let cal = test_calendar();
        let labor = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        assert!(!cal.is_trading_day(&super::super::Market::Cn, labor));
        assert!(!cal.is_trading_day(&super::super::Market::Hk, labor));
    }

    #[test]
    fn normal_weekday_is_trading() {
        let cal = test_calendar();
        let normal = NaiveDate::from_ymd_opt(2026, 5, 6).unwrap();
        assert!(cal.is_trading_day(&super::super::Market::Cn, normal));
        assert!(cal.is_trading_day(&super::super::Market::Hk, normal));
    }

    #[test]
    fn trading_days_between_counts_correctly() {
        let cal = test_calendar();
        let from = NaiveDate::from_ymd_opt(2026, 4, 27).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 5, 6).unwrap();
        let cn_days = cal.trading_days_between(&super::super::Market::Cn, from, to);
        assert_eq!(cn_days.len(), 4);
    }

    #[test]
    fn next_trading_day_skips_holidays() {
        let cal = test_calendar();
        let labor = NaiveDate::from_ymd_opt(2026, 5, 1).unwrap();
        let next = cal.next_trading_day(&super::super::Market::Cn, labor);
        assert_eq!(next, Some(NaiveDate::from_ymd_opt(2026, 5, 6).unwrap()));
    }
}

use core_domain::AnalysisScope as ReportScope;

use crate::{TrackedInstrumentSeries, TrackedUniverseWindow};

pub(crate) fn dashboard_latest_date(
    available_dates: &[chrono::NaiveDate],
    cutoff: Option<chrono::NaiveDate>,
) -> Option<chrono::NaiveDate> {
    available_dates
        .iter()
        .copied()
        .filter(|date| cutoff.is_none_or(|cutoff| *date <= cutoff))
        .max()
}

pub(crate) fn dashboard_snapshot_dates(
    available_dates: &[chrono::NaiveDate],
    requested_report_date: Option<chrono::NaiveDate>,
) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    let latest_available_date = available_dates.iter().copied().max()?;
    let report_date = match requested_report_date {
        Some(date) if available_dates.contains(&date) => date,
        Some(_) => return None,
        None => latest_available_date,
    };
    Some((report_date, latest_available_date))
}

pub(crate) fn series_for_scope(
    window: &TrackedUniverseWindow,
    scope: ReportScope,
) -> Vec<TrackedInstrumentSeries> {
    match scope {
        ReportScope::Global => window
            .cn_series
            .iter()
            .chain(window.hk_series.iter())
            .cloned()
            .collect(),
        ReportScope::Cn => window.cn_series.clone(),
        ReportScope::Hk => window.hk_series.clone(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn dashboard_latest_date_respects_explicit_cutoff() {
        let dates = [
            NaiveDate::from_ymd_opt(2026, 3, 27).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 4).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 30).unwrap(),
        ];
        let cutoff = NaiveDate::from_ymd_opt(2026, 3, 30).unwrap();

        assert_eq!(
            dashboard_latest_date(&dates, Some(cutoff)),
            Some(cutoff),
            "historical dashboard diagnostics must not select a date after the cutoff"
        );
    }

    #[test]
    fn dashboard_latest_date_preserves_latest_selection_without_cutoff() {
        let latest = NaiveDate::from_ymd_opt(2026, 9, 4).unwrap();
        let dates = [latest, NaiveDate::from_ymd_opt(2026, 3, 30).unwrap()];

        assert_eq!(dashboard_latest_date(&dates, None), Some(latest));
    }

    #[test]
    fn dashboard_snapshot_dates_keep_true_latest_for_historical_selection() {
        let historical = NaiveDate::from_ymd_opt(2026, 3, 30).unwrap();
        let latest = NaiveDate::from_ymd_opt(2026, 9, 4).unwrap();
        let dates = [
            NaiveDate::from_ymd_opt(2026, 3, 27).unwrap(),
            latest,
            historical,
        ];

        assert_eq!(
            dashboard_snapshot_dates(&dates, Some(historical)),
            Some((historical, latest))
        );
    }

    #[test]
    fn dashboard_snapshot_dates_reject_unavailable_requested_date() {
        let dates = [NaiveDate::from_ymd_opt(2026, 9, 4).unwrap()];
        let unavailable = NaiveDate::from_ymd_opt(2026, 3, 30).unwrap();

        assert_eq!(dashboard_snapshot_dates(&dates, Some(unavailable)), None);
    }
}

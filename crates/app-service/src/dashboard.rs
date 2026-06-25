use core_domain::AnalysisScope as ReportScope;

use crate::{TrackedInstrumentSeries, TrackedUniverseWindow};

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

//! Similarity matching engine.
//!
//! Given a target fingerprint and a historical window of normalized feature vectors,
//! finds the most similar historical dates and computes aggregate search statistics.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::distance::DistanceMetric;
use crate::fingerprint::MarketFingerprint;
use crate::normalize::FeatureVector;
use crate::outcome::OutcomeProfile;

/// Qualitative similarity level derived from distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchLevel {
    /// Distance <= 0.1 — exceptionally similar market conditions.
    VeryHigh,
    /// Distance <= 0.2 — strongly similar market conditions.
    High,
    /// Distance <= 0.35 — moderately similar market conditions.
    Moderate,
    /// Distance > 0.35 — weakly similar.
    Weak,
}

/// A single historical match: a market date whose conditions were similar
/// to the target date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalMatch {
    pub date: NaiveDate,
    pub level: MatchLevel,
}

/// Result of a similarity search, including aggregate statistics and
/// optional forward-outcome profiling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Total number of historical days searched (excluding target).
    pub searched_days: usize,
    /// Number of days whose distance fell within the filter threshold (<= 0.35).
    pub filtered_days: usize,
    /// Average distance across all searched days.
    pub average_distance: f64,
    /// Top-N historical matches, sorted by similarity (ascending distance).
    pub matches: Vec<HistoricalMatch>,
    /// Optional forward-outcome profile for the matched dates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<OutcomeProfile>,
}

/// A configurable similarity matcher parameterized by a distance metric.
pub struct SimilarityMatcher<D: DistanceMetric> {
    distance: D,
}

impl<D: DistanceMetric> SimilarityMatcher<D> {
    /// Create a new matcher with the given distance metric.
    pub fn new(distance: D) -> Self {
        Self { distance }
    }

    /// Find the `top_n` most similar historical dates to the target.
    ///
    /// The target is identified by `target_index` into the `fingerprints`
    /// and `normalized` slices (which must be parallel arrays of the same length).
    ///
    /// Matches are sorted by ascending distance (most similar first).
    pub fn find_similar(
        &self,
        target_index: usize,
        fingerprints: &[MarketFingerprint],
        normalized: &[FeatureVector],
        top_n: usize,
    ) -> Vec<HistoricalMatch> {
        if target_index >= normalized.len() || normalized.len() < 2 {
            return Vec::new();
        }

        let target_fv = &normalized[target_index];
        let mut indexed_distances: Vec<(usize, f64)> = Vec::with_capacity(normalized.len());

        for (i, fv) in normalized.iter().enumerate() {
            if i == target_index {
                continue;
            }
            let d = self.distance.distance(target_fv, fv);
            indexed_distances.push((i, d));
        }

        // Sort by distance ascending (most similar first)
        indexed_distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        indexed_distances
            .into_iter()
            .take(top_n)
            .map(|(idx, dist)| {
                let level = match dist {
                    d if d <= 0.1 => MatchLevel::VeryHigh,
                    d if d <= 0.2 => MatchLevel::High,
                    d if d <= 0.35 => MatchLevel::Moderate,
                    _ => MatchLevel::Weak,
                };
                HistoricalMatch {
                    date: fingerprints[idx].date,
                    level,
                }
            })
            .collect()
    }

    /// Perform a full search and return aggregate statistics.
    ///
    /// `searched_days` is the total number of historical days compared against.
    /// `filtered_days` is the count of days with distance <= 0.35 (the "meaningful" band).
    /// `average_distance` is the mean distance across all searched days.
    pub fn search(
        &self,
        target_index: usize,
        fingerprints: &[MarketFingerprint],
        normalized: &[FeatureVector],
        top_n: usize,
    ) -> SearchResult {
        let matches = self.find_similar(target_index, fingerprints, normalized, top_n);

        let searched_days = if normalized.len() > 1 {
            normalized.len() - 1
        } else {
            0
        };

        let target_fv = if target_index < normalized.len() {
            Some(&normalized[target_index])
        } else {
            None
        };

        let mut total_distance: f64 = 0.0;
        let mut filtered_count: usize = 0;

        if let Some(tfv) = target_fv {
            for (i, fv) in normalized.iter().enumerate() {
                if i == target_index {
                    continue;
                }
                let d = self.distance.distance(tfv, fv);
                total_distance += d;
                if d <= 0.35 {
                    filtered_count += 1;
                }
            }
        }

        let average_distance = if searched_days > 0 {
            total_distance / searched_days as f64
        } else {
            0.0
        };

        SearchResult {
            searched_days,
            filtered_days: filtered_count,
            average_distance,
            matches,
            outcome: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distance::CosineDistance;
    use crate::fingerprint::{EvolutionVector, ObservationVector};
    use core_domain::AnalysisScope;

    fn make_fps(count: usize) -> Vec<MarketFingerprint> {
        (0..count)
            .map(|i| MarketFingerprint {
                scope: AnalysisScope::Global,
                date: NaiveDate::from_ymd_opt(2026, 1, (i + 1) as u32).unwrap(),
                observation: ObservationVector {
                    environment: i as f64 * 10.0,
                    signal: i as f64 * 5.0,
                    stretch: 50.0,
                    rotation: vec![("A".to_string(), i as f64 * 8.0)],
                },
                evolution: EvolutionVector {
                    confirmation: i as f64 * 7.0,
                    recovery: i as f64 * 3.0,
                },
            })
            .collect()
    }

    #[test]
    fn find_similar_returns_top_n() {
        let fps = make_fps(10);
        let normalized = crate::normalize::normalize_all(&fps);
        let matcher = SimilarityMatcher::new(CosineDistance);

        let matches = matcher.find_similar(5, &fps, &normalized, 3);
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn search_computes_statistics() {
        let fps = make_fps(10);
        let normalized = crate::normalize::normalize_all(&fps);
        let matcher = SimilarityMatcher::new(CosineDistance);

        let result = matcher.search(5, &fps, &normalized, 3);
        assert_eq!(result.searched_days, 9);
        assert_eq!(result.matches.len(), 3);
        // At least average_distance should be sensible
        assert!(result.average_distance >= 0.0);
        assert!(result.average_distance <= 1.0);
    }

    #[test]
    fn search_empty_handles_gracefully() {
        let fps: Vec<MarketFingerprint> = Vec::new();
        let normalized = crate::normalize::normalize_all(&fps);
        let matcher = SimilarityMatcher::new(CosineDistance);

        let result = matcher.search(0, &fps, &normalized, 5);
        assert_eq!(result.searched_days, 0);
        assert_eq!(result.matches.len(), 0);
    }

    #[test]
    fn match_level_thresholds() {
        let levels = vec![
            (0.05, MatchLevel::VeryHigh),
            (0.1, MatchLevel::VeryHigh),
            (0.15, MatchLevel::High),
            (0.2, MatchLevel::High),
            (0.25, MatchLevel::Moderate),
            (0.35, MatchLevel::Moderate),
            (0.5, MatchLevel::Weak),
        ];
        for (dist, expected) in levels {
            let level = match dist {
                d if d <= 0.1 => MatchLevel::VeryHigh,
                d if d <= 0.2 => MatchLevel::High,
                d if d <= 0.35 => MatchLevel::Moderate,
                _ => MatchLevel::Weak,
            };
            assert_eq!(level, expected, "dist={} expected {:?} got {:?}", dist, expected, level);
        }
    }
}

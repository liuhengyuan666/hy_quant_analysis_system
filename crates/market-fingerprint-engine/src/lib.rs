//! Market Fingerprint Engine
//!
//! V7.2A: Market Fingerprint Foundation
//! V7.2B: Evidence Retrieval Engine — normalization, distance metrics,
//!        similarity matching, and forward-outcome profiling.
//!
//! This crate provides the canonical historical feature representation for the
//! Market Evolution Semantic Layer. It converts a `ResearchContext` (or future
//! other sources) into a stable `MarketFingerprint` that can be consumed by
//! similarity algorithms, clustering, pattern search, replay, and backtest
//! engines in V7.2B and beyond.
//!
//! Principle (ADR-071): `MarketFingerprint` is the canonical contract.
//! Similarity algorithms are consumers of this contract, not part of it.
//!
//! Principle (ADR-072): The Evidence Retrieval Engine is a retrieval engine,
//! not a prediction engine. It normalizes fingerprints, matches similar
//! historical conditions, and profiles forward outcomes — purely as evidence.

pub mod builder;
pub mod distance;
pub mod fingerprint;
pub mod matcher;
pub mod normalize;
pub mod outcome;

pub use builder::MarketFingerprintBuilder;
pub use distance::{CosineDistance, DistanceMetric};
pub use fingerprint::{EvolutionVector, MarketFingerprint, ObservationVector};
pub use matcher::{HistoricalMatch, MatchLevel, SearchResult, SimilarityMatcher};
pub use normalize::{normalize_all, FeatureVector};
pub use outcome::{ForwardReturnProvider, OutcomeProfile, OutcomeProfiler};

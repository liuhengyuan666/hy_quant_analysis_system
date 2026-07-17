//! V2 Execution Platform DTOs.
//!
//! This module defines the input/output contracts for the context-driven
//! Execution Platform described in ADR-082. It lives alongside the frozen V5
//! `types.rs`/`engine.rs` implementation without modifying it.

pub mod assessment;
pub mod decision;
pub mod evidence;
pub mod event;
pub mod feature;
pub mod observation;
pub mod pipeline;
pub mod request;

pub use assessment::{EqualWeightAssessmentEngine, ExecutionAssessment, RiskLevel};
pub use decision::{DecisionEngine, DecisionReason, DefaultDecisionEngine, ExecutionDecision};
pub use evidence::{
    DefaultEvidenceBuilder, Evidence, EvidenceBuilder, EvidenceKind, EvidencePayload, EvidenceSource,
};
pub use event::ExecutionEvent;
pub use feature::{
    DefaultFeatureExtractor, FeatureExtractor, FeatureExtractorInputs, FeatureReplayRecord,
    IntradayFeatures,
};
pub use observation::{
    DefaultObservationEngine, IntradayObservation, ObservationCategory, ObservationEngine,
    ObservationKind, ObservationPayload, ObservationReplayRecord,
};
pub use pipeline::{DefaultExecutionPipeline, ExecutionPipeline};
pub use request::{
    AssessmentMode, ExecutionMarketView, ExecutionPolicy, ExecutionRequest, QuoteSnapshot,
};

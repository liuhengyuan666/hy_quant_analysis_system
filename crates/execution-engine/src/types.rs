use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Execution guidance state for a candidate symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExecutionState {
    /// Pattern suggests immediate execution.
    BuyNow,
    /// Analyzed, no pattern matched — default neutral stance.
    Wait,
    /// Do not chase extended price.
    NoChase,
    /// Reduce existing position.
    Reduce,
    /// Did not enter analysis (hard gate or data failure).
    Skip,
}

impl ExecutionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BuyNow => "BUY_NOW",
            Self::Wait => "WAIT",
            Self::NoChase => "NO_CHASE",
            Self::Reduce => "REDUCE",
            Self::Skip => "SKIP",
        }
    }
}

/// Observable reason tags for an execution decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReasonTag {
    GapUpOverextended,
    VolumeSpike,
    FarFromMA5,
    DistributionDay,
    VolumeSurgeDecline,
    StrongClose,
    HighVolume,
    DataUnavailable,
    StateGate,
}

impl ReasonTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GapUpOverextended => "GapUpOverextended",
            Self::VolumeSpike => "VolumeSpike",
            Self::FarFromMA5 => "FarFromMA5",
            Self::DistributionDay => "DistributionDay",
            Self::VolumeSurgeDecline => "VolumeSurgeDecline",
            Self::StrongClose => "StrongClose",
            Self::HighVolume => "HighVolume",
            Self::DataUnavailable => "DataUnavailable",
            Self::StateGate => "StateGate",
        }
    }
}

/// Internal skip reason — not exposed to external API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    NoCandidate,
    StateGate,
    DataUnavailable,
}

/// Real-time market snapshot for a single symbol at 14:45 (or any point).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntradaySnapshot {
    pub symbol: String,
    pub ts: DateTime<Utc>,
    pub today_return: f64,
    pub distance_ma5: f64,
    pub volume_ratio: f64,
    pub close_position: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub prev_close: f64,
}

/// Public execution decision for a single symbol.
/// Minimal: no SignalContext held here to avoid forming a second snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDecision {
    pub symbol: String,
    pub state: ExecutionState,
    pub reasons: Vec<ReasonTag>,
}

impl ExecutionDecision {
    pub fn skipped(symbol: impl Into<String>, reason: SkipReason) -> Self {
        let symbol = symbol.into();
        let tag = match reason {
            SkipReason::DataUnavailable => ReasonTag::DataUnavailable,
            SkipReason::StateGate => ReasonTag::StateGate,
            SkipReason::NoCandidate => ReasonTag::StateGate,
        };
        Self {
            symbol,
            state: ExecutionState::Skip,
            reasons: vec![tag],
        }
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reasoning graph from SKILL.md
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReasoningGraph {
    #[serde(default)]
    pub steps: HashMap<String, ReasoningStep>,
}

/// Single reasoning step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub states: Vec<String>,
    #[serde(default)]
    pub transitions: Vec<StateTransition>,
}

/// State transition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub condition: String,
}

use serde::{Deserialize, Serialize};

/// Trigger DSL for skill activation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Trigger {
    #[serde(default)]
    pub all: Vec<Condition>,
    #[serde(default)]
    pub any: Vec<Condition>,
    #[serde(default)]
    pub none: Vec<Condition>,
    #[serde(default)]
    pub weight: std::collections::HashMap<String, f64>,
}

/// A single condition: field operator value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: ComparisonOp,
    pub value: ConditionValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOp {
    #[serde(rename = "<")]
    LessThan,
    #[serde(rename = "<=")]
    LessThanOrEqual,
    #[serde(rename = ">")]
    GreaterThan,
    #[serde(rename = ">=")]
    GreaterThanOrEqual,
    #[serde(rename = "==")]
    Equal,
    #[serde(rename = "!=")]
    NotEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionValue {
    Float(f64),
    Integer(i64),
    Boolean(bool),
    String(String),
}

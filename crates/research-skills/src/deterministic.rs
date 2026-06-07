use super::provider::LlmCallConfig;

/// Configuration for deterministic mode
#[derive(Debug, Clone)]
pub struct DeterministicConfig {
    pub temperature: f64,
    pub seed: u64,
    pub top_p: f64,
    pub max_tokens: usize,
}

impl Default for DeterministicConfig {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            seed: 42,
            top_p: 0.1,
            max_tokens: 2048,
        }
    }
}

impl DeterministicConfig {
    pub fn to_config(&self) -> LlmCallConfig {
        LlmCallConfig {
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            seed: Some(self.seed),
        }
    }
}

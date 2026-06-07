use super::provider::LlmCallConfig;

/// Execution-time inference parameters for LLM calls.
///
/// These values should be sourced from `ResolvedLlmConfig` (TOML + env + CLI)
/// rather than hardcoded defaults.
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    pub temperature: f64,
    pub seed: Option<u64>,
    pub max_tokens: usize,
}

impl InferenceConfig {
    pub fn to_call_config(&self) -> LlmCallConfig {
        LlmCallConfig {
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            seed: self.seed,
        }
    }
}

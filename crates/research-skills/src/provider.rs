use async_trait::async_trait;

/// Configuration for LLM calls
#[derive(Debug, Clone)]
pub struct LlmCallConfig {
    pub temperature: f64,
    pub max_tokens: usize,
    pub seed: Option<u64>,
}

/// Trait for LLM providers (OpenAI, DeepSeek, etc.)
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a chat completion request
    async fn chat(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        config: &LlmCallConfig,
    ) -> anyhow::Result<String>;

    /// Count tokens in text (approximate)
    fn token_count(&self, text: &str) -> usize {
        // Simple approximation: 4 chars ≈ 1 token
        text.len() / 4
    }
}
